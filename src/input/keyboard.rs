use std::collections::HashMap;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use crossterm::execute;
use std::io::stdout;

use crate::core::event::{NoteEvent, WaveformType};
use crate::core::types::MidiNote;

/// Maps QWERTY keyboard keys to MIDI note numbers
/// Layout mimics a piano keyboard across two rows
pub struct KeyboardMapping {
    key_to_note: HashMap<KeyCode, MidiNote>,
    base_octave: i8,
}

impl KeyboardMapping {
    pub fn new() -> Self {
        Self {
            key_to_note: Self::create_default_mapping(),
            base_octave: 4, // Middle C octave
        }
    }

    /// Create the default QWERTY to note mapping
    /// Lower row (ZXCV...): C, C#, D, D#, E, F, F#, G, G#, A, A#, B
    /// Upper row (QWER...): C, C#, D, D#, E, F, F#, G, G#, A, A#, B (one octave up)
    fn create_default_mapping() -> HashMap<KeyCode, MidiNote> {
        let mut map = HashMap::new();

        // Lower row - octave 4 (starting at C4 = MIDI 60)
        let lower_row = [
            (KeyCode::Char('z'), 60), // C4
            (KeyCode::Char('s'), 61), // C#4
            (KeyCode::Char('x'), 62), // D4
            (KeyCode::Char('d'), 63), // D#4
            (KeyCode::Char('c'), 64), // E4
            (KeyCode::Char('v'), 65), // F4
            (KeyCode::Char('g'), 66), // F#4
            (KeyCode::Char('b'), 67), // G4
            (KeyCode::Char('h'), 68), // G#4
            (KeyCode::Char('n'), 69), // A4 (440 Hz)
            (KeyCode::Char('j'), 70), // A#4
            (KeyCode::Char('m'), 71), // B4
            (KeyCode::Char(','), 72), // C5
        ];

        // Upper row - octave 5 (starting at C5 = MIDI 72)
        let upper_row = [
            (KeyCode::Char('q'), 72), // C5
            (KeyCode::Char('2'), 73), // C#5
            (KeyCode::Char('w'), 74), // D5
            (KeyCode::Char('3'), 75), // D#5
            (KeyCode::Char('e'), 76), // E5
            (KeyCode::Char('r'), 77), // F5
            (KeyCode::Char('5'), 78), // F#5
            (KeyCode::Char('t'), 79), // G5
            (KeyCode::Char('6'), 80), // G#5
            (KeyCode::Char('y'), 81), // A5
            (KeyCode::Char('7'), 82), // A#5
            (KeyCode::Char('u'), 83), // B5
            (KeyCode::Char('i'), 84), // C6
        ];

        for (key, note) in lower_row.iter().chain(upper_row.iter()) {
            map.insert(*key, *note);
        }

        map
    }

    /// Get the MIDI note for a key, if mapped
    pub fn get_note(&self, key: KeyCode) -> Option<MidiNote> {
        self.key_to_note.get(&key).copied()
    }

    /// Shift the octave up
    pub fn octave_up(&mut self) {
        if self.base_octave < 8 {
            self.base_octave += 1;
            self.rebuild_mapping();
        }
    }

    /// Shift the octave down
    pub fn octave_down(&mut self) {
        if self.base_octave > 0 {
            self.base_octave -= 1;
            self.rebuild_mapping();
        }
    }

    fn rebuild_mapping(&mut self) {
        let offset = (self.base_octave - 4) * 12;
        self.key_to_note = Self::create_default_mapping()
            .into_iter()
            .map(|(k, n)| (k, (n as i16 + offset as i16).clamp(0, 127) as MidiNote))
            .collect();
    }
}

impl Default for KeyboardMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles keyboard input and converts to note events
pub struct KeyboardInput {
    mapping: KeyboardMapping,
    current_note: Option<MidiNote>,
    enhanced_keyboard: bool,
}

impl KeyboardInput {
    pub fn new() -> Self {
        // Try to enable enhanced keyboard mode for key release detection
        let enhanced_keyboard = execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        ).is_ok();

        if !enhanced_keyboard {
            eprintln!("Note: Key release detection not available. Using monophonic mode.");
        }

        Self {
            mapping: KeyboardMapping::new(),
            current_note: None,
            enhanced_keyboard,
        }
    }

    /// Poll for keyboard events (non-blocking)
    /// Returns Some(NoteEvent) if a note key was pressed/released
    /// Returns None if no relevant event or timeout
    pub fn poll(&mut self) -> Result<Option<KeyboardEvent>, std::io::Error> {
        if event::poll(std::time::Duration::from_millis(1))? {
            if let Event::Key(key_event) = event::read()? {
                // Handle control keys
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        KeyCode::Up => {
                            self.mapping.octave_up();
                            return Ok(Some(KeyboardEvent::OctaveUp));
                        }
                        KeyCode::Down => {
                            self.mapping.octave_down();
                            return Ok(Some(KeyboardEvent::OctaveDown));
                        }
                        KeyCode::Esc => {
                            return Ok(Some(KeyboardEvent::Quit));
                        }
                        // Waveform selection: F1-F4
                        KeyCode::F(1) => {
                            return Ok(Some(KeyboardEvent::WaveformChange(WaveformType::Sine)));
                        }
                        KeyCode::F(2) => {
                            return Ok(Some(KeyboardEvent::WaveformChange(WaveformType::Square)));
                        }
                        KeyCode::F(3) => {
                            return Ok(Some(KeyboardEvent::WaveformChange(WaveformType::Saw)));
                        }
                        KeyCode::F(4) => {
                            return Ok(Some(KeyboardEvent::WaveformChange(WaveformType::Triangle)));
                        }
                        _ => {}
                    }
                }

                // Handle note keys
                if let Some(note) = self.mapping.get_note(key_event.code) {
                    match key_event.kind {
                        KeyEventKind::Press => {
                            // In monophonic mode, release previous note before playing new one
                            let mut events = Vec::new();
                            if let Some(prev_note) = self.current_note {
                                if prev_note != note {
                                    events.push(NoteEvent::note_off(prev_note));
                                }
                            }
                            self.current_note = Some(note);
                            events.push(NoteEvent::note_on(note, 0.8));
                            return Ok(Some(KeyboardEvent::Notes(events)));
                        }
                        KeyEventKind::Release => {
                            // Only release if this is the current note
                            if self.current_note == Some(note) {
                                self.current_note = None;
                                return Ok(Some(KeyboardEvent::Note(NoteEvent::note_off(note))));
                            }
                        }
                        KeyEventKind::Repeat => {}
                    };
                }
            }
        }
        Ok(None)
    }
}

impl Default for KeyboardInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Events that can come from keyboard input
#[derive(Debug)]
pub enum KeyboardEvent {
    Note(NoteEvent),
    Notes(Vec<NoteEvent>),
    OctaveUp,
    OctaveDown,
    WaveformChange(WaveformType),
    Quit,
}
