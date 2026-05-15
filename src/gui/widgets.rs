//! Minimoog-style custom widgets for egui
//!
//! Provides skeuomorphic knobs, switches, and displays that match
//! the vintage synthesizer aesthetic.

use egui::*;

/// A Minimoog-style rotary knob
/// 
/// - Draggable vertically to change value
/// - Visual arc shows current value
/// - Tick marks at key positions
pub fn knob(
    ui: &mut Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
    suffix: &str,
) -> Response {
    let desired_size = vec2(60.0, 70.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    // Handle drag - accumulate from drag id
    if response.dragged() {
        let drag_delta = response.drag_delta().y;
        let range_size = range.end() - range.start();
        let sensitivity = range_size / 100.0; // Full range = 100px drag
        *value -= drag_delta * sensitivity;
        *value = value.clamp(*range.start(), *range.end());
        response.mark_changed();
    }
    
    // Handle click to start drag
    if response.is_pointer_button_down_on() {
        response = response.on_hover_cursor(CursorIcon::Grabbing);
    } else if response.hovered() {
        response = response.on_hover_cursor(CursorIcon::Grab);
    }

    // Get normalized value (0.0 to 1.0)
    let t = (*value - range.start()) / (range.end() - range.start());
    let t = t.clamp(0.0, 1.0);

    // Draw the knob
    let visuals = ui.style().interact(&response);
    let painter = ui.painter_at(rect);

    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.35;

    // Background circle (dark metal)
    painter.circle_filled(center, radius, Color32::from_gray(40));

    // Outer ring (highlight when active)
    let ring_color = if response.hovered() || response.dragged() {
        Color32::from_rgb(180, 140, 80) // Gold/brass highlight
    } else {
        Color32::from_gray(60)
    };
    painter.circle_stroke(center, radius, Stroke::new(2.0, ring_color));

    // Arc indicator (value visualization)
    let start_angle = -150_f32.to_radians();
    let end_angle = 150_f32.to_radians();
    let current_angle = start_angle + t * (end_angle - start_angle);

    // Draw arc using line segments (egui doesn't have native arc)
    fn draw_arc(painter: &Painter, center: Pos2, radius: f32, start: f32, end: f32, stroke: Stroke) {
        let segments = 20;
        let step = (end - start) / segments as f32;
        for i in 0..segments {
            let a1 = start + step * i as f32;
            let a2 = start + step * (i + 1) as f32;
            let p1 = center + vec2(a1.cos() * radius, a1.sin() * radius);
            let p2 = center + vec2(a2.cos() * radius, a2.sin() * radius);
            painter.line_segment([p1, p2], stroke);
        }
    }

    // Background arc (darker)
    draw_arc(
        &painter,
        center,
        radius * 0.85,
        start_angle,
        end_angle,
        Stroke::new(3.0, Color32::from_gray(30)),
    );

    // Value arc (warm orange/yellow like Minimoog)
    let value_color = Color32::from_rgb(255, 180, 60);
    draw_arc(
        &painter,
        center,
        radius * 0.85,
        start_angle,
        current_angle,
        Stroke::new(3.0, value_color),
    );

    // Indicator dot
    let dot_radius = radius * 0.6;
    let dot_pos = center + vec2(
        current_angle.cos() * dot_radius,
        current_angle.sin() * dot_radius,
    );
    painter.circle_filled(dot_pos, 4.0, Color32::WHITE);

    // Tick marks at 0%, 50%, 100%
    for tick_t in [0.0, 0.5, 1.0] {
        let tick_angle = start_angle + tick_t * (end_angle - start_angle);
        let tick_start = center + vec2(
            tick_angle.cos() * (radius + 3.0),
            tick_angle.sin() * (radius + 3.0),
        );
        let tick_end = center + vec2(
            tick_angle.cos() * (radius + 8.0),
            tick_angle.sin() * (radius + 8.0),
        );
        painter.line_segment([tick_start, tick_end], Stroke::new(2.0, Color32::from_gray(100)));
    }

    // Label below knob
    let label_rect = Rect::from_min_size(
        rect.left_bottom() - vec2(0.0, 20.0),
        vec2(rect.width(), 20.0),
    );
    painter.text(
        label_rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::from_gray(200),
    );

    // Value display
    let value_text = if suffix == "Hz" && *value >= 1000.0 {
        format!("{:.1}k{}", *value / 1000.0, suffix)
    } else if suffix == "%" {
        format!("{:.0}{}", *value * 100.0, suffix)
    } else if *value < 0.01 && *value > 0.0 {
        format!("{:.1}ms", *value * 1000.0)
    } else if *value < 1.0 {
        format!("{:.2}{}", *value, suffix)
    } else {
        format!("{:.1}{}", *value, suffix)
    };

    let value_rect = Rect::from_min_size(
        rect.left_bottom() - vec2(0.0, 38.0),
        vec2(rect.width(), 16.0),
    );
    painter.text(
        value_rect.center(),
        Align2::CENTER_CENTER,
        value_text,
        FontId::proportional(10.0),
        Color32::from_rgb(255, 180, 60), // Match arc color
    );

    response
}

/// A Minimoog-style toggle switch (for on/off parameters)
pub fn toggle_switch(ui: &mut Ui, value: &mut bool, label: &str) -> Response {
    let size = vec2(40.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());

    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }

    let painter = ui.painter_at(rect);
    let visuals = ui.style().interact(&response);

    // Background (switch track)
    let bg_color = if *value {
        Color32::from_rgb(80, 60, 40) // Dark when on (lit from below)
    } else {
        Color32::from_gray(30)
    };
    painter.rect_filled(rect, 4.0, bg_color);
    painter.rect_stroke(rect, 4.0, Stroke::new(2.0, Color32::from_gray(60)));

    // Switch position
    let thumb_radius = 8.0;
    let thumb_center = if *value {
        rect.right_center() - vec2(thumb_radius + 2.0, 0.0)
    } else {
        rect.left_center() + vec2(thumb_radius + 2.0, 0.0)
    };

    // Thumb (the moving part)
    let thumb_color = if *value {
        Color32::from_rgb(255, 180, 60) // Lit up when on
    } else {
        Color32::from_gray(100)
    };
    painter.circle_filled(thumb_center, thumb_radius, thumb_color);
    painter.circle_stroke(thumb_center, thumb_radius, Stroke::new(1.0, Color32::from_gray(40)));

    // Label
    painter.text(
        rect.center_bottom() + vec2(0.0, 15.0),
        Align2::CENTER_TOP,
        label,
        FontId::proportional(10.0),
        Color32::from_gray(200),
    );

    response
}

/// A Minimoog-style selector switch (for discrete choices)
pub fn selector_switch(
    ui: &mut Ui,
    current: &mut usize,
    options: &[&str],
    label: &str,
) -> Response {
    let n = options.len();
    let button_width = 50.0;
    let spacing = 4.0;
    let total_width = n as f32 * (button_width + spacing) - spacing;
    let height = 24.0;

    ui.vertical(|ui| {
        // Label
        ui.label(RichText::new(label).size(11.0).color(Color32::from_gray(200)));

        ui.horizontal(|ui| {
            let mut changed = false;
            let mut any_response: Option<Response> = None;

            for (i, option) in options.iter().enumerate() {
                let is_selected = *current == i;

                let text = RichText::new(*option)
                    .size(10.0)
                    .color(if is_selected {
                        Color32::from_rgb(40, 30, 20)
                    } else {
                        Color32::from_gray(200)
                    });

                let button = ui.add_sized(
                    [button_width, height],
                    egui::Button::new(text)
                        .fill(if is_selected {
                            Color32::from_rgb(255, 180, 60)
                        } else {
                            Color32::from_gray(40)
                        })
                        .stroke(Stroke::new(1.0, Color32::from_gray(60))),
                );

                if button.clicked() {
                    *current = i;
                    changed = true;
                }

                any_response = Some(if let Some(r) = any_response {
                    r.union(button)
                } else {
                    button
                });
            }

            if let Some(mut response) = any_response {
                if changed {
                    response.mark_changed();
                }
                response
            } else {
                ui.allocate_response(vec2(0.0, 0.0), Sense::hover())
            }
        }).inner
    }).inner
}

/// A vintage-style VU meter / level display
pub fn vu_meter(ui: &mut Ui, level: f32, label: &str) {
    let size = vec2(20.0, 100.0);
    let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());

    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 2.0, Color32::from_gray(20));
    painter.rect_stroke(rect, 2.0, Stroke::new(2.0, Color32::from_gray(60)));

    // Level bar (from bottom up)
    let level = level.clamp(0.0, 1.0);
    let bar_height = rect.height() * level;
    let bar_rect = Rect::from_min_size(
        rect.left_bottom() - vec2(0.0, bar_height),
        vec2(rect.width(), bar_height),
    );

    // Gradient: green -> yellow -> red
    let color = if level < 0.6 {
        Color32::from_rgb(60, 180, 60)
    } else if level < 0.85 {
        Color32::from_rgb(220, 180, 60)
    } else {
        Color32::from_rgb(220, 60, 60)
    };

    if bar_height > 0.0 {
        painter.rect_filled(bar_rect, 1.0, color);
    }

    // Label at bottom
    painter.text(
        rect.center_bottom() + vec2(0.0, 15.0),
        Align2::CENTER_TOP,
        label,
        FontId::proportional(10.0),
        Color32::from_gray(200),
    );
}

/// Draw a vintage panel background
pub fn panel_background(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    Frame::group(ui.style())
        .fill(Color32::from_rgb(35, 30, 25)) // Dark brown-gray like vintage synths
        .stroke(Stroke::new(2.0, Color32::from_gray(50)))
        .rounding(4.0)
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                add_contents(ui);
                ui.add_space(4.0);
            }).inner
        });
}

/// Section header with vintage styling
pub fn section_header(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(14.0)
            .strong()
            .color(Color32::from_rgb(255, 180, 60))
    );
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

/// MIDI feedback indicator - shows last received CC value
pub fn midi_indicator(ui: &mut Ui, cc: u8, value: Option<u8>, name: &str) {
    ui.horizontal(|ui| {
        // LED-style indicator
        let (rect, _response) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
        let painter = ui.painter_at(rect);

        let led_color = if value.is_some() {
            // Flash with received value brightness
            let brightness = value.unwrap_or(0) as f32 / 127.0;
            Color32::from_rgb(
                (255.0 * brightness) as u8,
                (200.0 * brightness) as u8,
                60,
            )
        } else {
            Color32::from_gray(40)
        };

        painter.circle_filled(rect.center(), 5.0, led_color);
        painter.circle_stroke(rect.center(), 5.0, Stroke::new(1.0, Color32::from_gray(80)));

        // Name and value
        let value_text = value
            .map(|v| format!("{}: {}", name, v))
            .unwrap_or_else(|| format!("{}: --", name));

        ui.label(
            RichText::new(value_text)
                .size(10.0)
                .monospace()
                .color(Color32::from_gray(180))
        );
    });
}
