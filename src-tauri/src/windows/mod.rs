use crate::models::BackdropStatus;
use tauri::WebviewWindow;

#[cfg(windows)]
pub fn apply_rounded_region(window: &WebviewWindow, _radius_logical: f64) -> BackdropStatus {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE},
    };

    let Ok(raw) = window.hwnd() else {
        return fallback("Pencere HWND bilgisi alınamadı");
    };
    let hwnd = HWND(raw.0);

    unsafe {
        // DWM'nin sabit sistem radius'u CSS'teki 22/24 px şekille eşleşmez. Dış HWND
        // tamamen saydam kalır; görünen tek şekli WebView'in anti-aliased clip'i çizer.
        let corner_preference = 1i32; // DWMWCP_DONOTROUND
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            &corner_preference as *const _ as *const _,
            std::mem::size_of_val(&corner_preference) as u32,
        );
        let backdrop_type = 1i32; // DWMSBT_NONE
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(38),
            &backdrop_type as *const _ as *const _,
            std::mem::size_of_val(&backdrop_type) as u32,
        );
    }

    BackdropStatus {
        mode: "transparent_webview_css_clip".into(),
        native: false,
        detail: "Saydam WebView üzerinde tek anti-aliased CSS radius etkin".into(),
    }
}

#[cfg(windows)]
pub fn apply_settings_backdrop(window: &WebviewWindow) -> BackdropStatus {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE},
    };

    let Ok(raw) = window.hwnd() else {
        return fallback("Ayarlar penceresi HWND bilgisi alınamadı");
    };
    let hwnd = HWND(raw.0);

    unsafe {
        let corner_preference = 2i32; // DWMWCP_ROUND
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            &corner_preference as *const _ as *const _,
            std::mem::size_of_val(&corner_preference) as u32,
        );
        let backdrop_type = 2i32; // DWMSBT_MAINWINDOW (Windows 11 Mica)
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(38),
            &backdrop_type as *const _ as *const _,
            std::mem::size_of_val(&backdrop_type) as u32,
        );
    }

    BackdropStatus {
        mode: "windows_11_mica".into(),
        native: true,
        detail: "Ayarlar penceresinde Windows 11 Mica etkin".into(),
    }
}

#[cfg(not(windows))]
pub fn apply_rounded_region(_window: &WebviewWindow, _radius_logical: f64) -> BackdropStatus {
    fallback("Native pencere bölgesi yalnızca Windows'ta kullanılabilir")
}

#[cfg(not(windows))]
pub fn apply_settings_backdrop(_window: &WebviewWindow) -> BackdropStatus {
    fallback("Native ayarlar arka planı yalnızca Windows'ta kullanılabilir")
}

fn fallback(detail: impl Into<String>) -> BackdropStatus {
    BackdropStatus {
        mode: "css_fallback".into(),
        native: false,
        detail: detail.into(),
    }
}
