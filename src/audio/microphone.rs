//! Recording-endpoint volume and playback of a captured mono clip.

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
    IAudioClient, IAudioRenderClient, WAVEFORMATEX, WAVEFORMATEXTENSIBLE, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoTaskMemFree};
use windows::core::{Error, GUID, HSTRING, Result};

use super::enumerator;

const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xfffe;
const IEEE_FLOAT_SUBTYPE: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
const BUFFER_DURATION: i64 = 1_000_000; // 100 ms in WASAPI's 100 ns units.

pub struct InputVolume(IAudioEndpointVolume);

#[derive(Clone, Copy)]
pub struct InputGain {
    pub level_db: f32,
    pub min_db: f32,
    pub max_db: f32,
    pub step_db: f32,
}

impl InputVolume {
    pub fn open(id: &str) -> Result<Self> {
        let en = enumerator()?;
        // SAFETY: `id` is a Windows audio endpoint ID. COM is initialized by `enumerator`.
        unsafe { en.GetDevice(&HSTRING::from(id))?.Activate(CLSCTX_ALL, None).map(Self) }
    }

    pub fn level(&self) -> Result<f32> {
        // SAFETY: the interface was activated for this endpoint on the calling thread.
        unsafe { self.0.GetMasterVolumeLevelScalar() }
    }

    pub fn set(&self, level: f32) -> Result<()> {
        // SAFETY: the scalar is in range and the optional event context is null.
        unsafe { self.0.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null()) }
    }

    pub fn gain(&self) -> Result<InputGain> {
        let (mut min_db, mut max_db, mut step_db) = (0.0, 0.0, 0.0);
        // SAFETY: the output pointers refer to writable floats on this thread.
        let level_db = unsafe {
            self.0.GetVolumeRange(&mut min_db, &mut max_db, &mut step_db)?;
            self.0.GetMasterVolumeLevel()?
        };
        if ![level_db, min_db, max_db, step_db].into_iter().all(f32::is_finite) || min_db > max_db || step_db < 0.0 {
            return Err(Error::new(E_FAIL, "Windows reported an invalid microphone gain range"));
        }
        Ok(InputGain { level_db, min_db, max_db, step_db })
    }

    pub fn set_gain(&self, level_db: f32) -> Result<()> {
        // SAFETY: this endpoint was activated on the calling thread. WASAPI
        // checks the requested level against the current endpoint's range.
        unsafe { self.0.SetMasterVolumeLevel(level_db, std::ptr::null()) }
    }
}

pub(super) fn initialize_capture(client: &IAudioClient) -> Result<(usize, u32)> {
    // SAFETY: the allocated mix format stays valid through Initialize. The
    // extension is read only when the tag and extension length both match.
    unsafe {
        let format = client.GetMixFormat()?;
        let mix = *format;
        let channels = usize::from(mix.nChannels);
        let rate = mix.nSamplesPerSec;
        let extended_float = if mix.wFormatTag == WAVE_FORMAT_EXTENSIBLE && mix.cbSize >= 22 {
            let subtype = (*format.cast::<WAVEFORMATEXTENSIBLE>()).SubFormat;
            subtype == IEEE_FLOAT_SUBTYPE
        } else {
            false
        };
        let is_float = mix.wBitsPerSample == 32 && (mix.wFormatTag == WAVE_FORMAT_IEEE_FLOAT || extended_float);
        let result = if is_float && channels != 0 {
            client.Initialize(AUDCLNT_SHAREMODE_SHARED, 0, BUFFER_DURATION, 0, format, None).map(|()| (channels, rate))
        } else {
            initialize_mono(client, rate).map(|()| (1, rate))
        };
        CoTaskMemFree(Some(format.cast()));
        result
    }
}

fn initialize_mono(client: &IAudioClient, rate: u32) -> Result<()> {
    let Some(bytes_per_second) = rate.checked_mul(4).filter(|&n| n > 0) else {
        return Err(Error::new(E_INVALIDARG, "invalid audio sample rate"));
    };
    let format = WAVEFORMATEX {
        wFormatTag: WAVE_FORMAT_IEEE_FLOAT,
        nChannels: 1,
        nSamplesPerSec: rate,
        nAvgBytesPerSec: bytes_per_second,
        nBlockAlign: 4,
        wBitsPerSample: 32,
        cbSize: 0,
    };
    let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
    // SAFETY: the stack format describes the mono f32 frames exactly. WASAPI
    // converts to/from the endpoint mix format in shared mode.
    unsafe { client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, BUFFER_DURATION, 0, &format, None) }
}

/// Play one mono float clip on the Windows default playback device. Blocks on
/// the calling thread until the buffer drains or `keep_going` requests a stop.
pub fn play_mono(samples: &[f32], rate: u32, mut keep_going: impl FnMut() -> bool) -> Result<()> {
    if samples.is_empty() || !keep_going() {
        return Ok(());
    }
    let en = enumerator()?;
    // SAFETY: the default render device and its services are used on this COM thread.
    unsafe {
        let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = dev.Activate(CLSCTX_ALL, None)?;
        initialize_mono(&client, rate)?;
        let size = client.GetBufferSize()?;
        let render: IAudioRenderClient = client.GetService()?;
        let first = (size as usize).min(samples.len());
        enqueue(&render, &samples[..first])?;
        if !keep_going() {
            return Ok(());
        }
        client.Start()?;
        let deadline = Instant::now() + Duration::from_secs_f64(samples.len() as f64 / f64::from(rate) + 2.0);
        let result = drain(&client, &render, size, &samples[first..], deadline, keep_going);
        result.and(client.Stop())
    }
}

fn drain(
    client: &IAudioClient,
    render: &IAudioRenderClient,
    size: u32,
    mut samples: &[f32],
    deadline: Instant,
    mut keep_going: impl FnMut() -> bool,
) -> Result<()> {
    while keep_going() {
        // SAFETY: the client was initialized and started on this thread.
        let pending = unsafe { client.GetCurrentPadding()? };
        if samples.is_empty() && pending == 0 {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "audio playback did not finish"));
        }
        let count = (size.saturating_sub(pending) as usize).min(samples.len());
        enqueue(render, &samples[..count])?;
        samples = &samples[count..];
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn enqueue(render: &IAudioRenderClient, samples: &[f32]) -> Result<()> {
    if samples.is_empty() {
        return Ok(());
    }
    // SAFETY: the caller limits the packet to the available render frames. Each
    // mono frame is one f32, and the WASAPI buffer is valid until ReleaseBuffer.
    unsafe {
        let frames = samples.len() as u32;
        let dest = render.GetBuffer(frames)?;
        std::ptr::copy_nonoverlapping(samples.as_ptr().cast(), dest, std::mem::size_of_val(samples));
        render.ReleaseBuffer(frames, 0)
    }
}
