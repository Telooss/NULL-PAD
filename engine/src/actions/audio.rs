use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Output,
    Mic,
}

/// New level after applying `delta` percent points, kept in 0.0..=1.0 (2 decimals).
pub fn clamp_level(current: f32, delta: i8) -> f32 {
    let v = (current + f32::from(delta) / 100.0).clamp(0.0, 1.0);
    (v * 100.0).round() / 100.0
}

fn volume(ep: Endpoint) -> Result<IAudioEndpointVolume, String> {
    let e = |e: windows::core::Error| e.to_string();
    unsafe {
        // S_FALSE / RPC_E_CHANGED_MODE just mean COM is already up on this thread.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(e)?;
        let flow = if ep == Endpoint::Output {
            eRender
        } else {
            eCapture
        };
        let device = enumerator
            .GetDefaultAudioEndpoint(flow, eConsole)
            .map_err(e)?;
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(e)
    }
}

pub fn change_volume(ep: Endpoint, delta: i8) -> Result<f32, String> {
    let v = volume(ep)?;
    unsafe {
        let level = clamp_level(
            v.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?,
            delta,
        );
        v.SetMasterVolumeLevelScalar(level, std::ptr::null())
            .map_err(|e| e.to_string())?;
        Ok(level)
    }
}

/// Toggles mute and returns the new state.
pub fn toggle_mute(ep: Endpoint) -> Result<bool, String> {
    let v = volume(ep)?;
    unsafe {
        let muted = !v.GetMute().map_err(|e| e.to_string())?.as_bool();
        v.SetMute(muted, std::ptr::null())
            .map_err(|e| e.to_string())?;
        Ok(muted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_bounds_and_step() {
        assert_eq!(clamp_level(0.99, 2), 1.0);
        assert_eq!(clamp_level(0.01, -2), 0.0);
        assert_eq!(clamp_level(0.5, 2), 0.52);
    }

    #[test]
    #[ignore = "changes the real system volume"]
    fn manual_volume() {
        let up = change_volume(Endpoint::Output, 2).unwrap();
        let back = change_volume(Endpoint::Output, -2).unwrap();
        println!("{up} -> {back}");
    }
}
