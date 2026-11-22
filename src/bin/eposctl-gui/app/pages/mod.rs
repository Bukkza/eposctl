//! What the window shows: navigation, header and one module per page or panel.

mod chat_mix;
mod device;
mod device_switch;
mod mic;
mod official;
mod parametric;
mod per_app;
mod preset_files;
mod settings;
mod shortcuts;
mod sound;

use eframe::egui::{
    self, Align2, CornerRadius, Frame, Margin, Rect, RichText, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};

use super::App;
use crate::icons::{self, Icon};
use crate::strings::Strings;
use crate::theme;
use crate::widgets;
use crate::worker::DevState;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Sound,
    Mic,
    Device,
    Settings,
}

impl Page {
    fn label(self, s: &Strings) -> &'static str {
        match self {
            Self::Sound => s.nav_sound,
            Self::Mic => s.nav_mic,
            Self::Device => s.nav_device,
            Self::Settings => s.nav_settings,
        }
    }

    /// For `EPOSCTL_PAGE` in debug builds; unknown names give the Sound page.
    #[cfg(debug_assertions)]
    pub fn from_name(name: &str) -> Self {
        match name {
            "mic" => Self::Mic,
            "device" => Self::Device,
            "settings" => Self::Settings,
            _ => Self::Sound,
        }
    }
}

/// Normalise what the user typed into a lower-case executable file name.
fn exe_name(input: &str) -> Option<String> {
    let name = input.trim().trim_matches('"').rsplit(['\\', '/']).next()?.trim().to_lowercase();
    if name.is_empty() {
        return None;
    }
    Some(if name.contains('.') { name } else { format!("{name}.exe") })
}

/// A program-name field with an add button: the typed name, normalised, when
/// submitted. The field is cleared then.
fn exe_input(ui: &mut Ui, input: &mut String, hint: &str, button: &str) -> Option<String> {
    let exe = widgets::submit_field(ui, input, hint, button).as_deref().and_then(exe_name)?;
    input.clear();
    Some(exe)
}

impl App {
    /// The current page, scrollable.
    pub(super) fn page(&mut self, ui: &mut Ui, d: &DevState) {
        const SCROLL_GUTTER: i8 = 12;
        let mut viewport = ui.available_rect_before_wrap();
        viewport.max.x += f32::from(SCROLL_GUTTER);
        let area = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .content_margin(Margin { right: SCROLL_GUTTER, ..Margin::ZERO });
        #[cfg(debug_assertions)]
        let area = match crate::debug::scroll() {
            Some(y) => area.vertical_scroll_offset(y),
            None => area,
        };
        ui.scope_builder(UiBuilder::new().max_rect(viewport), |ui| {
            area.show(ui, |ui| {
                match self.view.page {
                    Page::Sound => self.page_sound(ui),
                    Page::Mic => self.page_mic(ui, d),
                    Page::Device => self.page_device(ui, d),
                    Page::Settings => self.page_settings(ui),
                }
                ui.add_space(theme::SECTION_GAP);
            });
        });
    }

    fn go_to(&mut self, page: Page) {
        if self.view.page == Page::Mic && page != Page::Mic {
            self.mic_tools.close();
        }
        self.view.page = page;
        self.view.naming = None;
        self.view.recording = None;
        self.editor.stop_compare();
    }

    /// The navigation rail.
    pub(super) fn nav(&mut self, ui: &mut Ui) {
        let s = self.s();
        let colors = theme::colors(ui);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let (mark, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
            ui.painter().image(
                self.brand.id(),
                mark,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            ui.label(RichText::new("eposctl").font(theme::font_strong(21.0)).color(colors.text));
        });
        ui.add_space(theme::SECTION_GAP);

        for (page, icon) in [
            (Page::Sound, Icon::Sound),
            (Page::Mic, Icon::Mic),
            (Page::Device, Icon::Device),
            (Page::Settings, Icon::Settings),
        ] {
            if nav_item(ui, icon, page.label(s), self.view.page == page) {
                self.go_to(page);
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.spacing_mut().interact_size.y = 18.0;
            ui.spacing_mut().item_spacing.y = 4.0;
            let connected = self.editor.device().is_some();
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                widgets::dot(ui, if connected { colors.success } else { colors.subdued });
                widgets::desc(ui, if connected { s.linked } else { s.apo_missing_short });
            });
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(RichText::new(s.apo_channel).font(theme::font_strong(13.0)).color(colors.text));
            });
        });
    }

    /// Product name with link and battery status.
    pub(super) fn header(&self, ui: &mut Ui, d: &DevState) {
        let s = self.s();
        let colors = theme::colors(ui);
        let title = |ui: &mut Ui| {
            ui.label(RichText::new(self.view.page.label(s)).font(theme::font_strong(32.0)).color(colors.text));
        };
        widgets::split_row(ui, title, |ui| {
            Frame::new()
                .fill(colors.panel)
                .stroke(Stroke::new(1.0, colors.line))
                .inner_margin(Margin::symmetric(12, 5))
                .corner_radius(CornerRadius::same(14))
                .show(ui, |ui| self.header_device(ui, d));
        });
    }

    fn header_device(&self, ui: &mut Ui, d: &DevState) {
        let s = self.s();
        let colors = theme::colors(ui);
        let (status, indicator) = match (d.dongle, d.linked) {
            (false, _) => (s.dongle_missing, colors.subdued),
            (true, Some(true)) => (s.linked, colors.success),
            (true, _) => (s.not_linked, colors.warn),
        };
        // The header lays this capsule out from the right edge.
        if let (true, Some(true), Some(b)) = (d.dongle, d.linked, d.battery) {
            ui.label(RichText::new(format!("{b}%")).font(theme::font_strong(14.0)).color(colors.text));
            widgets::battery(ui, b);
            ui.add_space(4.0);
        }
        widgets::desc(ui, status);
        widgets::dot(ui, indicator);
        ui.label(
            RichText::new(d.product.as_deref().unwrap_or(s.headset)).font(theme::font_strong(14.0)).color(colors.text),
        );
        let (glyph, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
        ui.painter().rect_filled(glyph, CornerRadius::same(8), colors.accent_dim);
        icons::paint(ui.painter(), glyph.shrink(6.0), Icon::Device, colors.accent);
    }

    /// Placeholder while the EPOS driver's channel is not found.
    fn apo_missing(&self, ui: &mut Ui) {
        let s = self.s();
        let colors = theme::colors(ui);
        widgets::card(ui, |ui| {
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(14), colors.accent_dim);
                icons::paint(ui.painter(), r.shrink(12.0), Icon::Sound, colors.accent);
                ui.add_space(8.0);
                ui.vertical(|ui| apo_missing_text(ui, s));
            });
        });
    }
}

fn apo_missing_text(ui: &mut Ui, s: &Strings) {
    widgets::heading(ui, s.apo_missing_title);
    widgets::desc(ui, s.apo_missing_body);
    ui.horizontal(|ui| {
        widgets::spinner(ui);
        widgets::desc(ui, s.apo_retrying);
    });
}

/// One entry of the navigation rail. True when clicked.
fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> bool {
    let colors = theme::colors(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), theme::CONTROL_HEIGHT), Sense::click());
    let hot = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.14);
    let tint = if selected { colors.accent_dim } else { colors.sidebar.lerp_to_gamma(colors.hover, hot) };
    let color = if selected { colors.accent } else { colors.text };
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(12), tint);
    if selected {
        let bar = Rect::from_center_size(pos2(rect.left() + 3.0, rect.center().y), vec2(3.0, 18.0));
        p.rect_filled(bar, CornerRadius::same(2), colors.accent);
    }
    if resp.has_focus() {
        p.rect_stroke(rect, CornerRadius::same(12), Stroke::new(1.5, colors.accent), egui::StrokeKind::Inside);
    }
    let y = rect.center().y;
    icons::paint(p, Rect::from_center_size(pos2(rect.left() + 22.0, y), vec2(19.0, 19.0)), icon, color);
    p.text(pos2(rect.left() + 42.0, y), Align2::LEFT_CENTER, label, theme::font_strong(14.0), color);
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::exe_name;

    #[test]
    fn exe_names_are_normalised() {
        assert_eq!(exe_name(" Valorant ").as_deref(), Some("valorant.exe"));
        assert_eq!(exe_name(r#""C:\Riot Games\VALORANT\Game.EXE""#).as_deref(), Some("game.exe"));
        assert_eq!(exe_name("D:/Games/cs2.exe").as_deref(), Some("cs2.exe"));
        assert_eq!(exe_name("   "), None);
    }
}
