//! UI Theme and Color Scheme for RustInSynth
//!
//! Inspired by Megadeth's "Rust In Peace" album artwork:
//! - Dark metallic grays (rust, steel)
//! - Blood red accents
//! - Warning yellows/oranges
//! - Electric blues for indicators

use egui::{Color32, Stroke};

/// Rust In Peace inspired color palette
pub struct Theme {
    // Background blue from Rust In Peace artwork
    pub bg_blue: Color32,          // Deep blue background
    pub bg_blue_light: Color32,    // Lighter blue for panels
    
    // Megadeth logo text color (reddish-orange)
    pub logo_red: Color32,         // Logo text color
    pub logo_red_dark: Color32,    // Darker logo text
    pub logo_red_light: Color32,   // Lighter logo accent

    // Chrome silver of the Megadeth logo lettering
    pub chrome: Color32,           // Silver body, cooled by the hangar's blue light
    pub chrome_light: Color32,     // Top edge highlight
    pub chrome_shadow: Color32,    // Lower edge shadow

    // Rust from the cryogenic pod's corroded panels (used sparingly)
    pub rust: Color32,             // Oxidized orange-brown
    
    // Goldish color from artwork
    pub gold: Color32,             // Gold accents
    pub gold_dark: Color32,        // Darker gold
    pub gold_light: Color32,       // Lighter gold highlight
    
    // Radioactive green for knobs/sliders
    pub toxic_green: Color32,      // Radioactive green
    pub toxic_green_dark: Color32, // Darker toxic
    pub toxic_green_light: Color32,// Lighter toxic
    pub toxic_green_deep: Color32, // Deep interior behind the glow
    
    // Supporting grays (subtle, not dominant)
    pub steel_dark: Color32,       // Dark steel for borders
    pub steel_medium: Color32,     // Medium steel
    pub steel_light: Color32,      // Light steel highlights
    
    // Text colors - using logo red hierarchy
    pub text_primary: Color32,     // Main text (logo red)
    pub text_secondary: Color32,   // Secondary text
    pub text_dim: Color32,         // Dimmed text
    
    // Panel colors - using blue from artwork
    pub panel_bg: Color32,         // Panel background
    pub panel_border: Color32,     // Panel borders
    pub panel_shadow: Color32,     // Panel shadows
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Background blue from Rust In Peace artwork
            bg_blue: Color32::from_rgb(15, 25, 45),          // Deep blue background
            bg_blue_light: Color32::from_rgb(25, 35, 55),    // Lighter blue for panels
            
            // Megadeth logo text color (reddish-orange)
            logo_red: Color32::from_rgb(220, 60, 20),        // Logo text color
            logo_red_dark: Color32::from_rgb(160, 40, 10),   // Darker logo text
            logo_red_light: Color32::from_rgb(255, 100, 50), // Lighter logo accent

            // Chrome silver of the Megadeth logo lettering
            chrome: Color32::from_rgb(198, 208, 222),        // Silver body
            chrome_light: Color32::from_rgb(236, 242, 250),  // Top edge highlight
            chrome_shadow: Color32::from_rgb(24, 32, 48),    // Lower edge shadow

            // Rust from the cryogenic pod's corroded panels
            rust: Color32::from_rgb(150, 66, 24),            // Oxidized orange-brown
            
            // Goldish color from artwork
            gold: Color32::from_rgb(255, 180, 40),            // Gold accents
            gold_dark: Color32::from_rgb(200, 140, 20),      // Darker gold
            gold_light: Color32::from_rgb(255, 200, 80),     // Lighter gold highlight
            
            // Radioactive green for knobs/sliders
            toxic_green: Color32::from_rgb(40, 255, 80),     // Radioactive green
            toxic_green_dark: Color32::from_rgb(20, 180, 50), // Darker toxic
            toxic_green_light: Color32::from_rgb(100, 255, 120), // Lighter toxic
            toxic_green_deep: Color32::from_rgb(6, 46, 16),  // Deep interior behind the glow
            
            // Supporting grays (subtle, not dominant)
            steel_dark: Color32::from_rgb(35, 35, 40),       // Dark steel for borders
            steel_medium: Color32::from_rgb(50, 50, 55),     // Medium steel
            steel_light: Color32::from_rgb(70, 70, 75),      // Light steel highlights
            
            // Text colors - using logo red hierarchy
            text_primary: Color32::from_rgb(220, 60, 20),    // Main text (logo red)
            text_secondary: Color32::from_rgb(160, 40, 10),   // Secondary text
            text_dim: Color32::from_rgb(100, 30, 5),        // Dimmed text
            
            // Panel colors - using blue from artwork
            panel_bg: Color32::from_rgb(25, 35, 55),         // Panel background
            panel_border: Color32::from_rgb(35, 45, 65),     // Panel borders
            panel_shadow: Color32::from_rgb(10, 20, 35),     // Panel shadows
        }
    }
}

impl Theme {
    /// Get stroke for knobs based on state
    pub fn knob_stroke(&self, hovered: bool, dragged: bool) -> Stroke {
        let color = if dragged {
            self.toxic_green_light
        } else if hovered {
            self.gold
        } else {
            self.steel_medium
        };
        Stroke::new(2.0, color)
    }
    
    /// Get value indicator color for knobs (radioactive green)
    pub fn knob_value_color(&self) -> Color32 {
        self.toxic_green
    }
    
    /// Get panel background with subtle gradient effect
    pub fn panel_gradient_top(&self) -> Color32 {
        self.panel_bg
    }
    
    pub fn panel_gradient_bottom(&self) -> Color32 {
        Color32::from_rgb(
            self.panel_bg.r() - 10,
            self.panel_bg.g() - 10,
            self.panel_bg.b() - 10,
        )
    }
    
    /// Get section header color (logo red)
    pub fn section_header(&self) -> Color32 {
        self.logo_red
    }
    
    /// Get MIDI activity color (gold when active)
    pub fn midi_activity(&self, active: bool) -> Color32 {
        if active {
            self.gold
        } else {
            self.steel_dark
        }
    }
    
    /// Get button colors based on state
    pub fn button_colors(&self, hovered: bool, active: bool) -> (Color32, Color32) {
        if active {
            (self.logo_red, self.text_primary)
        } else if hovered {
            (self.gold, self.text_primary)
        } else {
            (self.steel_dark, self.text_secondary)
        }
    }
    
    /// Get toggle switch colors (toxic green when on)
    pub fn toggle_switch(&self, on: bool) -> Color32 {
        if on {
            self.toxic_green
        } else {
            self.steel_dark
        }
    }
    
    /// Get VU meter colors (toxic green to gold to logo red)
    pub fn vu_meter_color(&self, level: f32) -> Color32 {
        if level < 0.6 {
            // Toxic green for low levels
            Color32::from_rgb(
                (20.0 + level * 80.0) as u8,
                (180.0 + level * 75.0) as u8,
                (50.0 + level * 70.0) as u8,
            )
        } else if level < 0.8 {
            // Gold for medium levels
            Color32::from_rgb(
                255,
                (140.0 + (level - 0.6) * 300.0) as u8,
                (20.0 + (level - 0.6) * 150.0) as u8,
            )
        } else {
            // Logo red for high levels
            Color32::from_rgb(
                255,
                (40.0 + (1.0 - level) * 40.0) as u8,
                (10.0 + (1.0 - level) * 40.0) as u8,
            )
        }
    }
}

/// Global theme instance
pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::default);

/// Helper functions for themed UI elements
pub fn panel_background(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let theme = &THEME;
    
    egui::Frame::new()
        .fill(theme.panel_bg)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .outer_margin(egui::Margin::ZERO)
        .corner_radius(egui::CornerRadius::same(4))
        .shadow(egui::epaint::Shadow {
            offset: [2, 2],
            blur: 4,
            spread: 0,
            color: theme.panel_shadow,
        })
        .stroke(Stroke::new(1.0, theme.panel_border))
        .show(ui, add_contents);
}

pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .color(THEME.section_header())
            .size(12.0)
            .strong()
    );
}

/// Paint the synth name like the Megadeth logo on the cover: widely tracked
/// chrome lettering with a bright top edge, a dark lower edge, and a thin
/// gold rule underneath echoing the "RUST IN PEACE" band.
pub fn chrome_title(ui: &mut egui::Ui, text: &str, size: f32) -> egui::Response {
    let galley_size = {
        let mut job = egui::text::LayoutJob::default();
        job.append(text, 0.0, chrome_format(size, THEME.chrome));
        ui.ctx().fonts_mut(|f| f.layout_job(job)).size()
    };
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(galley_size.x, galley_size.y + 4.0),
        egui::Sense::hover(),
    );
    let ctx = ui.ctx().clone();
    paint_chrome_title(&ctx, ui.painter_at(rect), rect.center(), text, size);
    response
}

fn chrome_format(size: f32, color: Color32) -> egui::text::TextFormat {
    egui::text::TextFormat {
        font_id: egui::FontId::new(size, egui::FontFamily::Proportional),
        color,
        extra_letter_spacing: size * 0.16,
        ..Default::default()
    }
}

/// Paint the chrome logo centered on `center` (used for the top banner).
pub fn paint_chrome_title(
    ctx: &egui::Context,
    painter: egui::Painter,
    center: egui::Pos2,
    text: &str,
    size: f32,
) {
    let galley_for = |color: Color32| {
        let mut job = egui::text::LayoutJob::default();
        job.append(text, 0.0, chrome_format(size, color));
        ctx.fonts_mut(|f| f.layout_job(job))
    };

    let main = galley_for(THEME.chrome);
    let shadow = galley_for(THEME.chrome_shadow);
    let light = galley_for(THEME.chrome_light);

    let dims = main.size();
    let pos = center - dims * 0.5;

    // Metal depth: dark base below, bright edge above, silver body on top
    painter.galley(pos + egui::vec2(0.0, 2.0), shadow, Color32::BLACK);
    painter.galley(pos + egui::vec2(0.0, -1.0), light, Color32::WHITE);
    painter.galley(pos, main, Color32::WHITE);

    // Gold rule under the logo
    let y = pos.y + dims.y + 2.0;
    painter.hline(egui::Rangef::new(pos.x, pos.x + dims.x), y, Stroke::new(1.0, THEME.gold_dark));
}

/// A label with wide letter tracking, like the spaced capitals under the
/// logo on the album cover.
pub fn spaced_label(ui: &mut egui::Ui, text: &str, size: f32, color: Color32) -> egui::Response {
    use egui::text::{LayoutJob, TextFormat};

    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: egui::FontId::new(size, egui::FontFamily::Proportional),
            color,
            extra_letter_spacing: size * 0.35,
            ..Default::default()
        },
    );
    ui.label(job)
}

/// Draw the hangar backdrop: blue-steel girders forming tall wall-panel
/// boxes, rivets at the joints, and a whisper of the cryo-pod's rust
/// bleeding up from the floor line.
pub fn hangar_background(painter: &egui::Painter, rect: egui::Rect) {
    let rib = Color32::from_rgba_unmultiplied(70, 95, 140, 22);
    let beam = Color32::from_rgba_unmultiplied(90, 120, 170, 30);
    let rivet = Color32::from_rgba_unmultiplied(120, 150, 200, 36);

    // Vertical ribs — heavier girder every fourth bay
    let bays = 16;
    let bay = rect.width() / bays as f32;
    for i in 0..=bays {
        let x = rect.left() + bay * i as f32;
        let stroke = if i % 4 == 0 {
            Stroke::new(1.5, beam)
        } else {
            Stroke::new(1.0, rib)
        };
        painter.vline(x, rect.y_range(), stroke);
    }

    // Horizontal beams → tall panel boxes
    let rows = 4;
    for r in 1..rows {
        let y = rect.top() + rect.height() * r as f32 / rows as f32;
        painter.hline(rect.x_range(), y, Stroke::new(1.0, rib));
    }

    // Rivets where the girders cross the beams
    for i in (0..=bays).step_by(4) {
        let x = rect.left() + bay * i as f32;
        for r in 1..rows {
            let y = rect.top() + rect.height() * r as f32 / rows as f32;
            painter.circle_filled(egui::pos2(x, y), 1.6, rivet);
        }
    }

    // Subtle rust rising from the floor line
    let band_h = (rect.height() * 0.28).min(160.0);
    let band = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - band_h),
        rect.right_bottom(),
    );
    let fade_in = Color32::from_rgba_unmultiplied(THEME.rust.r(), THEME.rust.g(), THEME.rust.b(), 0);
    let rust_low = Color32::from_rgba_unmultiplied(THEME.rust.r(), THEME.rust.g(), THEME.rust.b(), 20);
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [
        (band.left_top(), fade_in),
        (band.right_top(), fade_in),
        (band.right_bottom(), rust_low),
        (band.left_bottom(), rust_low),
    ] {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(egui::Shape::mesh(mesh));
}
