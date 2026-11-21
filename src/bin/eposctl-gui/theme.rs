//! Shared typography, spacing and colors for the desktop UI.

use std::collections::BTreeMap;
use std::sync::Arc;

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Ui};
use serde::{Deserialize, Serialize};

pub const GAP: f32 = 16.0;
pub const SECTION_GAP: f32 = 24.0;
pub const CONTROL_HEIGHT: f32 = 44.0;
pub const CONTENT_WIDTH: f32 = 744.0;
pub const R_SMALL: u8 = 10;
pub const R_CARD: u8 = 18;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub fn apply(self, ctx: &egui::Context) {
        ctx.set_theme(match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        });
    }
}

pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub panel: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub line: Color32,
    pub grid: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub subdued: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_dim: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warn: Color32,
    pub danger: Color32,
    pub shadow: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const LIGHT: Palette = Palette {
    bg: rgb(0xF4F6FA),
    sidebar: rgb(0xEAF0F8),
    panel: rgb(0xFFFFFF),
    raised: rgb(0xEDF2F9),
    hover: rgb(0xE1EAF7),
    line: rgb(0xD8E2EF),
    grid: rgb(0xE9EFF7),
    text: rgb(0x152339),
    secondary: rgb(0x46566F),
    subdued: rgb(0x586780),
    accent: rgb(0x2254C9),
    accent_hover: rgb(0x1948B7),
    accent_dim: rgb(0xE5EDFF),
    on_accent: rgb(0xFFFFFF),
    success: rgb(0x187050),
    warn: rgb(0x8A4A17),
    danger: rgb(0xB12B3F),
    shadow: Color32::from_black_alpha(12),
};

const DARK: Palette = Palette {
    bg: rgb(0x111622),
    sidebar: rgb(0x151C2B),
    panel: rgb(0x1C2535),
    raised: rgb(0x263349),
    hover: rgb(0x303F58),
    line: rgb(0x34435B),
    grid: rgb(0x2A374C),
    text: rgb(0xF2F6FD),
    secondary: rgb(0xBBC9DE),
    subdued: rgb(0xA3B3CC),
    accent: rgb(0x98B9FF),
    accent_hover: rgb(0xB0CAFF),
    accent_dim: rgb(0x283E67),
    on_accent: rgb(0x102248),
    success: rgb(0x75D8AF),
    warn: rgb(0xF5C17E),
    danger: rgb(0xFFA2AF),
    shadow: Color32::from_black_alpha(32),
};

pub fn palette(dark: bool) -> &'static Palette {
    if dark { &DARK } else { &LIGHT }
}

pub fn colors(ui: &Ui) -> &'static Palette {
    palette(ui.visuals().dark_mode)
}

/// Semibold face for headings and numbers.
pub fn strong() -> FontFamily {
    FontFamily::Name("strong".into())
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn font_strong(size: f32) -> FontId {
    FontId::new(size, strong())
}

fn load(defs: &mut FontDefinitions, key: &str, file: &str, index: u32) -> bool {
    let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| r"C:\Windows".into());
    match std::fs::read(dir.join("Fonts").join(file)) {
        Ok(bytes) => {
            let mut fd = FontData::from_owned(bytes);
            fd.index = index;
            defs.font_data.insert(key.to_owned(), Arc::new(fd));
            true
        }
        Err(_) => false,
    }
}

/// Use a system display face with Microsoft YaHei for CJK fallback.
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    let mut regular = Vec::new();
    let mut bold = Vec::new();
    for (key, file, is_bold) in [
        ("sf-pro", "SF-Pro-Display-Regular.otf", false),
        ("segoe-variable", "SegUIVar.ttf", false),
        ("segoe", "segoeui.ttf", false),
        ("yahei", "msyh.ttc", false),
        ("sf-pro-semibold", "SF-Pro-Display-Semibold.otf", true),
        ("segoe-sb", "seguisb.ttf", true),
        ("yahei-b", "msyhbd.ttc", true),
    ] {
        if load(&mut defs, key, file, 0) {
            if is_bold { &mut bold } else { &mut regular }.push(key.to_owned());
        }
    }
    let builtin = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut prop = regular.clone();
    prop.extend(builtin.iter().cloned());
    let mut strong_list = bold;
    strong_list.extend(regular);
    strong_list.extend(builtin);
    let mut families = BTreeMap::new();
    families.insert(FontFamily::Proportional, prop);
    families.insert(strong(), strong_list);
    if let Some(mono) = defs.families.get(&FontFamily::Monospace) {
        families.insert(FontFamily::Monospace, mono.clone());
    }
    defs.families = families;
    ctx.set_fonts(defs);
}

fn visuals(theme: egui::Theme) -> egui::Visuals {
    let mut v = match theme {
        egui::Theme::Light => egui::Visuals::light(),
        egui::Theme::Dark => egui::Visuals::dark(),
    };
    let c = palette(v.dark_mode);
    v.window_fill = c.panel;
    v.panel_fill = c.bg;
    v.extreme_bg_color = c.raised;
    v.faint_bg_color = c.raised;
    v.window_corner_radius = CornerRadius::same(R_CARD);
    v.window_stroke = Stroke::new(1.0, c.line);
    v.selection.bg_fill = c.accent_dim;
    v.selection.stroke = Stroke::new(1.0, c.accent);
    v.hyperlink_color = c.accent;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;

    let w = &mut v.widgets;
    let r = CornerRadius::same(R_SMALL);
    w.noninteractive.bg_fill = c.panel;
    w.noninteractive.weak_bg_fill = c.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, c.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = c.raised;
    w.inactive.weak_bg_fill = c.raised;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, c.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = c.hover;
    w.hovered.weak_bg_fill = c.hover;
    w.hovered.bg_stroke = Stroke::new(1.0, c.line);
    w.hovered.fg_stroke = Stroke::new(1.5, c.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = c.accent_dim;
    w.active.weak_bg_fill = c.accent_dim;
    w.active.bg_stroke = Stroke::new(1.0, c.accent);
    w.active.fg_stroke = Stroke::new(1.5, c.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open = w.active;
    v
}

pub fn install_style(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.set_visuals_of(theme, visuals(theme));
    }
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, font(13.0)),
            (TextStyle::Body, font(15.0)),
            (TextStyle::Button, font(14.0)),
            (TextStyle::Heading, font_strong(19.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.spacing.slider_rail_height = 4.0;
        s.spacing.interact_size.y = CONTROL_HEIGHT;
        s.animation_time = 0.14;
        s.interaction.selectable_labels = false;
    });
}
