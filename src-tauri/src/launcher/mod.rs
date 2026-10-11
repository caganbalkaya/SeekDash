use crate::models::{ResultKind, SearchResult};
use base64::Engine as _;
use std::{iter, os::windows::ffi::OsStrExt, path::Path, process::Command};
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        },
        Storage::FileSystem::{FILE_FLAGS_AND_ATTRIBUTES, WIN32_FIND_DATAW},
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IBindCtx,
            IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READ,
        },
        UI::{
            Shell::{
                IShellLinkW, SHGetFileInfoW, SHParseDisplayName, ShellExecuteExW, ShellExecuteW,
                ShellLink, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW, SHFILEINFOW, SHGFI_ICON,
                SHGFI_LARGEICON, SHGFI_PIDL,
            },
            WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO, SW_SHOWNORMAL},
        },
    },
};

pub fn shortcut_identity(value: &str) -> Option<String> {
    if !Path::new(value)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
    {
        return None;
    }
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| {
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            let persist: IPersistFile = link.cast().ok()?;
            let shortcut = wide(value);
            persist.Load(PCWSTR(shortcut.as_ptr()), STGM_READ).ok()?;
            let mut target = vec![0u16; 32_768];
            let mut data = WIN32_FIND_DATAW::default();
            link.GetPath(&mut target, &mut data, 0).ok()?;
            let mut arguments = vec![0u16; 2_048];
            link.GetArguments(&mut arguments).ok()?;
            let target = String::from_utf16_lossy(
                &target[..target
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(target.len())],
            );
            if target.trim().is_empty() {
                return None;
            }
            let arguments = String::from_utf16_lossy(
                &arguments[..arguments
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(arguments.len())],
            );
            Some(format!(
                "{}|{}",
                target.replace('/', "\\").to_lowercase(),
                arguments.trim().to_lowercase()
            ))
        })();
        if initialized {
            CoUninitialize();
        }
        result
    }
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

pub fn launch(result: &SearchResult, action: &str) -> Result<(), String> {
    if result.kind == ResultKind::Calculator {
        let escaped = result.path.replace('\'', "''");
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", escaped),
            ])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if result.kind == ResultKind::WebSearch {
        return shell_open(&format!(
            "https://www.google.com/search?q={}",
            urlencoding::encode(&result.path)
        ));
    }
    match action {
        "runas" => run_as(result),
        "reveal" => {
            let target = if Path::new(&result.path).is_dir() {
                result.path.clone()
            } else {
                format!("/select,{}", result.path)
            };
            Command::new("explorer.exe")
                .arg(target)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        "properties" => properties(&result.path),
        _ => shell_open(&result.path),
    }
}

fn is_elevatable(result: &SearchResult) -> bool {
    matches!(result.kind, ResultKind::Application | ResultKind::Shortcut)
        && Path::new(&result.path)
            .extension()
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("lnk")
            })
}

fn run_as(result: &SearchResult) -> Result<(), String> {
    if !is_elevatable(result) {
        return Err("This item cannot be run as administrator".into());
    }
    let target = wide(&result.path);
    let verb = wide("runas");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        hwnd: HWND(std::ptr::null_mut()),
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(target.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info).map_err(|error| error.to_string()) }
}

fn shell_open(value: &str) -> Result<(), String> {
    let target = wide(value);
    let verb = wide("open");
    let result = unsafe {
        ShellExecuteW(
            HWND(std::ptr::null_mut()),
            PCWSTR(verb.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err(format!(
            "Windows could not open this item ({})",
            result.0 as isize
        ))
    } else {
        Ok(())
    }
}

pub fn open_external_link(value: &str) -> Result<(), String> {
    shell_open(value)
}

fn properties(value: &str) -> Result<(), String> {
    let target = wide(value);
    let verb = wide("properties");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_INVOKEIDLIST,
        hwnd: HWND(std::ptr::null_mut()),
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(target.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info).map_err(|e| e.to_string()) }
}

pub fn associated_icon(value: &str) -> Option<String> {
    let icon = unsafe { shell_icon(value)? };
    let encoded = icon_to_png(icon);
    unsafe {
        let _ = DestroyIcon(icon);
    }
    encoded.map(|png| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    })
}

unsafe fn shell_icon(value: &str) -> Option<HICON> {
    let mut info = SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_LARGEICON;

    let found = if value.starts_with("shell:") {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let display_name = wide(value);
        let mut pidl = std::ptr::null_mut();
        let parsed = SHParseDisplayName(
            PCWSTR(display_name.as_ptr()),
            None::<&IBindCtx>,
            &mut pidl,
            0,
            None,
        )
        .is_ok();
        let result = parsed
            && !pidl.is_null()
            && SHGetFileInfoW(
                PCWSTR(pidl.cast()),
                FILE_FLAGS_AND_ATTRIBUTES(0),
                Some(&mut info),
                std::mem::size_of::<SHFILEINFOW>() as u32,
                flags | SHGFI_PIDL,
            ) != 0;
        if !pidl.is_null() {
            CoTaskMemFree(Some(pidl.cast()));
        }
        if initialized {
            CoUninitialize();
        }
        result
    } else {
        let path = wide(value);
        SHGetFileInfoW(
            PCWSTR(path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        ) != 0
    };

    (found && !info.hIcon.0.is_null()).then_some(info.hIcon)
}

fn icon_to_png(icon: HICON) -> Option<Vec<u8>> {
    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &mut icon_info).ok()? };

    let result = (|| {
        if icon_info.hbmColor.0.is_null() {
            return None;
        }
        let mut bitmap = BITMAP::default();
        let copied = unsafe {
            GetObjectW(
                icon_info.hbmColor,
                std::mem::size_of::<BITMAP>() as i32,
                Some((&mut bitmap as *mut BITMAP).cast()),
            )
        };
        if copied == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        let width = bitmap.bmWidth as u32;
        let height = bitmap.bmHeight as u32;
        let mut rgba = unsafe { read_bgra(icon_info.hbmColor, width, height)? };

        if rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) && !icon_info.hbmMask.0.is_null() {
            let mask = unsafe { read_bgra(icon_info.hbmMask, width, height)? };
            for (pixel, mask_pixel) in rgba.chunks_exact_mut(4).zip(mask.chunks_exact(4)) {
                pixel[3] = if mask_pixel[0] == 0 { 255 } else { 0 };
            }
        }
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        let mut output = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut output, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&rgba).ok()?;
        }
        Some(output)
    })();

    unsafe {
        if !icon_info.hbmColor.0.is_null() {
            let _ = DeleteObject(icon_info.hbmColor);
        }
        if !icon_info.hbmMask.0.is_null() {
            let _ = DeleteObject(icon_info.hbmMask);
        }
    }
    result
}

unsafe fn read_bgra(
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    let byte_count = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    let mut pixels = vec![0; byte_count];
    let mut info = BITMAPINFO::default();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };
    let dc = CreateCompatibleDC(None);
    let rows = GetDIBits(
        dc,
        bitmap,
        0,
        height,
        Some(pixels.as_mut_ptr().cast()),
        &mut info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);
    (rows == height as i32).then_some(pixels)
}

#[cfg(test)]
mod icon_tests {
    use super::{associated_icon, is_elevatable};
    use crate::models::{ResultKind, SearchResult};

    #[test]
    fn elevation_is_limited_to_win32_executables_and_shortcuts() {
        let result = |path: &str, kind: ResultKind| SearchResult {
            id: "1".into(),
            title: "x".into(),
            subtitle: String::new(),
            path: path.into(),
            kind,
            score: 0.0,
            icon: None,
            favorite: false,
        };
        assert!(is_elevatable(&result(
            r"C:\\Tools\\app.exe",
            ResultKind::Application
        )));
        assert!(is_elevatable(&result(
            r"C:\\Tools\\app.lnk",
            ResultKind::Shortcut
        )));
        assert!(!is_elevatable(&result(
            r"C:\\Docs\\file.txt",
            ResultKind::File
        )));
        assert!(!is_elevatable(&result(
            "shell:AppsFolder\\Example",
            ResultKind::Application
        )));
    }

    #[test]
    fn extracts_a_native_windows_executable_icon() {
        let windows = std::env::var("WINDIR").expect("WINDIR must be available on Windows");
        let path = std::path::Path::new(&windows)
            .join("System32")
            .join("notepad.exe");
        let icon = associated_icon(path.to_string_lossy().as_ref())
            .expect("Windows Shell should return the executable icon");
        assert!(icon.starts_with("data:image/png;base64,"));
        assert!(icon.len() > 128);
    }

    #[test]
    fn extracts_a_windows_shortcut_icon() {
        let appdata = std::env::var("APPDATA").expect("APPDATA must be available on Windows");
        let programs =
            std::path::Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs");
        let icon = walkdir::WalkDir::new(programs)
            .max_depth(4)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))
            })
            .find_map(|entry| associated_icon(entry.path().to_string_lossy().as_ref()));
        let Some(icon) = icon else {
            return;
        };
        assert!(icon.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn extracts_a_uwp_shell_icon_when_available() {
        let output = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-StartApps | Where-Object AppID -Like '*!*' | Select-Object -First 1 -ExpandProperty AppID",
            ])
            .output()
            .expect("PowerShell should start");
        let app_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if app_id.is_empty() {
            return;
        }
        let icon = associated_icon(&format!(r"shell:AppsFolder\{app_id}"))
            .expect("Windows Shell should resolve a UWP icon");
        assert!(icon.starts_with("data:image/png;base64,"));
    }
}
