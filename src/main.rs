//! RustSynth GUI Application
//! 
//! Graphical interface using egui with Minimoog-style aesthetic.
//!
//! This standalone binary is only built when the `plugin` feature is OFF.
//! Under `--features plugin`, the crate is intended to be loaded as a plugin
//! (cdylib) by a host, so the standalone entry point is compiled out.

#[cfg(not(feature = "plugin"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use rust_in_synth::gui::{run_gui, SharedState};

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

#[cfg(feature = "plugin")]
fn main() {
    // The standalone GUI is disabled in plugin builds; load the crate as a
    // plugin (cdylib) from your DAW instead.
}
