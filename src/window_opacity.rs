use eframe::Frame;

pub const MIN_OPACITY: f32 = 0.25;
pub const MAX_OPACITY: f32 = 1.0;

pub fn clamp(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(MIN_OPACITY, MAX_OPACITY)
    } else {
        MAX_OPACITY
    }
}

pub fn is_supported() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(target_os = "windows")]
pub fn apply(frame: &Frame, opacity: f32) -> Result<f32, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
        WS_EX_LAYERED,
    };

    let handle = frame
        .window_handle()
        .map_err(|error| format!("native window handle is unavailable: {error}"))?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("the active window is not a Win32 window".into());
    };
    let hwnd = handle.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    let opacity = clamp(opacity);
    let alpha = (opacity * 255.0).round() as u8;

    unsafe {
        let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, current_style | WS_EX_LAYERED as isize);
        if SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) == 0 {
            return Err("Windows rejected the requested window opacity".into());
        }
    }

    Ok(opacity)
}

#[cfg(not(target_os = "windows"))]
pub fn apply(_frame: &Frame, _opacity: f32) -> Result<f32, String> {
    Err("window opacity is not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_is_finite_and_stays_in_supported_range() {
        assert_eq!(clamp(0.1), MIN_OPACITY);
        assert_eq!(clamp(0.75), 0.75);
        assert_eq!(clamp(1.2), MAX_OPACITY);
        assert_eq!(clamp(f32::NAN), MAX_OPACITY);
        assert_eq!(is_supported(), cfg!(target_os = "windows"));
    }
}
