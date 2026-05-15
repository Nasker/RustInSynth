//! UI Theme and Color Scheme for RustInSynth
//!
//! Inspired by Megadeth's "Rust In Peace" album artwork:
//! - Dark metallic grays (rust, steel)
//! - Blood red accents
//! - Warning yellows/oranges
//! - Electric blues for indicators

use egui::{Color32, Stroke, Rounding};

/// Rust In Peace inspired color palette
pub struct Theme {
    // Background blue from Rust In Peace artwork
    pub bg_blue: Color32,          // Deep blue background
    pub bg_blue_light: Color32,    // Lighter blue for panels
    
    // Megadeth logo text color (reddish-orange)
    pub logo_red: Color32,         // Logo text color
    pub logo_red_dark: Color32,    // Darker logo text
    pub logo_red_light: Color32,   // Lighter logo accent
    
    // Goldish color from artwork
    pub gold: Color32,             // Gold accents
    pub gold_dark: Color32,        // Darker gold
    pub gold_light: Color32,       // Lighter gold highlight
    
    // Radioactive green for knobs/sliders
    pub toxic_green: Color32,      // Radioactive green
    pub toxic_green_dark: Color32, // Darker toxic
    pub toxic_green_light: Color32,// Lighter toxic
    
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
            
            // Goldish color from artwork
            gold: Color32::from_rgb(255, 180, 40),            // Gold accents
            gold_dark: Color32::from_rgb(200, 140, 20),      // Darker gold
            gold_light: Color32::from_rgb(255, 200, 80),     // Lighter gold highlight
            
            // Radioactive green for knobs/sliders
            toxic_green: Color32::from_rgb(40, 255, 80),     // Radioactive green
            toxic_green_dark: Color32::from_rgb(20, 180, 50), // Darker toxic
            toxic_green_light: Color32::from_rgb(100, 255, 120), // Lighter toxic
            
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
    
    egui::Frame {
        fill: theme.panel_bg,
        inner_margin: egui::Margin::symmetric(12.0, 8.0),
        outer_margin: egui::Margin::ZERO,
        rounding: Rounding::same(4.0),
        shadow: egui::epaint::Shadow {
            offset: egui::vec2(2.0, 2.0),
            blur: 4.0,
            spread: 0.0,
            color: theme.panel_shadow,
        },
        stroke: Stroke::new(1.0, theme.panel_border),
    }
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
