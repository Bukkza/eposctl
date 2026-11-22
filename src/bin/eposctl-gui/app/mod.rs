//! The application: components wired together, the per-frame loop, and the
//! actions that span several components. Pages live in `pages`.

mod actions;
mod background;
mod pages;

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;

use eframe::egui::{self, Frame, Margin, Ui, UiBuilder, vec2};

use crate::autostart;
use crate::autoswitch::AutoSwitch;
use crate::battery;
use crate::catalog;
use crate::chatmix;
use crate::config::{Autosave, Config, Last};
use crate::devswitch;
use crate::editor::Editor;
use crate::hotkeys::{self, Action};
use crate::instance::ShowEvent;
use crate::meters::Meters;
use crate::mic_tools::MicTools;
use crate::strings::{Strings, t};
use crate::theme;
use crate::tray::{self, Tray};
use crate::widgets::Notice;
use crate::window::Window;
use crate::worker::{self, Cmd};

use pages::Page;

/// Which 9-band EQ an action is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqTarget {
    Speaker,
    Mic,
}

pub struct App {
    cfg: Config,
    autosave: Autosave,
    /// DSP settings in the EPOS driver, with undo and A/B.
    editor: Editor,
    /// Headset state from the HID thread, and commands to it.
    dev: worker::Shared,
    tx: Sender<Cmd>,
    flags: Arc<tray::Flags>,
    tray: Option<Tray>,
    brand: egui::TextureHandle,
    window: Window,
    view: View,
    hotkeys: Option<hotkeys::Hotkeys>,
    auto: AutoSwitch,
    devices: devswitch::Devices,
    follower: devswitch::Follower,
    battery: battery::Tracker,
    low_battery: battery::LowAlarm,
    mixer: chatmix::Mixer,
    meters: Meters,
    mic_tools: MicTools,
    /// The official EPOS Gaming Suite next to eposctl.
    official: crate::official::Official,
    autostart: bool,
    quitting: bool,
}

/// UI state that lives only as long as the window: what is open, being typed
/// or being reported.
#[derive(Default)]
struct View {
    page: Page,
    /// Name typed for a new preset; `Some` while the field is open.
    naming: Option<String>,
    /// Sidetone set from the UI, shown until the headset confirms it.
    sidetone_drag: Option<(f32, Instant)>,
    /// Program name typed into the "add rule" field.
    rule_input: String,
    /// Program name typed into the voice-apps field.
    voice_input: String,
    /// Hotkey being recorded; all hotkeys are unregistered meanwhile so the
    /// current ones can be pressed again.
    recording: Option<Action>,
    /// The parametric EQ panel is open.
    peq_open: bool,
    /// Equalizer APO is installed (checked when the panel opens).
    eqapo_installed: bool,
    preset_files_notice: Option<Notice>,
    eq_notice: Option<(EqTarget, Notice)>,
    peq_notice: Option<Notice>,
    official_notice: Option<Notice>,
}

/// Read a file chosen in a dialog and parse it; errors as text for a notice.
fn read_with<T>(path: &std::path::Path, parse: impl FnOnce(&str) -> Result<T, String>) -> Result<T, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| parse(&t))
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, show_event: ShowEvent) -> Self {
        let ctx = &cc.egui_ctx;
        theme::install_fonts(ctx);
        theme::install_style(ctx);
        let cfg = Config::load();
        cfg.appearance.apply(ctx);
        let wake = ctx.clone();
        let (dev, tx) = worker::spawn(move || wake.request_repaint());
        let flags = Arc::new(tray::Flags::default());
        show_event.watch(flags.clone(), ctx.clone());
        let mark = crate::icons::brand();
        let image = egui::ColorImage::from_rgba_unmultiplied([mark.width as usize, mark.height as usize], &mark.rgba);
        let mut app = Self {
            cfg,
            autosave: Autosave::default(),
            editor: Editor::new(),
            dev,
            tx,
            flags: flags.clone(),
            tray: None,
            brand: ctx.load_texture("eposctl logo", image, egui::TextureOptions::LINEAR),
            window: Window::new(std::env::args().any(|a| a == "--minimized")),
            view: View { eqapo_installed: eposctl::eqapo::config_dir().is_some(), ..View::default() },
            hotkeys: hotkeys::Hotkeys::spawn(ctx.clone()),
            auto: AutoSwitch::spawn(ctx.clone()),
            devices: devswitch::Devices::new(ctx.clone()),
            follower: devswitch::Follower::default(),
            battery: battery::Tracker::default(),
            low_battery: battery::LowAlarm::default(),
            mixer: chatmix::Mixer::spawn(),
            meters: Meters::default(),
            mic_tools: MicTools::default(),
            official: crate::official::Official::default(),
            autostart: autostart::is_enabled(),
            quitting: false,
        };
        #[cfg(debug_assertions)]
        app.apply_debug_env();
        app.tray = Tray::new(ctx, flags, app.s(), &app.tray_presets());
        app
    }

    #[cfg(debug_assertions)]
    fn apply_debug_env(&mut self) {
        if let Some(p) = crate::debug::start_page() {
            self.view.page = Page::from_name(&p);
            self.view.peq_open = p == "peq";
        }
        crate::debug::sample_toast(self.s());
    }

    fn s(&self) -> &'static Strings {
        t(self.cfg.lang)
    }

    fn mark_dirty(&mut self) {
        self.autosave.touch();
    }

    fn tray_presets(&self) -> Vec<(String, String)> {
        catalog::tray_entries(self.s(), &self.cfg.custom_presets)
    }

    /// Open the driver's channel if needed, restoring the last settings.
    fn connect(&mut self) {
        let restore = self.cfg.restore_on_start.then_some(&self.cfg.last);
        self.editor.connect(restore);
    }

    /// A user edit of the DSP settings, remembered for the next start.
    fn apply(&mut self, f: impl FnOnce(&mut eposctl::apo::Settings)) {
        if self.editor.apply(f) {
            self.remember();
        }
    }

    /// Save the current driver state as the settings restored on start.
    fn remember(&mut self) {
        if let Some(s) = self.editor.settings() {
            self.cfg.last = Last::of(s);
            self.mark_dirty();
        }
    }

    /// Stop everything that changes the system while we run, and save.
    fn shut_down(&mut self) {
        self.mic_tools.close();
        self.editor.stop_compare();
        self.mixer.shutdown();
        self.editor.set_metering(false);
        self.cfg.save();
    }
}

impl eframe::App for App {
    /// Runs even while the window is hidden, whenever something wakes the UI.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let d = self.dev.lock().unwrap().clone();
        #[cfg(debug_assertions)]
        crate::debug::screenshot(ui.ctx());

        // A drag (EQ point, slider) is one undo step however long it lasts.
        if ui.ctx().dragged_id().is_some() {
            self.editor.dragging();
        }
        self.record_hotkey(ui);
        self.undo_shortcuts(ui);

        let colors = theme::colors(ui);
        egui::Panel::left("nav")
            .exact_size(172.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(colors.sidebar).inner_margin(Margin::symmetric(12, 20)))
            .show(ui, |ui| self.nav(ui));

        let margin = Margin { left: 24, right: 24, top: 24, bottom: 0 };
        egui::CentralPanel::no_frame().frame(Frame::new().fill(colors.bg).inner_margin(margin)).show(ui, |ui| {
            let area = ui.available_rect_before_wrap();
            let pad = ((area.width() - theme::CONTENT_WIDTH) * 0.5).max(0.0);
            let content = area.shrink2(vec2(pad, 0.0));
            ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
                ui.set_width(content.width());
                self.header(ui, &d);
                ui.add_space(theme::SECTION_GAP);
                self.page(ui, &d);
            });
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.shut_down();
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        theme::palette(visuals.dark_mode).bg.to_normalized_gamma_f32()
    }
}
