use std::io::{self, stdout, Write};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use rustsynth::audio::AudioEngine;
use rustsynth::core::event::SynthEvent;
use rustsynth::input::keyboard::{KeyboardEvent, KeyboardInput};
use rustsynth::input::midi::MidiInputHandler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                    RustSynth v0.1.0                       ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  Keyboard Layout (piano style):                           ║");
    println!("║                                                           ║");
    println!("║  Upper row: Q W E R T Y U I  (C5 to C6)                   ║");
    println!("║  Black keys: 2 3   5 6 7     (sharps/flats)               ║");
    println!("║                                                           ║");
    println!("║  Lower row: Z X C V B N M ,  (C4 to C5)                   ║");
    println!("║  Black keys: S D   G H J     (sharps/flats)               ║");
    println!("║                                                           ║");
    println!("║  Controls:                                                ║");
    println!("║    ↑/↓   - Octave up/down                                 ║");
    println!("║    F1-F4 - Waveform (Sine/Square/Saw/Triangle)            ║");
    println!("║    ESC   - Quit                                           ║");
    println!("║                                                           ║");
    println!("║  MIDI CC Mappings:                                        ║");
    println!("║    CC #75 - Waveform (0-31=Sine, 32-63=Sq, 64-95=Saw, 96+)║");
    println!("║    CC #73 - Attack time  (1ms - 2s)                       ║");
    println!("║    CC #72 - Release time (1ms - 5s)                       ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();

    // Ask user for input mode
    let midi_input = prompt_input_mode();

    // Initialize audio engine
    let mut engine = AudioEngine::new()?;
    engine.set_master_volume(0.5);
    engine.start()?;

    println!("\nAudio engine started. Press keys to play!");
    println!("Press ESC to quit.\n");

    // Enable raw mode for keyboard input
    enable_raw_mode()?;

    let mut keyboard = KeyboardInput::new();

    loop {
        // Poll MIDI input if connected
        if let Some(ref midi) = midi_input {
            while let Some(note_event) = midi.poll() {
                engine.send_event(note_event);
            }
        }

        // Poll keyboard input
        match keyboard.poll() {
            Ok(Some(event)) => match event {
                KeyboardEvent::Note(note_event) => {
                    engine.send_event(note_event);
                }
                KeyboardEvent::Notes(note_events) => {
                    for note_event in note_events {
                        engine.send_event(note_event);
                    }
                }
                KeyboardEvent::OctaveUp => {
                    print!("\rOctave shifted up   ");
                    stdout().flush()?;
                }
                KeyboardEvent::OctaveDown => {
                    print!("\rOctave shifted down ");
                    stdout().flush()?;
                }
                KeyboardEvent::WaveformChange(waveform) => {
                    engine.send_event(SynthEvent::waveform_change(waveform));
                    print!("\rWaveform: {}        ", waveform.name());
                    stdout().flush()?;
                }
                KeyboardEvent::Quit => {
                    break;
                }
            },
            Ok(None) => {}
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    engine.stop();

    println!("\nGoodbye!");
    Ok(())
}

/// Prompt user to select input mode (keyboard only or MIDI)
fn prompt_input_mode() -> Option<MidiInputHandler> {
    println!("Select input mode:");
    println!("  [1] Computer keyboard only");
    println!("  [2] MIDI controller (+ keyboard fallback)");
    print!("\nChoice (1-2): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    match input.trim() {
        "2" => {
            match MidiInputHandler::connect_interactive() {
                Ok(midi) => Some(midi),
                Err(e) => {
                    eprintln!("MIDI error: {}. Falling back to keyboard only.", e);
                    None
                }
            }
        }
        _ => {
            println!("Using computer keyboard input.");
            None
        }
    }
}
