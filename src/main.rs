//! RustSynth GUI Application
//! 
//! Graphical interface using egui with Minimoog-style aesthetic.

use RustInSynth::gui::{run_gui, SharedState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔊 Rust In Synth v0.4.0 - GUI Mode");
    println!("Starting...\n");

    // Create shared state for GUI ↔ Audio communication
    let shared = SharedState::new();

    // Run GUI - audio engine is managed within the GUI app
    if let Err(e) = run_gui(shared) {
        eprintln!("GUI error: {}", e);
    }

    println!("\nGoodbye!");
    Ok(())
}
