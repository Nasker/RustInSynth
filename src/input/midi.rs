use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput as MidirInput, MidiInputConnection, MidiInputPort};

use crate::core::event::NoteEvent;
use crate::core::types::MidiNote;

/// MIDI channel (0-15, displayed as 1-16 to user)
pub type MidiChannel = u8;

/// Error type for MIDI operations
#[derive(Debug)]
pub enum MidiError {
    NoPortsAvailable,
    InvalidPortIndex,
    ConnectionFailed(String),
    InitFailed(String),
}

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MidiError::NoPortsAvailable => write!(f, "No MIDI input ports available"),
            MidiError::InvalidPortIndex => write!(f, "Invalid port index"),
            MidiError::ConnectionFailed(e) => write!(f, "MIDI connection failed: {}", e),
            MidiError::InitFailed(e) => write!(f, "MIDI initialization failed: {}", e),
        }
    }
}

impl std::error::Error for MidiError {}

/// Handles MIDI input from USB devices
pub struct MidiInputHandler {
    _connection: MidiInputConnection<()>,
    receiver: Receiver<NoteEvent>,
    channel_filter: Option<MidiChannel>,
    debug_mode: bool,
}

impl MidiInputHandler {
    /// List all available MIDI input ports
    pub fn list_ports() -> Result<Vec<String>, MidiError> {
        let midi_in = MidirInput::new("RustSynth Port Lister")
            .map_err(|e| MidiError::InitFailed(e.to_string()))?;

        let ports = midi_in.ports();
        if ports.is_empty() {
            return Err(MidiError::NoPortsAvailable);
        }

        let port_names: Vec<String> = ports
            .iter()
            .map(|p| midi_in.port_name(p).unwrap_or_else(|_| "Unknown".to_string()))
            .collect();

        Ok(port_names)
    }

    /// Prompt user to select a MIDI port
    pub fn prompt_port_selection() -> Result<usize, MidiError> {
        let ports = Self::list_ports()?;

        println!("\nAvailable MIDI input ports:");
        for (i, name) in ports.iter().enumerate() {
            println!("  [{}] {}", i + 1, name);
        }

        print!("\nSelect port (1-{}): ", ports.len());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let selection: usize = input
            .trim()
            .parse()
            .map_err(|_| MidiError::InvalidPortIndex)?;

        if selection < 1 || selection > ports.len() {
            return Err(MidiError::InvalidPortIndex);
        }

        Ok(selection - 1)
    }

    /// Prompt user to select a MIDI channel to filter
    pub fn prompt_channel_selection() -> Option<MidiChannel> {
        println!("\nMIDI channel filter:");
        println!("  [0]  All channels");
        println!("  [1-16] Specific channel");

        print!("\nSelect channel (0-16): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let selection: u8 = input.trim().parse().unwrap_or(0);

        if selection == 0 {
            println!("Listening to all MIDI channels");
            None
        } else if selection <= 16 {
            println!("Filtering to MIDI channel {}", selection);
            Some(selection - 1) // Convert to 0-indexed
        } else {
            println!("Invalid selection, listening to all channels");
            None
        }
    }

    /// Connect to a MIDI input port by index
    pub fn connect(port_index: usize, channel_filter: Option<MidiChannel>) -> Result<Self, MidiError> {
        Self::connect_with_debug(port_index, channel_filter, false)
    }

    /// Connect to a MIDI input port with optional debug output
    pub fn connect_with_debug(port_index: usize, channel_filter: Option<MidiChannel>, debug_mode: bool) -> Result<Self, MidiError> {
        let midi_in = MidirInput::new("RustSynth")
            .map_err(|e| MidiError::InitFailed(e.to_string()))?;

        let ports = midi_in.ports();
        if port_index >= ports.len() {
            return Err(MidiError::InvalidPortIndex);
        }

        let port: &MidiInputPort = &ports[port_index];
        let port_name = midi_in.port_name(port).unwrap_or_else(|_| "Unknown".to_string());

        let (sender, receiver): (Sender<NoteEvent>, Receiver<NoteEvent>) = mpsc::channel();

        let filter = channel_filter;
        let connection = midi_in
            .connect(
                port,
                "RustSynth Input",
                move |timestamp, message, _| {
                    if debug_mode {
                        print_midi_debug(timestamp, message);
                    }
                    if let Some(event) = parse_midi_message(message, filter) {
                        let _ = sender.send(event);
                    }
                },
                (),
            )
            .map_err(|e| MidiError::ConnectionFailed(e.to_string()))?;

        println!("Connected to MIDI port: {}", port_name);
        if debug_mode {
            println!("MIDI debug mode enabled - raw messages will be printed");
        }

        Ok(Self {
            _connection: connection,
            receiver,
            channel_filter,
            debug_mode,
        })
    }

    /// Prompt user to enable debug mode
    pub fn prompt_debug_mode() -> bool {
        print!("\nEnable MIDI debug output? (y/N): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }

    /// Connect with interactive port and channel selection
    pub fn connect_interactive() -> Result<Self, MidiError> {
        let port_index = Self::prompt_port_selection()?;
        let channel_filter = Self::prompt_channel_selection();
        let debug_mode = Self::prompt_debug_mode();
        Self::connect_with_debug(port_index, channel_filter, debug_mode)
    }

    /// Poll for incoming MIDI events (non-blocking)
    pub fn poll(&self) -> Option<NoteEvent> {
        self.receiver.try_recv().ok()
    }

    /// Get the channel filter setting
    pub fn channel_filter(&self) -> Option<MidiChannel> {
        self.channel_filter
    }
}

/// Parse a raw MIDI message into a NoteEvent
fn parse_midi_message(message: &[u8], channel_filter: Option<MidiChannel>) -> Option<NoteEvent> {
    if message.is_empty() {
        return None;
    }

    let status = message[0];
    let message_type = status & 0xF0;
    let channel = status & 0x0F;

    // Apply channel filter
    if let Some(filter_channel) = channel_filter {
        if channel != filter_channel {
            return None;
        }
    }

    match message_type {
        0x90 => {
            // Note On
            if message.len() >= 3 {
                let note: MidiNote = message[1];
                let velocity = message[2];
                if velocity > 0 {
                    Some(NoteEvent::note_on_midi(note, velocity))
                } else {
                    // Note On with velocity 0 is equivalent to Note Off
                    Some(NoteEvent::note_off(note))
                }
            } else {
                None
            }
        }
        0x80 => {
            // Note Off
            if message.len() >= 2 {
                let note: MidiNote = message[1];
                Some(NoteEvent::note_off(note))
            } else {
                None
            }
        }
        0xB0 => {
            // Control Change
            if message.len() >= 3 {
                let cc = message[1];
                let value = message[2];
                Some(NoteEvent::control_change(cc, value))
            } else {
                None
            }
        }
        0xE0 => {
            // Pitch Bend
            if message.len() >= 3 {
                let lsb = message[1] as u16;
                let msb = message[2] as u16;
                let value = (msb << 7) | lsb; // 14-bit value (0-16383)
                Some(NoteEvent::pitch_bend_midi(value))
            } else {
                None
            }
        }
        _ => None, // Ignore other message types for now
    }
}

/// Print debug information for a MIDI message
fn print_midi_debug(timestamp: u64, message: &[u8]) {
    if message.is_empty() {
        return;
    }

    let status = message[0];
    let message_type = status & 0xF0;
    let channel = (status & 0x0F) + 1; // Display as 1-16

    let description = match message_type {
        0x80 => {
            if message.len() >= 3 {
                format!("Note Off      | note={:3} vel={:3} | {}", 
                    message[1], message[2], note_name(message[1]))
            } else {
                "Note Off (incomplete)".to_string()
            }
        }
        0x90 => {
            if message.len() >= 3 {
                let vel = message[2];
                let kind = if vel == 0 { "Note Off (v=0)" } else { "Note On       " };
                format!("{} | note={:3} vel={:3} | {}", 
                    kind, message[1], vel, note_name(message[1]))
            } else {
                "Note On (incomplete)".to_string()
            }
        }
        0xA0 => {
            if message.len() >= 3 {
                format!("Aftertouch    | note={:3} pressure={:3}", message[1], message[2])
            } else {
                "Aftertouch (incomplete)".to_string()
            }
        }
        0xB0 => {
            if message.len() >= 3 {
                format!("Control Change| CC#{:3}={:3} | {}", 
                    message[1], message[2], cc_name(message[1]))
            } else {
                "Control Change (incomplete)".to_string()
            }
        }
        0xC0 => {
            if message.len() >= 2 {
                format!("Program Change| program={:3}", message[1])
            } else {
                "Program Change (incomplete)".to_string()
            }
        }
        0xD0 => {
            if message.len() >= 2 {
                format!("Ch Pressure   | pressure={:3}", message[1])
            } else {
                "Channel Pressure (incomplete)".to_string()
            }
        }
        0xE0 => {
            if message.len() >= 3 {
                let bend = ((message[2] as u16) << 7) | (message[1] as u16);
                let bend_centered = bend as i16 - 8192;
                format!("Pitch Bend    | value={:6} (raw: {:5})", bend_centered, bend)
            } else {
                "Pitch Bend (incomplete)".to_string()
            }
        }
        0xF0 => {
            match status {
                0xF0 => format!("SysEx Start   | {} bytes", message.len()),
                0xF7 => "SysEx End".to_string(),
                0xF8 => "Timing Clock".to_string(),
                0xFA => "Start".to_string(),
                0xFB => "Continue".to_string(),
                0xFC => "Stop".to_string(),
                0xFE => "Active Sensing".to_string(),
                0xFF => "System Reset".to_string(),
                _ => format!("System: 0x{:02X}", status),
            }
        }
        _ => format!("Unknown: 0x{:02X}", status),
    };

    // Format: timestamp, channel, description, raw hex
    let raw_hex: String = message.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ");
    
    println!("\r[{:10}] Ch{:2} | {} | [{}]", 
        timestamp / 1000, // Convert to ms
        channel,
        description,
        raw_hex
    );
}

/// Convert MIDI note number to note name
fn note_name(note: u8) -> String {
    let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (note / 12) as i8 - 1;
    let name = names[(note % 12) as usize];
    format!("{}{}", name, octave)
}

/// Get common CC name
fn cc_name(cc: u8) -> &'static str {
    match cc {
        0 => "Bank Select MSB",
        1 => "Mod Wheel",
        2 => "Breath",
        7 => "Volume",
        10 => "Pan",
        11 => "Expression",
        32 => "Bank Select LSB",
        64 => "Sustain Pedal",
        65 => "Portamento",
        66 => "Sostenuto",
        67 => "Soft Pedal",
        70 => "Sound Ctrl 1 (Waveform)",
        71 => "Sound Ctrl 2 (Resonance)",
        72 => "Sound Ctrl 3 (Release)",
        73 => "Sound Ctrl 4 (Attack)",
        74 => "Sound Ctrl 5 (Cutoff)",
        91 => "Reverb",
        93 => "Chorus",
        120 => "All Sound Off",
        121 => "Reset All Ctrl",
        123 => "All Notes Off",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let message = [0x90, 60, 100]; // Note On, C4, velocity 100
        let event = parse_midi_message(&message, None).unwrap();
        assert_eq!(event.note, 60);
        assert!(event.velocity > 0.0);
    }

    #[test]
    fn test_parse_note_off() {
        let message = [0x80, 60, 0]; // Note Off, C4
        let event = parse_midi_message(&message, None).unwrap();
        assert_eq!(event.note, 60);
        assert_eq!(event.velocity, 0.0);
    }

    #[test]
    fn test_channel_filter() {
        let message = [0x91, 60, 100]; // Note On, channel 1
        
        // Should pass with no filter
        assert!(parse_midi_message(&message, None).is_some());
        
        // Should pass with matching channel
        assert!(parse_midi_message(&message, Some(1)).is_some());
        
        // Should fail with different channel
        assert!(parse_midi_message(&message, Some(0)).is_none());
    }
}
