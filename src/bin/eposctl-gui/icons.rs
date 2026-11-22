//! Line icons, the eposctl logo, and the tray battery indicator.

use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use eframe::egui::{Color32, IconData, Painter, Pos2, Rect, Stroke, vec2};

pub fn brand() -> &'static IconData {
    static BRAND: OnceLock<IconData> = OnceLock::new();
    BRAND.get_or_init(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/logo.png"))
            .expect("the embedded eposctl logo must be a valid PNG")
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Sound,
    Mic,
    Device,
    Settings,
}

fn arc(p: &Painter, c: Pos2, r: f32, a0: f32, a1: f32, s: Stroke) {
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    p.line(pts, s);
}

/// Draw `icon` centred in `rect` (designed on a 20×20 grid).
pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let k = rect.width().min(rect.height()) / 20.0;
    let o = rect.center() - vec2(10.0, 10.0) * k;
    let at = |x: f32, y: f32| o + vec2(x, y) * k;
    let s = Stroke::new(1.6 * k.max(1.0), color);
    match icon {
        Icon::Sound => {
            // Three mixer faders.
            for (x, knob) in [(4.0, 12.5), (10.0, 6.5), (16.0, 10.5)] {
                p.line_segment([at(x, 3.0), at(x, 17.0)], s);
                p.circle_filled(at(x, knob), 2.4 * k, color);
            }
        }
        Icon::Mic => {
            p.rect_stroke(
                Rect::from_min_max(at(7.0, 2.0), at(13.0, 12.0)),
                3.0 * k,
                s,
                eframe::egui::StrokeKind::Middle,
            );
            arc(p, at(10.0, 9.0), 6.0 * k, 0.0, PI, s);
            p.line_segment([at(10.0, 15.0), at(10.0, 18.0)], s);
            p.line_segment([at(7.0, 18.0), at(13.0, 18.0)], s);
        }
        Icon::Device => {
            // Headset: band and two cups.
            arc(p, at(10.0, 11.0), 7.0 * k, PI, TAU, s);
            for x in [2.0, 14.0] {
                p.rect_stroke(
                    Rect::from_min_size(at(x, 10.5), vec2(4.0, 7.0) * k),
                    1.5 * k,
                    s,
                    eframe::egui::StrokeKind::Middle,
                );
            }
        }
        Icon::Settings => {
            // Gear: ring with eight teeth.
            let c = at(10.0, 10.0);
            p.circle_stroke(c, 5.0 * k, s);
            p.circle_stroke(c, 2.0 * k, s);
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 5.0 * k, c + d * 8.0 * k], Stroke::new(2.4 * k.max(1.0), color));
            }
        }
    }
}

/// Tray bitmap with the logo inside the battery ring. An unknown level fills the ring.
pub fn tray_bitmap(size: u32, battery: Option<u8>, connected: bool) -> Vec<u8> {
    let n = size as f32;
    let ring = Ring {
        centre: n / 2.0,
        outer: n * 0.48,
        inner: n * 0.36,
        connected,
        accent: if connected { [0x68, 0x92, 0xFA] } else { [0x8E, 0x90, 0x99] },
        dim: [0x3A, 0x3D, 0x45],
        level: battery.map(|b| b as f32 / 100.0).unwrap_or(1.0),
    };
    (0..size).flat_map(|y| (0..size).map(move |x| (x, y))).flat_map(|(x, y)| ring.pixel(x, y)).collect()
}

/// The icon's shapes and colours at one size.
struct Ring {
    centre: f32,
    outer: f32,
    inner: f32,
    connected: bool,
    accent: [u8; 3],
    dim: [u8; 3],
    /// Bright part of the ring, 0..=1 clockwise from 12 o'clock.
    level: f32,
}

impl Ring {
    /// 4×4 supersampling for smooth edges at 16 px.
    const SS: u32 = 4;

    /// RGBA of pixel (`x`, `y`): the average colour of the covered samples,
    /// with their coverage as alpha.
    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let offsets = (0..Self::SS).map(|k| (k as f32 + 0.5) / Self::SS as f32);
        let samples = offsets.clone().flat_map(|oy| offsets.clone().map(move |ox| (ox, oy)));
        let mut acc = [0f32; 4];
        for (ox, oy) in samples {
            if let Some(rgb) = self.sample(x as f32 + ox - self.centre, y as f32 + oy - self.centre) {
                for (a, c) in acc.iter_mut().zip(rgb) {
                    *a += c as f32;
                }
                acc[3] += 1.0;
            }
        }
        if acc[3] == 0.0 {
            return [0; 4];
        }
        let coverage = acc[3] / (Self::SS * Self::SS) as f32;
        [(acc[0] / acc[3]) as u8, (acc[1] / acc[3]) as u8, (acc[2] / acc[3]) as u8, (coverage * 255.0) as u8]
    }

    /// Colour at offset (`fx`, `fy`) from the centre; `None` is transparent.
    fn sample(&self, fx: f32, fy: f32) -> Option<[u8; 3]> {
        let d = (fx * fx + fy * fy).sqrt();
        if (self.inner..=self.outer).contains(&d) {
            // Angle clockwise from 12 o'clock, 0..1.
            let a = (fx.atan2(-fy) / TAU).rem_euclid(1.0);
            return Some(if a <= self.level { self.accent } else { self.dim });
        }
        // Fit the waveforms inside the ring with a gap between the two shapes.
        let width = self.inner * 1.85;
        let (x, y) = (fx / width + 0.5, fy / width + 0.5);
        if !(0.0..1.0).contains(&x) || !(0.0..1.0).contains(&y) {
            return None;
        }
        let logo = brand();
        let x = (x * logo.width as f32) as usize;
        let y = (y * logo.height as f32) as usize;
        let offset = 4 * (y * logo.width as usize + x);
        let &[r, g, b, a] = logo.rgba.get(offset..offset + 4)? else { return None };
        (a >= 128).then_some(if self.connected { [r, g, b] } else { self.accent })
    }
}
