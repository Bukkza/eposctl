//! Microphone monitoring and short, in-memory test recordings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use eposctl::audio::microphone::{InputVolume, play_mono};
use eposctl::audio::{self, Endpoint, Flow};

use crate::mic_calibration::{self, Calibration};

pub const RECORD_SECONDS: usize = 15;
const CLIP_LEVEL: f32 = 0.99;
const CLIP_HOLD: Duration = Duration::from_millis(1200);
const REFRESH: Duration = Duration::from_secs(1);

#[derive(Default)]
pub struct MicTools {
    input: Option<Input>,
    next_refresh: Option<Instant>,
    clip: Option<Arc<Clip>>,
    playback: Option<Playback>,
    error: Option<String>,
}

#[derive(Default)]
pub struct MicStatus {
    pub name: Option<String>,
    pub volume: Option<f32>,
    pub peak: Option<f32>,
    pub clipping: bool,
    pub calibration: mic_calibration::Status,
    pub recording_for: Option<Duration>,
    pub has_clip: bool,
    pub playing: bool,
    pub error: Option<String>,
}

impl MicTools {
    pub fn tick(&mut self, ctx: &egui::Context, visible: bool) {
        if !visible {
            self.close();
            return;
        }
        let now = Instant::now();
        if self.next_refresh.is_none_or(|next| next <= now) {
            self.next_refresh = Some(now + REFRESH);
            self.refresh();
        }
        if let Some(capture) = self.input.as_ref().and_then(|input| input.capture.as_ref()) {
            let mut signal = capture.signal.lock().unwrap();
            signal.calibration.finish(now);
            if signal.take.as_ref().is_some_and(|take| take.since.elapsed().as_secs() >= RECORD_SECONDS as u64) {
                signal.finish();
            }
            if let Some(clip) = signal.completed.take() {
                self.clip = Some(Arc::new(clip));
            }
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        self.poll_playback();
    }

    fn refresh(&mut self) {
        let endpoint = match audio::epos_endpoint(Flow::Input) {
            Ok(ep) => ep,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        if self.input.as_ref().map(|i| &i.endpoint) != endpoint.as_ref() {
            self.release_input();
            self.error = None;
            self.input = endpoint.map(Input::open);
        }
        if let Some(input) = &mut self.input {
            input.refresh_volume();
            input.check_calibration();
        }
    }

    pub fn status(&self) -> MicStatus {
        let mut status = MicStatus {
            has_clip: self.clip.is_some(),
            playing: self.playback.is_some(),
            error: self.error.clone(),
            ..Default::default()
        };
        if let Some(input) = &self.input {
            status.name = Some(input.endpoint.name.clone());
            status.volume = input.level;
            status.error = status.error.or_else(|| input.error.clone());
            if let Some(capture) = &input.capture {
                let signal = capture.signal.lock().unwrap();
                status.peak = signal.peak;
                status.clipping = signal.clipped_at.is_some_and(|t| t.elapsed() < CLIP_HOLD);
                status.calibration = signal.calibration.status(Instant::now());
                status.recording_for = signal.take.as_ref().map(|take| take.since.elapsed());
                status.error = status.error.or_else(|| signal.error.clone());
            }
        }
        status
    }

    pub fn set_volume(&mut self, level: f32) {
        let Some(input) = &mut self.input else { return };
        let Some(volume) = &input.volume else { return };
        match volume.set(level) {
            Ok(()) => {
                input.level = Some(level);
                self.error = None;
                if let Some(capture) = &input.capture {
                    capture.signal.lock().unwrap().calibration = Calibration::Idle;
                }
            }
            Err(e) => {
                input.volume = None;
                input.level = None;
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn calibrate(&mut self, on: bool) {
        let Some(input) = &self.input else { return };
        let Some(capture) = &input.capture else { return };
        if !on {
            capture.signal.lock().unwrap().calibration = Calibration::Idle;
            return;
        }
        let Some(volume) = &input.volume else { return };
        let gain = match volume.gain() {
            Ok(gain) => gain,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        let mut signal = capture.signal.lock().unwrap();
        if signal.peak.is_some() && signal.take.is_none() && self.playback.is_none() {
            self.error = None;
            signal.calibration = Calibration::start(gain, Instant::now());
        }
    }

    pub fn apply_calibration(&mut self) {
        use mic_calibration::Result::{Adjust, Applied, GainChanged};

        let Some(input) = &mut self.input else { return };
        let (Some(capture), Some(volume)) = (&input.capture, &input.volume) else { return };
        let Calibration::Done(Adjust(r)) = capture.signal.lock().unwrap().calibration else { return };
        let result = volume.gain().and_then(|current| {
            if !mic_calibration::same_gain(r.before, current) {
                return Ok(GainChanged);
            }
            volume.set_gain(r.after_db)?;
            // Show the change Windows actually applied after quantization.
            Ok(Applied(volume.gain()?.level_db - current.level_db))
        });
        input.level = volume.level().ok();
        match result {
            Ok(result) => {
                capture.signal.lock().unwrap().calibration = Calibration::Done(result);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    pub fn record(&mut self, on: bool) {
        let Some(capture) = self.input.as_ref().and_then(|input| input.capture.as_ref()) else { return };
        let mut signal = capture.signal.lock().unwrap();
        if !on {
            signal.finish();
        } else if signal.peak.is_some()
            && signal.take.is_none()
            && self.playback.is_none()
            && !matches!(signal.calibration, Calibration::Measuring(_))
        {
            self.error = None;
            signal.take = Some(Take { since: Instant::now(), clip: Clip::default() });
        }
    }

    pub fn play(&mut self, on: bool, ctx: &egui::Context) {
        if !on {
            self.playback = None;
            return;
        }
        if self.status().calibration.measuring() {
            return;
        }
        let Some(clip) = self.clip.clone() else { return };
        match Playback::start(clip, ctx.clone()) {
            Ok(playback) => {
                self.playback = Some(playback);
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn poll_playback(&mut self) {
        let Some(playback) = &self.playback else { return };
        match playback.result.try_recv() {
            Err(TryRecvError::Empty) => return,
            Ok(result) => self.error = result.err(),
            Err(TryRecvError::Disconnected) => {}
        }
        self.playback = None;
    }

    pub fn close(&mut self) {
        self.release_input();
        self.playback = None;
        self.next_refresh = None;
        self.error = None;
    }

    fn release_input(&mut self) {
        let Some(input) = self.input.take() else { return };
        let Some(capture) = input.capture else { return };
        let mut signal = capture.signal.lock().unwrap();
        signal.finish();
        if let Some(clip) = signal.completed.take() {
            self.clip = Some(Arc::new(clip));
        }
    }
}

struct Input {
    endpoint: Endpoint,
    volume: Option<InputVolume>,
    level: Option<f32>,
    capture: Option<Capture>,
    error: Option<String>,
}

impl Input {
    fn open(endpoint: Endpoint) -> Self {
        let capture = Capture::start(endpoint.id.clone());
        let error = capture.as_ref().err().map(ToString::to_string);
        Self { endpoint, volume: None, level: None, capture: capture.ok(), error }
    }

    fn refresh_volume(&mut self) {
        self.volume = self.volume.take().or_else(|| InputVolume::open(&self.endpoint.id).ok());
        self.level = self.volume.as_ref().and_then(|v| v.level().ok());
        if self.level.is_none() {
            self.volume = None;
        }
    }

    fn check_calibration(&self) {
        let (Some(capture), Some(volume)) = (&self.capture, &self.volume) else { return };
        let Some(before) = capture.signal.lock().unwrap().calibration.baseline() else { return };
        if let Ok(current) = volume.gain()
            && !mic_calibration::same_gain(before, current)
        {
            capture.signal.lock().unwrap().calibration = Calibration::Done(mic_calibration::Result::GainChanged);
        }
    }
}

#[derive(Default)]
struct Clip {
    samples: Vec<f32>,
    rate: u32,
}

struct Take {
    clip: Clip,
    since: Instant,
}

#[derive(Default)]
struct Signal {
    peak: Option<f32>,
    clipped_at: Option<Instant>,
    calibration: Calibration,
    take: Option<Take>,
    completed: Option<Clip>,
    error: Option<String>,
}

impl Signal {
    fn frames(&mut self, samples: &[f32], rate: u32, now: Instant) {
        let peak = samples.iter().map(|s| s.abs()).filter(|v| v.is_finite()).fold(0.0, f32::max);
        self.peak = Some(peak.max(self.peak.unwrap_or_default() * 0.9));
        if peak >= CLIP_LEVEL {
            self.clipped_at = Some(now);
        }
        self.calibration.sample(samples, rate, peak, now);
        let Some(take) = &mut self.take else { return };
        let clip = &mut take.clip;
        clip.rate = rate;
        let limit = rate as usize * RECORD_SECONDS;
        let left = limit.saturating_sub(clip.samples.len());
        clip.samples.extend(samples.iter().take(left).map(|s| if s.is_finite() { s.clamp(-1.0, 1.0) } else { 0.0 }));
        if clip.samples.len() >= limit {
            self.finish();
        }
    }

    fn finish(&mut self) {
        if let Some(take) = self.take.take()
            && !take.clip.samples.is_empty()
        {
            self.completed = Some(take.clip);
        }
    }
}

#[derive(Default)]
struct Stop(Arc<AtomicBool>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

struct Capture {
    _stop: Stop,
    signal: Arc<Mutex<Signal>>,
}

impl Capture {
    fn start(id: String) -> std::io::Result<Self> {
        let stop = Stop::default();
        let flag = stop.0.clone();
        let signal = Arc::new(Mutex::new(Signal::default()));
        let shared = signal.clone();
        std::thread::Builder::new().name("eposctl-mic-capture".into()).spawn(move || {
            let running = || !flag.load(Ordering::Relaxed);
            let result = audio::capture(&id, running, |samples, rate| {
                let mut signal = shared.lock().unwrap();
                if running() {
                    signal.frames(samples, rate, Instant::now());
                }
            });
            if let Err(e) = result {
                let mut signal = shared.lock().unwrap();
                signal.finish();
                signal.peak = None;
                signal.clipped_at = None;
                signal.calibration = Calibration::Idle;
                signal.error = Some(e.to_string());
            }
        })?;
        Ok(Self { _stop: stop, signal })
    }
}

struct Playback {
    _stop: Stop,
    result: Receiver<Result<(), String>>,
}

impl Playback {
    fn start(clip: Arc<Clip>, ctx: egui::Context) -> std::io::Result<Self> {
        let stop = Stop::default();
        let flag = stop.0.clone();
        let (tx, result) = mpsc::channel();
        std::thread::Builder::new().name("eposctl-mic-playback".into()).spawn(move || {
            let status = play_mono(&clip.samples, clip.rate, || !flag.load(Ordering::Relaxed));
            let _ = tx.send(status.map_err(|e| e.to_string()));
            ctx.request_repaint();
        })?;
        Ok(Self { _stop: stop, result })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitoring_detects_near_full_scale_without_saving_audio() {
        let mut signal = Signal::default();
        let now = Instant::now();
        signal.frames(&[0.7, -0.8], 48_000, now);
        assert!(signal.clipped_at.is_none());
        signal.frames(&[0.7, -0.995], 48_000, now);
        assert_eq!(signal.clipped_at, Some(now));
        assert!(signal.take.is_none() && signal.completed.is_none());
    }

    #[test]
    fn a_recording_stops_at_the_limit_and_keeps_only_armed_samples() {
        let mut signal = Signal::default();
        let now = Instant::now();
        signal.frames(&[0.1], 2, now);
        signal.take = Some(Take { since: now, clip: Clip::default() });
        signal.frames(&[0.2; RECORD_SECONDS * 2 + 1], 2, now);
        signal.frames(&[0.3], 2, now);
        assert!(signal.take.is_none());
        let clip = signal.completed.unwrap();
        assert_eq!(clip.rate, 2);
        assert_eq!(clip.samples, vec![0.2; RECORD_SECONDS * 2]);
    }
}
