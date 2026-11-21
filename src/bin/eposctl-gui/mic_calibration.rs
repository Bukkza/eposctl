//! Estimate an input gain from a short measurement. Only the peak and the
//! accumulated time above the speech threshold are kept.

use std::time::{Duration, Instant};

use eposctl::audio::microphone::InputGain;

pub const SECONDS: u64 = 6;
const START_DELAY: Duration = Duration::from_millis(500);
const ACTIVE_LEVEL: f32 = 0.006;
const MIN_ACTIVE_SECONDS: f32 = 0.09;
const TARGET_DBFS: f32 = -12.0;
const GOOD_PEAK: std::ops::RangeInclusive<f32> = -18.0..=-9.0;
const MAX_CUT_DB: f32 = 18.0;
const MAX_BOOST_DB: f32 = 12.0;

#[derive(Clone, Copy)]
pub struct Recommendation {
    pub peak_dbfs: f32,
    pub before: InputGain,
    pub after_db: f32,
}

impl Recommendation {
    pub fn change_db(self) -> f32 {
        self.after_db - self.before.level_db
    }
}

#[derive(Clone, Copy)]
pub enum Result {
    InsufficientSpeech,
    WithinTarget(f32),
    AtLimit(f32),
    GainChanged,
    Adjust(Recommendation),
    Applied(f32),
}

#[derive(Clone, Copy, Default)]
pub enum Status {
    #[default]
    Idle,
    Measuring(Duration),
    Done(Result),
}

impl Status {
    pub fn measuring(self) -> bool {
        matches!(self, Self::Measuring(_))
    }
}

#[derive(Default)]
pub enum Calibration {
    #[default]
    Idle,
    Measuring(Measurement),
    Done(Result),
}

impl Calibration {
    pub fn start(gain: InputGain, now: Instant) -> Self {
        Self::Measuring(Measurement { gain, start: now, peak: 0.0, active_seconds: 0.0 })
    }

    pub fn status(&self, now: Instant) -> Status {
        match self {
            Self::Idle => Status::Idle,
            Self::Measuring(m) => Status::Measuring(Duration::from_secs(SECONDS).saturating_sub(now - m.start)),
            Self::Done(result) => Status::Done(*result),
        }
    }

    pub fn sample(&mut self, samples: &[f32], rate: u32, peak: f32, now: Instant) {
        let Self::Measuring(m) = self else { return };
        let elapsed = now - m.start;
        if !(START_DELAY..Duration::from_secs(SECONDS)).contains(&elapsed) || rate == 0 {
            return;
        }
        m.peak = m.peak.max(peak);
        let active = samples.iter().filter(|s| s.is_finite() && s.abs() >= ACTIVE_LEVEL).count();
        m.active_seconds += active as f32 / rate as f32;
    }

    pub fn finish(&mut self, now: Instant) {
        if let Self::Measuring(m) = self
            && now - m.start >= Duration::from_secs(SECONDS)
        {
            *self = Self::Done(m.result());
        }
    }

    pub fn baseline(&self) -> Option<InputGain> {
        match self {
            Self::Measuring(m) => Some(m.gain),
            Self::Done(Result::Adjust(r)) => Some(r.before),
            _ => None,
        }
    }
}

pub struct Measurement {
    gain: InputGain,
    start: Instant,
    peak: f32,
    active_seconds: f32,
}

impl Measurement {
    fn result(&self) -> Result {
        if self.active_seconds < MIN_ACTIVE_SECONDS || self.peak <= 0.0 {
            return Result::InsufficientSpeech;
        }
        let peak_dbfs = 20.0 * self.peak.log10();
        if GOOD_PEAK.contains(&peak_dbfs) {
            return Result::WithinTarget(peak_dbfs);
        }
        let change = (TARGET_DBFS - peak_dbfs).clamp(-MAX_CUT_DB, MAX_BOOST_DB);
        let g = self.gain;
        let after_db = quantize(g.level_db + change, g);
        if (after_db - g.level_db).abs() < 0.05 {
            Result::AtLimit(peak_dbfs)
        } else {
            Result::Adjust(Recommendation { peak_dbfs, before: g, after_db })
        }
    }
}

fn quantize(level_db: f32, g: InputGain) -> f32 {
    let level_db = level_db.clamp(g.min_db, g.max_db);
    if g.step_db == 0.0 {
        level_db
    } else {
        let steps = ((level_db - g.min_db) / g.step_db).round();
        (g.min_db + steps * g.step_db).clamp(g.min_db, g.max_db)
    }
}

pub fn same_gain(a: InputGain, b: InputGain) -> bool {
    (a.level_db - b.level_db).abs() < 0.05 && a.min_db == b.min_db && a.max_db == b.max_db && a.step_db == b.step_db
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(peak: f32, gain: InputGain) -> Result {
        let start = Instant::now();
        let mut calibration = Calibration::start(gain, start);
        calibration.sample(&[1.0; 500], 1000, 1.0, start); // Ignore the button click.
        calibration.sample(&[peak; 500], 1000, peak, start + START_DELAY);
        calibration.finish(start + Duration::from_secs(SECONDS));
        let Calibration::Done(result) = calibration else { panic!("missing result") };
        result
    }

    #[test]
    fn recommends_cut_for_clipping_with_real_endpoint_steps() {
        let gain = InputGain { level_db: 0.0, min_db: -30.0, max_db: 2.0, step_db: 2.0 };
        let Result::Adjust(r) = measure(1.0, gain) else { panic!("expected a cut") };
        assert_eq!(r.after_db, -12.0);
    }

    #[test]
    fn keeps_good_gain_and_does_not_boost_silence() {
        let gain = InputGain { level_db: -3.0, min_db: -30.0, max_db: 3.0, step_db: 1.0 };
        assert!(matches!(measure(0.2, gain), Result::WithinTarget(_)));
        assert!(matches!(measure(0.0, gain), Result::InsufficientSpeech));
        let Result::Adjust(r) = measure(0.02, gain) else { panic!("expected a boost") };
        assert_eq!(r.after_db, 3.0);
        assert!(matches!(measure(0.02, InputGain { level_db: 3.0, ..gain }), Result::AtLimit(_)));
    }
}
