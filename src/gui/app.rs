//! Main egui application — backend-agnostic version.
//!
//! All audio/MIDI access goes through `Box<dyn SynthBackend>`.

use crate::core::presets::install_factory_presets;
use crate::gui::backend::SynthBackend;
use crate::gui::panels::{self, PresetState};

/// Main synth application — owns the backend abstraction
pub struct SynthApp {
    /// Backend abstraction (standalone or plugin)
    backend: Box<dyn SynthBackend + 'static>,

    /// UI-only preset state (shared definition with the plugin editor)
    presets: PresetState,
}

impl SynthApp {
    pub fn new(backend: Box<dyn SynthBackend + 'static>) -> Self {
        match install_factory_presets() {
            Ok(n) if n > 0 => println!("Installed {} factory presets", n),
            Err(e) => eprintln!("Failed to install factory presets: {}", e),
            _ => {}
        }

        Self {
            backend,
            presets: PresetState::new(),
        }
    }

    // ========================================================================
    // UI
    // ========================================================================

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        // Let the backend poll MIDI and sync params every frame
        self.backend.update();

        // Render the shared window shell. Standalone shows the CPU meter and
        // the MIDI panel; the plugin host omits both.
        let cfg = panels::ShellConfig {
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            show_cpu: true,
            show_midi_panel: true,
        };
        panels::shell(ctx, self.backend.as_mut(), &mut self.presets, &cfg);
    }

    fn on_exit(&mut self) {
        // The backend owns the audio engine; when it drops, audio stops automatically.
        println!("Goodbye!");
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        SynthApp::update(self, ctx, frame);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        SynthApp::on_exit(self);
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

/// Run the standalone GUI application.
///
/// Creates a `StandaloneBackend`, wraps it in `SynthApp`, and runs eframe.
#[cfg(not(feature = "plugin"))]
pub fn run_gui(shared: crate::gui::SharedState) -> Result<(), eframe::Error> {
    use crate::gui::StandaloneBackend;

    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 550.0])
            .with_min_inner_size([1450.0, 520.0])
            .with_resizable(false),
        ..Default::default()
    };

    // On Linux, eframe 0.34's glutin Wayland/EGL path can fail to create a
    // surface ("provided native window is not supported") on some systems
    // (e.g. hybrid Intel+NVIDIA laptops). Force the X11/XWayland backend,
    // which works reliably.
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_x11();
        }));
    }

    eframe::run_native(
        "Rust In Synth",
        options,
        Box::new(|_cc| {
            let backend = Box::new(StandaloneBackend::new(shared));
            Ok(Box::new(SynthApp::new(backend)))
        }),
    )
}
