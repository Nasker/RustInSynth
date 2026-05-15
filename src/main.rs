use std::io::{self, stdout, Write};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use rustsynth::audio::AudioEngine;
use rustsynth::core::event::SynthEvent;
use rustsynth::input::keyboard::{KeyboardEvent, KeyboardInput};
use rustsynth::input::midi::MidiInputHandler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                    RustSynth v0.1.0                       ║");
    println!("║              3-Oscillator Minimoog-style Synth            ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  Keyboard Layout (piano style):                           ║");
    println!("║  Upper: Q W E R T Y U I  |  Lower: Z X C V B N M ,        ║");
    println!("║  Black: 2 3   5 6 7      |  Black: S D   G H J            ║");
    println!("║  Controls: ↑/↓ Octave | F1-F4 Waveform | ESC Quit         ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  MIDI CC Mappings:                                        ║");
    println!("║  ┌─────────────────────────────────────────────────────┐  ║");
    println!("║  │ ENVELOPE        │ FILTER          │ OSCILLATORS     │  ║");
    println!("║  │ 73: Attack      │ 74: Cutoff      │ 75-77: Waveform │  ║");
    println!("║  │ 72: Release     │ 71: Resonance   │ 80-82: Level    │  ║");
    println!("║  │                 │                 │ 78-79: Semitone │  ║");
    println!("║  │                 │                 │ 85-86: Cents    │  ║");
    println!("║  └─────────────────────────────────────────────────────┘  ║");
    println!("║  Default: OSC1=Saw, OSC2=Saw+7¢, OSC3=Square-12st (sub)   ║");
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
