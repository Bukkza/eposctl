//! Settings page: general options, then one panel per feature, then About.

use eframe::egui::{RichText, Ui};

use super::super::App;
use crate::autostart;
use crate::config::SmartButton;
use crate::strings::Lang;
use crate::theme::{self, Appearance};
use crate::widgets;

impl App {
    pub(super) fn page_settings(&mut self, ui: &mut Ui) {
        let s = self.s();
        widgets::card(ui, |ui| self.general(ui));
        ui.add_space(theme::SECTION_GAP);
        widgets::heading(ui, s.settings_automation);
        self.per_app_panel(ui);
        ui.add_space(theme::GAP);
        self.shortcuts_panel(ui);
        ui.add_space(theme::GAP);
        widgets::card(ui, |ui| self.smart_button_panel(ui));
        ui.add_space(theme::GAP);
        self.device_switch_panel(ui);
        ui.add_space(theme::SECTION_GAP);
        widgets::heading(ui, s.settings_management);
        self.preset_files_panel(ui);
        ui.add_space(theme::GAP);
        self.official_panel(ui);
        ui.add_space(theme::GAP);
        widgets::card(ui, |ui| self.about(ui));
    }

    fn general(&mut self, ui: &mut Ui) {
        let s = self.s();
        widgets::heading(ui, s.settings_general);
        ui.add_space(8.0);
        widgets::labeled_row(ui, s.appearance, |ui| {
            for (mode, label) in [
                (Appearance::Dark, s.appearance_dark),
                (Appearance::Light, s.appearance_light),
                (Appearance::System, s.appearance_system),
            ] {
                if widgets::chip(ui, label, self.cfg.appearance == mode).clicked() && self.cfg.appearance != mode {
                    self.cfg.appearance = mode;
                    mode.apply(ui.ctx());
                    ui.ctx().request_repaint();
                    self.mark_dirty();
                }
            }
        });
        widgets::divider(ui);
        widgets::labeled_row(ui, s.language, |ui| {
            // Right-to-left: English first so 中文 ends up on the left.
            for (lang, label) in [(Lang::En, "English"), (Lang::Zh, "中文")] {
                if widgets::chip(ui, label, self.cfg.lang == lang).clicked() && self.cfg.lang != lang {
                    self.cfg.lang = lang;
                    self.presets_changed();
                }
            }
        });
        widgets::spaced_divider(ui, 4.0, 4.0);

        let mut v = self.cfg.close_to_tray;
        if widgets::setting_row(ui, s.close_to_tray, s.close_to_tray_desc, &mut v, false) {
            self.cfg.close_to_tray = v;
            self.mark_dirty();
        }
        widgets::spaced_divider(ui, 4.0, 4.0);

        let mut v = self.autostart;
        if widgets::setting_row(ui, s.autostart, s.autostart_desc, &mut v, false) && autostart::set(v).is_ok() {
            self.autostart = v;
        }
        widgets::spaced_divider(ui, 4.0, 4.0);

        let mut v = self.cfg.restore_on_start;
        if widgets::setting_row(ui, s.restore, s.restore_desc, &mut v, false) {
            self.cfg.restore_on_start = v;
            self.mark_dirty();
        }
        widgets::spaced_divider(ui, 4.0, 4.0);

        let mut v = self.cfg.battery_alerts;
        if widgets::setting_row(ui, s.battery_alerts, s.battery_alerts_desc, &mut v, false) {
            self.cfg.battery_alerts = v;
            self.mark_dirty();
        }
    }

    fn smart_button_panel(&mut self, ui: &mut Ui) {
        let s = self.s();
        widgets::heading(ui, s.smart_button_title);
        widgets::desc(ui, s.smart_button_desc);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (action, label) in [
                (SmartButton::Off, s.smart_button_off),
                (SmartButton::NextPreset, s.hk_next),
                (SmartButton::ToggleSurround, s.hk_surround),
            ] {
                let selected = self.cfg.smart_button == action;
                if widgets::chip(ui, label, selected).clicked() && !selected {
                    self.cfg.smart_button = action;
                    self.mark_dirty();
                }
            }
        });
    }

    fn about(&self, ui: &mut Ui) {
        let s = self.s();
        let colors = theme::colors(ui);
        ui.horizontal(|ui| {
            widgets::heading(ui, s.about);
            ui.label(RichText::new(format!("eposctl {}", env!("CARGO_PKG_VERSION"))).color(colors.secondary));
        });
        ui.add_space(2.0);
        widgets::desc(ui, s.about_body);
    }
}
