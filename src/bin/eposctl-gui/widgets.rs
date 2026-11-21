//! Cards and controls shared by the desktop pages.

use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Key, Layout, Margin, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, pos2, vec2,
};

use crate::theme;

/// The surface used for grouped controls.
pub fn panel(ui: &Ui) -> Frame {
    let colors = theme::colors(ui);
    Frame::new()
        .fill(colors.panel)
        .stroke(Stroke::new(1.0, colors.line))
        .shadow(egui::epaint::Shadow { offset: [0, 3], blur: 14, spread: 0, color: colors.shadow })
        .corner_radius(CornerRadius::same(theme::R_CARD))
        .inner_margin(Margin::same(24))
}

/// A `panel` spanning the available width.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    panel(ui)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// `left` on the left; `right` laid out from the right edge, vertically centred.
pub fn split_row<R>(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        left(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), right).inner
    })
    .inner
}

/// A heading with actions on the right, at the top of a panel.
pub fn heading_row<R>(ui: &mut Ui, title: &str, actions: impl FnOnce(&mut Ui) -> R) -> R {
    split_row(ui, |ui| heading(ui, title), actions)
}

/// A settings line: a label with a control on the right.
pub fn labeled_row<R>(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    let colors = theme::colors(ui);
    let left = |ui: &mut Ui| {
        ui.set_min_height(theme::CONTROL_HEIGHT);
        ui.label(RichText::new(label).color(colors.text));
    };
    split_row(ui, left, control)
}

/// Title and description with a switch on the right. True when toggled.
pub fn setting_row(ui: &mut Ui, title: &str, desc_text: &str, on: &mut bool, strong: bool) -> bool {
    let colors = theme::colors(ui);
    let left = |ui: &mut Ui| {
        ui.vertical(|ui| {
            ui.set_max_width(ui.available_width() - 70.0);
            if strong {
                heading(ui, title);
            } else {
                ui.label(RichText::new(title).color(colors.text));
            }
            desc(ui, desc_text);
        });
    };
    split_row(ui, left, |ui| switch(ui, on, true).changed())
}

/// A divider with space above and below.
pub fn spaced_divider(ui: &mut Ui, above: f32, below: f32) {
    ui.add_space(above);
    divider(ui);
    ui.add_space(below);
}

/// Single-line text field in the suite's style.
pub fn text_field<'a>(text: &'a mut String, hint: &'a str, width: f32) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text).hint_text(hint).desired_width(width).margin(Margin::symmetric(10, 7))
}

/// A text field with an "add" button. Returns the text when it is submitted
/// (button or Enter); the caller clears the field if it accepts it.
pub fn submit_field(ui: &mut Ui, text: &mut String, hint: &str, button: &str) -> Option<String> {
    ui.horizontal(|ui| {
        let te = ui.add(text_field(text, hint, 240.0));
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        (primary(ui, button).clicked() || enter).then(|| text.clone())
    })
    .inner
}

/// The result of an action, shown under the control that started it.
#[derive(Clone, Debug)]
pub struct Notice {
    text: String,
    ok: bool,
}

impl Notice {
    pub fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), ok: true }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), ok: false }
    }

    pub fn show(&self, ui: &mut Ui) {
        let colors = theme::colors(ui);
        let color = if self.ok { colors.success } else { colors.danger };
        ui.label(RichText::new(&self.text).font(theme::font(13.0)).color(color));
    }
}

/// Preset pill. `selected` = currently applied.
pub fn chip(ui: &mut Ui, text: &str, selected: bool) -> Response {
    let colors = theme::colors(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::font(14.0), Color32::PLACEHOLDER);
    let size = vec2((galley.size().x + 24.0).max(theme::CONTROL_HEIGHT), theme::CONTROL_HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    let (fill, stroke, fg) = if selected {
        (colors.accent_dim, Stroke::NONE, colors.accent)
    } else if hovered {
        (colors.hover, Stroke::NONE, colors.text)
    } else {
        (colors.raised, Stroke::NONE, colors.secondary)
    };
    let p = ui.painter();
    p.rect(rect.shrink2(vec2(0.0, 4.0)), CornerRadius::same(12), fill, stroke, StrokeKind::Inside);
    p.galley(rect.center() - galley.size() / 2.0, galley, fg);
    focus_ring(ui, &resp, rect, 12);
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Lightweight action button for secondary controls.
pub fn ghost(ui: &mut Ui, text: &str) -> Response {
    ghost_toggle(ui, text, true, false)
}

/// `ghost` with an enabled state and an "on" state (accent outline).
pub fn ghost_toggle(ui: &mut Ui, text: &str, enabled: bool, on: bool) -> Response {
    let colors = theme::colors(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::font(13.0), Color32::PLACEHOLDER);
    let size = vec2((galley.size().x + 28.0).max(theme::CONTROL_HEIGHT), theme::CONTROL_HEIGHT);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    let hot = enabled && resp.hovered();
    let fg = if !enabled {
        colors.panel.lerp_to_gamma(colors.subdued, 0.65)
    } else if on {
        colors.accent
    } else if hot {
        colors.text
    } else {
        colors.secondary
    };
    let fill = if on {
        colors.accent_dim
    } else if hot {
        colors.hover
    } else {
        Color32::TRANSPARENT
    };
    let visual = rect.shrink2(vec2(0.0, 4.0));
    ui.painter().rect(visual, CornerRadius::same(18), fill, Stroke::new(1.0, colors.line), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, fg);
    focus_ring(ui, &resp, visual, 18);
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Filled accent button for the primary action in a small form.
pub fn primary(ui: &mut Ui, text: &str) -> Response {
    primary_enabled(ui, text, true)
}

pub fn primary_enabled(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let colors = theme::colors(ui);
    let color = if enabled { colors.on_accent } else { colors.subdued };
    let galley = ui.painter().layout_no_wrap(text.to_owned(), theme::font_strong(14.0), color);
    let size = vec2(galley.size().x + 32.0, theme::CONTROL_HEIGHT);
    let (rect, resp) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let hover = ui.ctx().animate_bool_with_time(resp.id, enabled && resp.hovered(), 0.14);
    let fill = if enabled { colors.accent.lerp_to_gamma(colors.accent_hover, hover) } else { colors.raised };
    let visual = rect.shrink2(vec2(0.0, 3.0));
    ui.painter().rect_filled(visual, CornerRadius::same(19), fill);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    focus_ring(ui, &resp, visual, 19);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

fn focus_ring(ui: &Ui, resp: &Response, rect: Rect, r: u8) {
    let colors = theme::colors(ui);
    if resp.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            CornerRadius::same(r + 3),
            Stroke::new(1.5, colors.accent.gamma_multiply(0.7)),
            StrokeKind::Outside,
        );
    }
}

/// On/off switch. Returns a response that is `changed()` when toggled.
pub fn switch(ui: &mut Ui, on: &mut bool, enabled: bool) -> Response {
    let colors = theme::colors(ui);
    let size = vec2(48.0, theme::CONTROL_HEIGHT);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, mut resp) = ui.allocate_exact_size(size, sense);
    if enabled && resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let track = if enabled { colors.raised.lerp_to_gamma(colors.accent, t) } else { colors.raised };
    let pill = Rect::from_center_size(rect.center(), vec2(46.0, 26.0));
    let p = ui.painter();
    p.rect(
        pill,
        CornerRadius::same(13),
        track,
        Stroke::new(1.0, if *on { colors.accent } else { colors.line }),
        StrokeKind::Inside,
    );
    let x = egui::lerp((pill.left() + 13.0)..=(pill.right() - 13.0), t);
    let knob = if enabled { Color32::WHITE } else { colors.panel };
    p.circle_filled(pos2(x, pill.center().y), 10.0, knob);
    p.circle_stroke(pos2(x, pill.center().y), 10.0, Stroke::new(1.0, colors.line));
    focus_ring(ui, &resp, pill, 13);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

/// Horizontal slider over 0..=1 with an accent fill. Arrow keys step 1%.
pub fn slider(ui: &mut Ui, value: &mut f32, enabled: bool) -> Response {
    let colors = theme::colors(ui);
    let h = theme::CONTROL_HEIGHT;
    let sense = if enabled { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, mut resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), sense);
    let rail = Rect::from_center_size(rect.center(), vec2(rect.width() - 16.0, 4.0));
    if enabled {
        if let Some(p) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                let v = egui::remap_clamp(p.x, rail.left()..=rail.right(), 0.0..=1.0);
                let v = (v * 100.0).round() / 100.0;
                if v != *value {
                    *value = v;
                    resp.mark_changed();
                }
            }
            resp.request_focus();
        }
        if resp.has_focus() {
            let (l, r) = ui.input(|i| (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight)));
            let d = if r {
                0.01
            } else if l {
                -0.01
            } else {
                0.0
            };
            if d != 0.0 {
                *value = ((*value + d) * 100.0).round().clamp(0.0, 100.0) / 100.0;
                resp.mark_changed();
            }
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(resp.id, egui::EventFilter { horizontal_arrows: true, ..Default::default() });
            });
        }
    }
    let p = ui.painter();
    let x = egui::lerp(rail.left()..=rail.right(), value.clamp(0.0, 1.0));
    let (fill, knob) = if enabled { (colors.accent, Color32::WHITE) } else { (colors.line, colors.raised) };
    p.rect_filled(rail, CornerRadius::same(2), colors.raised);
    p.rect_filled(Rect::from_min_max(rail.min, pos2(x, rail.max.y)), CornerRadius::same(2), fill);
    let active = enabled && (resp.hovered() || resp.dragged() || resp.has_focus());
    let knob_radius = if active { 10.0 } else { 9.0 };
    p.circle_filled(pos2(x, rail.center().y), knob_radius, knob);
    p.circle_stroke(
        pos2(x, rail.center().y),
        knob_radius,
        Stroke::new(1.0, if active { colors.accent } else { colors.subdued }),
    );
    if resp.has_focus() {
        p.circle_stroke(pos2(x, rail.center().y), 12.0, Stroke::new(1.5, colors.accent.gamma_multiply(0.6)));
    }
    resp
}

/// Title on the left, optional right-aligned value, used above a slider.
pub fn row_title(ui: &mut Ui, title: &str, value: Option<String>, enabled: bool) {
    let colors = theme::colors(ui);
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = 22.0;
        let title_color = if enabled { colors.text } else { colors.secondary };
        let label = RichText::new(title).font(theme::font_strong(17.0)).color(title_color);
        if let Some(v) = value {
            let value_color = if enabled { colors.accent } else { colors.subdued };
            split_row(
                ui,
                |ui| {
                    ui.label(label);
                },
                |ui| {
                    ui.label(RichText::new(v).font(theme::font_strong(15.0)).color(value_color));
                },
            );
        } else {
            ui.set_min_height(22.0);
            ui.label(label);
        }
    });
}

pub fn desc(ui: &mut Ui, text: &str) {
    let colors = theme::colors(ui);
    ui.label(egui::RichText::new(text).font(theme::font(13.0)).color(colors.secondary));
}

/// Battery glyph with level fill; colour warns when low.
pub fn battery(ui: &mut Ui, level: u8) {
    let colors = theme::colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(28.0, 14.0), Sense::hover());
    let body = Rect::from_min_size(rect.min, vec2(25.0, 14.0));
    let p = ui.painter();
    p.rect_stroke(body, CornerRadius::same(3), Stroke::new(1.2, colors.secondary), StrokeKind::Inside);
    p.rect_filled(
        Rect::from_min_size(pos2(body.right() + 1.0, body.center().y - 3.0), vec2(2.0, 6.0)),
        CornerRadius::same(1),
        colors.secondary,
    );
    let color = match level {
        0..=10 => colors.danger,
        11..=20 => colors.warn,
        _ => colors.accent,
    };
    let inner = body.shrink(2.5);
    let w = inner.width() * level.min(100) as f32 / 100.0;
    p.rect_filled(Rect::from_min_size(inner.min, vec2(w.max(1.0), inner.height())), CornerRadius::same(1), color);
}

/// Status dot.
pub fn dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// Thin divider spanning the available width.
pub fn divider(ui: &mut Ui) {
    let colors = theme::colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment([rect.left_center(), rect.right_center()], Stroke::new(1.0, colors.line));
}

/// Label/value line for the device page.
pub fn info_row(ui: &mut Ui, label: &str, value: &str, value_color: Color32) {
    let colors = theme::colors(ui);
    ui.horizontal(|ui| {
        ui.set_min_height(36.0);
        ui.label(egui::RichText::new(label).color(colors.secondary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).color(value_color));
        });
    });
}

/// Small rotating arc used while waiting.
pub fn spinner(ui: &mut Ui) {
    let colors = theme::colors(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let c = rect.center();
    let pts: Vec<_> = (0..=16)
        .map(|i| {
            let a = t * 4.0 + i as f32 / 16.0 * 4.0;
            c + vec2(a.cos(), a.sin()) * 6.0
        })
        .collect();
    ui.painter().line(pts, Stroke::new(1.8, colors.accent));
    ui.ctx().request_repaint();
}

/// Left-aligned section heading inside the page.
pub fn heading(ui: &mut Ui, text: &str) {
    let colors = theme::colors(ui);
    ui.label(egui::RichText::new(text).font(theme::font_strong(18.0)).color(colors.text));
}

/// Height of `level_meter`, so a neighbouring panel can reserve the same space.
pub const LEVEL_ROW_H: f32 = 20.0;

/// Live input level under a slider, on one row: a bar aligned with the slider
/// rail, a peak tick, and the value in dB. `floor..=0` dB spans the bar.
pub fn level_meter(ui: &mut Ui, label: &str, level: f32, peak: f32, floor: f32) {
    let colors = theme::colors(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), LEVEL_ROW_H), Sense::hover());
    let value = if level > floor { format!("{level:.0} dB") } else { format!("< {floor:.0} dB") };
    let galley = ui.painter().layout_no_wrap(value, theme::font(12.0), colors.subdued);
    let text_w = 56.0;
    // Same left inset as the slider rail so the two line up.
    let bar = Rect::from_min_max(
        pos2(rect.left() + 8.0, rect.center().y - 3.0),
        pos2(rect.right() - 8.0 - text_w, rect.center().y + 3.0),
    );
    let x = |db: f32| egui::lerp(bar.left()..=bar.right(), ((db - floor) / -floor).clamp(0.0, 1.0));
    let p = ui.painter();
    p.rect_filled(bar, CornerRadius::same(3), colors.raised);
    if level > floor {
        p.rect_filled(Rect::from_min_max(bar.min, pos2(x(level), bar.max.y)), CornerRadius::same(3), colors.accent);
    }
    if peak > floor {
        let px = x(peak);
        p.line_segment([pos2(px, bar.top() - 2.0), pos2(px, bar.bottom() + 2.0)], Stroke::new(2.0, colors.text));
    }
    p.galley(
        pos2(rect.right() - 8.0 - galley.size().x, rect.center().y - galley.size().y / 2.0),
        galley,
        colors.subdued,
    );
    resp.on_hover_text(label);
}
