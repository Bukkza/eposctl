//! Mic page: noise gate with the live level, sidetone, the headset's noise
//! reduction, and the microphone EQ with its spectrum.

use std::time::{Duration, Instant};

use eframe::egui::{RichText, Ui};
use eposctl::apo::layout::BANDS;

use super::super::{App, EqTarget};
use crate::mic_calibration::{Result as GainResult, SECONDS, Status as GainStatus};
use crate::mic_tools::{MicStatus, RECORD_SECONDS};
use crate::theme;
use crate::worker::{Cmd, DevState};
use crate::{eq, widgets};

/// A sidetone value set here is shown this long before the headset's own reading.
const SIDETONE_HOLD: Duration = Duration::from_millis(1500);

/// Convert the worker's ascending 0–255 level into the slider's 0–1 range.
fn sidetone_ui(level: u16) -> f32 {
    f32::from(level.min(255)) / 255.0
}

fn sidetone_level(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u16
}

fn tool_hint(ui: &mut Ui, text: &str) {
    ui.scope(|ui| {
        if ui.available_width() < 400.0 {
            ui.set_min_height(36.0);
        }
        widgets::desc(ui, text);
    });
    ui.add_space(4.0);
}

impl App {
    pub(super) fn page_mic(&mut self, ui: &mut Ui, d: &DevState) {
        let mic = self.mic_tools.status();
        widgets::card(ui, |ui| self.mic_input(ui, &mic));
        ui.add_space(theme::GAP);
        self.mic_tool_cards(ui, &mic);
        ui.add_space(theme::GAP);
        let gate = self.editor.settings().map(eposctl::apo::Settings::noise_gate);
        widgets::card(ui, |ui| {
            self.sidetone(ui, d);
            if let Some(level) = d.noise_reduction.filter(|_| d.linked == Some(true)) {
                widgets::spaced_divider(ui, 8.0, 8.0);
                self.noise_reduction(ui, level);
            }
            if let Some(gate) = gate {
                widgets::spaced_divider(ui, 8.0, 8.0);
                self.noise_gate(ui, gate);
            }
        });
        ui.add_space(theme::GAP);
        if gate.is_some() {
            widgets::card(ui, |ui| self.mic_eq(ui));
        } else {
            widgets::desc(ui, self.s().mic_apo_needed);
        }
    }

    fn mic_tool_cards(&mut self, ui: &mut Ui, mic: &MicStatus) {
        if ui.available_width() >= 620.0 {
            ui.columns(2, |columns| {
                widgets::card(&mut columns[0], |ui| self.mic_calibration(ui, mic));
                widgets::card(&mut columns[1], |ui| self.mic_test(ui, mic));
            });
        } else {
            widgets::card(ui, |ui| self.mic_calibration(ui, mic));
            ui.add_space(theme::GAP);
            widgets::card(ui, |ui| self.mic_test(ui, mic));
        }
    }

    fn mic_input(&mut self, ui: &mut Ui, mic: &MicStatus) {
        let s = self.s();
        let mut level = mic.volume.unwrap_or_default();
        let enabled = mic.volume.is_some() && !mic.calibration.measuring();
        widgets::row_title(ui, s.mic_volume, mic.volume.map(|v| format!("{:.0}%", v * 100.0)), enabled);
        widgets::desc(ui, mic.name.as_deref().unwrap_or(s.mic_missing));
        ui.add_space(4.0);
        if widgets::slider(ui, &mut level, enabled).changed() {
            self.mic_tools.set_volume(level);
        }
        self.mic_input_meter(ui, mic);
        if let Some(error) = &mic.error {
            widgets::Notice::error(error.as_str()).show(ui);
        }
    }

    fn mic_input_meter(&self, ui: &mut Ui, mic: &MicStatus) {
        let floor = eq::METER_FLOOR_DB;
        if let Some(peak) = mic.peak {
            let db = (20.0 * peak.log10()).clamp(floor, 0.0);
            widgets::level_meter(ui, self.s().input_level, db, db, floor);
            if mic.clipping {
                ui.label(RichText::new(self.s().mic_clipping).color(theme::colors(ui).danger));
            }
        } else if self.editor.is_metering() {
            widgets::level_meter(ui, self.s().input_level, self.meters.level, self.meters.peak, floor);
        }
    }

    fn mic_calibration(&mut self, ui: &mut Ui, mic: &MicStatus) {
        let s = self.s();
        let measuring = mic.calibration.measuring();
        let ready = mic.volume.is_some() && mic.peak.is_some() && mic.recording_for.is_none() && !mic.playing;
        widgets::row_title(ui, s.mic_calibration, None, true);
        let message = match mic.calibration {
            GainStatus::Idle => s.mic_calibration_hint.replace("{seconds}", &SECONDS.to_string()),
            GainStatus::Measuring(remaining) => {
                s.mic_calibration_measuring.replace("{seconds}", &format!("{:.0}", remaining.as_secs_f32().ceil()))
            }
            GainStatus::Done(GainResult::InsufficientSpeech) => s.mic_calibration_quiet.to_owned(),
            GainStatus::Done(GainResult::WithinTarget(peak)) => {
                s.mic_calibration_ok.replace("{peak}", &format!("{peak:.0}"))
            }
            GainStatus::Done(GainResult::AtLimit(peak)) => {
                s.mic_calibration_limit.replace("{peak}", &format!("{peak:.0}"))
            }
            GainStatus::Done(GainResult::GainChanged) => s.mic_calibration_changed.to_owned(),
            GainStatus::Done(GainResult::Adjust(r)) => s
                .mic_calibration_suggestion
                .replace("{peak}", &format!("{:.0}", r.peak_dbfs))
                .replace("{change}", &format!("{:+.1}", r.change_db())),
            GainStatus::Done(GainResult::Applied(change)) => {
                s.mic_calibration_applied.replace("{change}", &format!("{change:+.1}"))
            }
        };
        tool_hint(ui, &message);
        ui.horizontal_wrapped(|ui| {
            let suggested = matches!(mic.calibration, GainStatus::Done(GainResult::Adjust(_)));
            if suggested && widgets::primary_enabled(ui, s.mic_calibration_apply, ready).clicked() {
                self.mic_tools.apply_calibration();
            }
            let label = if measuring { s.mic_calibration_cancel } else { s.mic_calibration_start };
            let response = if suggested || measuring {
                widgets::ghost_toggle(ui, label, ready || measuring, measuring)
            } else {
                widgets::primary_enabled(ui, label, ready)
            };
            if response.clicked() {
                self.mic_tools.calibrate(!measuring);
            }
        });
    }

    fn mic_test(&mut self, ui: &mut Ui, mic: &MicStatus) {
        let s = self.s();
        let recording = mic.recording_for.is_some();
        widgets::row_title(
            ui,
            s.mic_test,
            mic.recording_for.map(|d| format!("{:.1} / {RECORD_SECONDS} s", d.as_secs_f32())),
            true,
        );
        tool_hint(ui, &s.mic_test_hint.replace("{seconds}", &RECORD_SECONDS.to_string()));
        ui.horizontal_wrapped(|ui| {
            let record = if recording { s.mic_record_stop } else { s.mic_record };
            let play = if mic.playing { s.mic_play_stop } else { s.mic_play };
            let available = !mic.calibration.measuring();
            if widgets::ghost_toggle(ui, record, available && mic.peak.is_some() && !mic.playing, recording).clicked() {
                self.mic_tools.record(!recording);
            }
            if widgets::ghost_toggle(ui, play, available && mic.has_clip && !recording, mic.playing).clicked() {
                self.mic_tools.play(!mic.playing, ui.ctx());
            }
        });
    }

    fn noise_gate(&mut self, ui: &mut Ui, gate: f32) {
        let s = self.s();
        widgets::row_title(ui, s.noise_gate, Some(format!("{:.0}%", gate * 100.0)), true);
        widgets::desc(ui, s.noise_gate_desc);
        ui.add_space(6.0);
        let mut v = gate;
        if widgets::slider(ui, &mut v, true).changed() {
            self.apply(|st| st.set_noise_gate(v));
        }
    }

    fn sidetone(&mut self, ui: &mut Ui, d: &DevState) {
        let s = self.s();
        let linked = d.linked == Some(true) && d.sidetone.is_some();
        let pct = self
            .view
            .sidetone_drag
            .filter(|(_, t)| t.elapsed() < SIDETONE_HOLD)
            .map(|(v, _)| v)
            .or(d.sidetone.map(sidetone_ui))
            .unwrap_or(0.0);
        widgets::row_title(ui, s.sidetone, linked.then(|| format!("{:.0}%", pct * 100.0)), linked);
        widgets::desc(ui, if linked { s.sidetone_desc } else { s.sidetone_unavailable });
        ui.add_space(6.0);
        let mut v = pct;
        let r = widgets::slider(ui, &mut v, linked);
        if r.changed() {
            self.view.sidetone_drag = Some((v, Instant::now()));
            let _ = self.tx.send(Cmd::SetSidetone(sidetone_level(v)));
        }
        if r.dragged()
            && let Some((_, t)) = &mut self.view.sidetone_drag
        {
            *t = Instant::now();
        }
    }

    fn noise_reduction(&mut self, ui: &mut Ui, level: u16) {
        let s = self.s();
        let title = |ui: &mut Ui| {
            ui.vertical(|ui| {
                widgets::row_title(ui, s.noise_reduction, None, true);
                widgets::desc(ui, s.noise_reduction_desc);
            });
        };
        widgets::split_row(ui, title, |ui| {
            for (l, label) in [(2, s.nr_strong), (1, s.nr_medium), (0, s.nr_off)] {
                if widgets::chip(ui, label, level == l).clicked() && level != l {
                    let _ = self.tx.send(Cmd::SetNoiseReduction(l));
                }
            }
        });
    }

    fn mic_eq(&mut self, ui: &mut Ui) {
        let s = self.s();
        widgets::heading_row(ui, s.mic_eq, |ui| {
            if widgets::ghost(ui, s.reset).clicked() {
                self.apply(|st| st.set_mic_eq(&[0.0; BANDS]));
            }
            if widgets::ghost(ui, s.import).on_hover_text(s.eq_import_tip).clicked() {
                self.import_eq(EqTarget::Mic);
            }
        });
        ui.add_space(6.0);
        let mut g = self.editor.settings().map(eposctl::apo::Settings::mic_eq).unwrap_or_default();
        let bands = self.meters.bands;
        let levels = self.editor.is_metering().then_some(&bands[..]);
        if eq::editor(ui, "mic", &mut g, 230.0, levels) {
            self.apply(|st| st.set_mic_eq(&g));
        }
        self.eq_notice_line(ui, EqTarget::Mic);
        widgets::desc(ui, if levels.is_some() { s.mic_eq_hint } else { s.eq_hint });
    }
}

#[cfg(test)]
mod tests {
    use super::{sidetone_level, sidetone_ui};

    #[test]
    fn sidetone_increases_with_the_slider_and_round_trips() {
        assert_eq!(sidetone_level(0.0), 0);
        assert_eq!(sidetone_level(0.5), 128);
        assert_eq!(sidetone_level(1.0), 255);
        for level in 0..=255 {
            assert_eq!(sidetone_level(sidetone_ui(level)), level);
        }
    }
}
