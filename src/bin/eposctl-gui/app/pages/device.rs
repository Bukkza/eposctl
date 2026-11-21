//! Device page: what the headset and the driver report.

use eframe::egui::{Color32, CornerRadius, Rect, RichText, Sense, Ui, pos2, vec2};

use super::super::App;
use crate::theme::{self, Palette};
use crate::widgets;
use crate::worker::{Cmd, DevState};

impl App {
    pub(super) fn page_device(&mut self, ui: &mut Ui, d: &DevState) {
        widgets::card(ui, |ui| self.device_battery(ui, d.battery));
        ui.add_space(theme::GAP);
        let rows = self.device_rows(d, theme::colors(ui));
        widgets::card(ui, |ui| {
            widgets::heading(ui, self.s().device_details);
            ui.add_space(8.0);
            for (i, (label, value, color)) in rows.iter().enumerate() {
                if i > 0 {
                    widgets::divider(ui);
                }
                widgets::info_row(ui, label, value, *color);
            }
        });
    }

    fn device_battery(&self, ui: &mut Ui, battery: Option<u8>) {
        let s = self.s();
        let colors = theme::colors(ui);
        widgets::heading_row(ui, s.battery, |ui| {
            if widgets::ghost(ui, s.refresh).clicked() {
                let _ = self.tx.send(Cmd::Refresh);
            }
        });
        let Some(level) = battery else {
            widgets::desc(ui, s.unknown);
            return;
        };
        let color = match level {
            0..=10 => colors.danger,
            11..=20 => colors.warn,
            _ => colors.success,
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{level}%")).font(theme::font_strong(36.0)).color(colors.text));
            if let Some(left) = self.battery_left(level) {
                widgets::desc(ui, &left);
            }
        });
        let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
        ui.painter().rect_filled(track, CornerRadius::same(4), colors.raised);
        if level > 0 {
            let x = track.left() + track.width() * f32::from(level.min(100)) / 100.0;
            let fill = Rect::from_min_max(track.min, pos2(x, track.bottom()));
            ui.painter().rect_filled(fill, CornerRadius::same(4), color);
        }
    }

    /// (label, value, value colour) for each line.
    fn device_rows(&self, d: &DevState, colors: &Palette) -> Vec<(&'static str, String, Color32)> {
        let s = self.s();
        let or_dash = |v: &Option<String>| v.clone().unwrap_or_else(|| s.unknown.to_owned());
        let (link, link_color) = match (d.dongle, d.linked) {
            (false, _) => (s.dongle_missing, colors.secondary),
            (true, Some(true)) => (s.linked, colors.success),
            (true, _) => (s.not_linked, colors.warn),
        };
        let (anc, anc_color) = match d.anc {
            Some(true) => (s.anc_on, colors.success),
            Some(false) => (s.anc_off, colors.text),
            None => (s.anc_unknown, colors.subdued),
        };
        let apo = match self.editor.device() {
            Some(dev) => (dev.to_string(), colors.text),
            None => (s.apo_missing_short.to_owned(), colors.secondary),
        };
        vec![
            (s.link, link.to_owned(), link_color),
            (s.product, or_dash(&d.product), colors.text),
            (s.serial, or_dash(&d.serial), colors.text),
            (s.firmware, or_dash(&d.firmware), colors.text),
            (s.anc, anc.to_owned(), anc_color),
            (s.apo_channel, apo.0, apo.1),
        ]
    }
}
