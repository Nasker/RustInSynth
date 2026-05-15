//! Minimoog-style custom widgets for egui
//!
//! Provides skeuomorphic knobs, switches, and displays that match
//! the vintage synthesizer aesthetic with Rust In Peace theming.

use egui::*;
use super::theme::{THEME, panel_background, section_header};

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

    // Background circle (dark blue from artwork)
    painter.circle_filled(center, radius, THEME.bg_blue);

    // Outer ring (highlight when active)
    painter.circle_stroke(center, radius, THEME.knob_stroke(response.hovered(), response.dragged()));

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
        Stroke::new(3.0, THEME.steel_dark),
    );

    // Value arc (warning orange)
    draw_arc(
        &painter,
        center,
        radius * 0.85,
        start_angle,
        current_angle,
        Stroke::new(3.0, THEME.knob_value_color()),
    );

    // Indicator dot
    let dot_radius = radius * 0.6;
    let dot_pos = center + vec2(
        current_angle.cos() * dot_radius,
        current_angle.sin() * dot_radius,
    );
    painter.circle_filled(dot_pos, 4.0, THEME.toxic_green_light);

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
        painter.line_segment([tick_start, tick_end], Stroke::new(2.0, THEME.steel_medium));
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
        THEME.text_primary,
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
        THEME.knob_value_color(),
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
    let bg_color = THEME.toggle_switch(*value);
    painter.rect_filled(rect, 4.0, bg_color);
    painter.rect_stroke(rect, 4.0, Stroke::new(2.0, THEME.steel_medium));

    // Switch position
    let thumb_radius = 8.0;
    let thumb_center = if *value {
        rect.right_center() - vec2(thumb_radius + 2.0, 0.0)
    } else {
        rect.left_center() + vec2(thumb_radius + 2.0, 0.0)
    };

    // Thumb (the moving part)
    let thumb_color = if *value {
        THEME.toxic_green // Lit up when on
    } else {
        THEME.steel_medium
    };
    painter.circle_filled(thumb_center, thumb_radius, thumb_color);
    painter.circle_stroke(thumb_center, thumb_radius, Stroke::new(1.0, THEME.steel_dark));

    // Label
    painter.text(
        rect.center_bottom() + vec2(0.0, 15.0),
        Align2::CENTER_TOP,
        label,
        FontId::proportional(10.0),
        THEME.text_primary,
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

    let (rect, mut response) = ui.allocate_exact_size(
        vec2(total_width, height + 20.0),
        Sense::click(),
    );

    // Label
    let label_rect = Rect::from_min_size(
        rect.left_top(),
        vec2(rect.width(), 20.0),
    );
    ui.painter_at(label_rect).text(
        label_rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        THEME.text_primary,
    );

    // Buttons
    let button_rect = Rect::from_min_size(
        rect.left_top() + vec2(0.0, 20.0),
        vec2(rect.width(), height),
    );

    ui.horizontal(|ui| {
        ui.set_clip_rect(button_rect);
        
        let mut changed = false;
        let mut any_response: Option<Response> = None;

        for (i, option) in options.iter().enumerate() {
            let is_selected = *current == i;
            let x_offset = i as f32 * (button_width + spacing);
            let button_pos = button_rect.left_top() + vec2(x_offset, 0.0);
            let button_area = Rect::from_min_size(button_pos, vec2(button_width, height));

            let text = RichText::new(*option)
                .size(10.0)
                .color(if is_selected {
                    THEME.text_primary
                } else {
                    THEME.text_secondary
                });

            let button = ui.allocate_rect(button_area, Sense::click());
            let was_clicked = button.clicked();

            if button.hovered() || was_clicked {
                any_response = Some(if let Some(r) = any_response {
                    r.union(button)
                } else {
                    button
                });
            }

            if was_clicked {
                *current = i;
                changed = true;
            }

            // Draw button
            ui.painter_at(button_area).rect_filled(
                button_area,
                2.0,
                if is_selected {
                    THEME.gold
                } else {
                    THEME.steel_dark
                },
            );
            ui.painter_at(button_area).rect_stroke(
                button_area,
                2.0,
                Stroke::new(1.0, THEME.steel_medium),
            );
            ui.painter_at(button_area).text(
                button_area.center(),
                Align2::CENTER_CENTER,
                *option,
                FontId::proportional(10.0),
                if is_selected {
                    THEME.text_primary
                } else {
                    THEME.text_secondary
                },
            );
        }

        if let Some(mut r) = any_response {
            if changed {
                r.mark_changed();
            }
            response = response.union(r);
        }
    });

    response
}

/// A vintage-style VU meter / level display
pub fn vu_meter(ui: &mut Ui, level: f32, label: &str) {
    let size = vec2(20.0, 100.0);
    let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());

    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 2.0, THEME.steel_dark);
    painter.rect_stroke(rect, 2.0, Stroke::new(2.0, THEME.steel_medium));

    // Level bar (from bottom up)
    let level = level.clamp(0.0, 1.0);
    let bar_height = rect.height() * level;
    let bar_rect = Rect::from_min_size(
        rect.left_bottom() - vec2(0.0, bar_height),
        vec2(rect.width(), bar_height),
    );

    // Use themed VU meter colors
    let color = THEME.vu_meter_color(level);

    if bar_height > 0.0 {
        painter.rect_filled(bar_rect, 1.0, color);
    }

    // Label at bottom
    painter.text(
        rect.center_bottom() + vec2(0.0, 15.0),
        Align2::CENTER_TOP,
        label,
        FontId::proportional(10.0),
        THEME.text_primary,
    );
}

/// MIDI feedback indicator - shows last received CC value
pub fn midi_indicator(ui: &mut Ui, cc: u8, value: Option<u8>, name: &str) {
    ui.horizontal(|ui| {
        // LED-style indicator
        let (rect, _response) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
        let painter = ui.painter_at(rect);

        let led_color = if value.is_some() {
            // Use electric blue for MIDI activity
            THEME.midi_activity(true)
        } else {
            THEME.steel_dark
        };

        painter.circle_filled(rect.center(), 5.0, led_color);
        painter.circle_stroke(rect.center(), 5.0, Stroke::new(1.0, THEME.steel_medium));

        // Name and value
        let value_text = value
            .map(|v| format!("{}: {}", name, v))
            .unwrap_or_else(|| format!("{}: --", name));

        ui.label(
            RichText::new(value_text)
                .size(10.0)
                .monospace()
                .color(THEME.text_secondary)
        );
    });
}
