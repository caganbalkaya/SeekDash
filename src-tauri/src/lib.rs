mod clipboard;
mod database;
mod indexer;
mod launcher;
mod models;
mod quick_actions;
mod search;
mod state;
mod windows;

use models::{AppSettings, BackdropStatus, IndexStatus, SearchResult};
use state::{EngineState, SidebarState, WindowPhase};
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

fn supported_language(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().split(['-', '_']).next()? {
        "en" => Some("en"),
        "tr" => Some("tr"),
        "de" => Some("de"),
        "fr" => Some("fr"),
        _ => None,
    }
}

fn windows_ui_language() -> Option<String> {
    let mut buffer = [0u16; 85];
    let length = unsafe { ::windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buffer) };
    (length > 1).then(|| String::from_utf16_lossy(&buffer[..length as usize - 1]))
}

fn installer_language_id(value: u32) -> Option<&'static str> {
    match value {
        1033 => Some("en"),
        1055 => Some("tr"),
        1031 => Some("de"),
        1036 => Some("fr"),
        _ => None,
    }
}

fn installer_language() -> Option<&'static str> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\SeekDash\SeekDash")
        .ok()?;
    key.get_value::<String, _>("InitialLanguage")
        .ok()
        .and_then(|value| supported_language(&value))
        .or_else(|| {
            key.get_value::<u32, _>("Installer Language")
                .ok()
                .and_then(installer_language_id)
        })
        .or_else(|| {
            key.get_value::<String, _>("Installer Language")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .and_then(installer_language_id)
        })
}

fn resolve_language(saved: &str) -> String {
    supported_language(saved)
        .or_else(installer_language)
        .or_else(|| {
            std::env::var("SEEKDASH_INSTALL_LANGUAGE")
                .or_else(|_| std::env::var("GLASS_INSTALL_LANGUAGE"))
                .ok()
                .and_then(|v| supported_language(&v))
        })
        .or_else(|| windows_ui_language().and_then(|v| supported_language(&v)))
        .unwrap_or("en")
        .to_string()
}

fn native_text(language: &str, key: &str) -> &'static str {
    match (language, key) {
        ("tr", "settings") => "SeekDash Ayarları",
        ("de", "settings") => "SeekDash-Einstellungen",
        ("fr", "settings") => "Réglages SeekDash",
        ("tr", "open") => "SeekDash'i Aç",
        ("de", "open") => "SeekDash öffnen",
        ("fr", "open") => "Ouvrir SeekDash",
        ("tr", "rebuild") => "İndeksi Yenile",
        ("de", "rebuild") => "Index neu erstellen",
        ("fr", "rebuild") => "Reconstruire l’index",
        ("tr", "exit") => "Çıkış",
        ("de", "exit") => "Beenden",
        ("fr", "exit") => "Quitter",
        (_, "settings") => "SeekDash Settings",
        (_, "open") => "Open SeekDash",
        (_, "rebuild") => "Rebuild Index",
        (_, "exit") => "Exit",
        _ => "SeekDash",
    }
}

fn tray_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(
        app,
        "open",
        native_text(language, "open"),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(
        app,
        "settings",
        native_text(language, "settings"),
        true,
        None::<&str>,
    )?;
    let rebuild = MenuItem::with_id(
        app,
        "rebuild",
        native_text(language, "rebuild"),
        true,
        None::<&str>,
    )?;
    let exit = MenuItem::with_id(
        app,
        "exit",
        native_text(language, "exit"),
        true,
        None::<&str>,
    )?;
    Menu::with_items(app, &[&open, &settings, &rebuild, &exit])
}

#[cfg(test)]
mod language_tests {
    use super::{installer_language_id, supported_language};

    #[test]
    fn accepts_supported_windows_locale_variants() {
        assert_eq!(supported_language("tr-TR"), Some("tr"));
        assert_eq!(supported_language("de_DE"), Some("de"));
        assert_eq!(supported_language("FR-fr"), Some("fr"));
        assert_eq!(supported_language("es-ES"), None);
    }

    #[test]
    fn maps_supported_nsis_language_ids() {
        assert_eq!(installer_language_id(1033), Some("en"));
        assert_eq!(installer_language_id(1055), Some("tr"));
        assert_eq!(installer_language_id(1031), Some("de"));
        assert_eq!(installer_language_id(1036), Some("fr"));
        assert_eq!(installer_language_id(1040), None);
    }
}

fn load_settings(app: &AppHandle) -> AppSettings {
    let mut settings: AppSettings = settings_path(app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    settings.language = resolve_language(&settings.language);
    settings
}

#[tauri::command]
async fn search(
    app: AppHandle,
    query: String,
    request_id: u64,
) -> Result<Vec<SearchResult>, String> {
    app.state::<EngineState>()
        .latest_request
        .store(request_id, Ordering::Release);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<EngineState>();
        if state.latest_request.load(Ordering::Acquire) != request_id {
            return Ok(Vec::new());
        }
        let result = search::execute(&state, &query, 50, request_id)?;
        if state.latest_request.load(Ordering::Acquire) != request_id {
            return Ok(Vec::new());
        }
        Ok(result)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn launch_result(app: AppHandle, result: SearchResult, action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        launcher::launch(&result, &action)?;
        if action == "open" {
            let _ = app
                .state::<EngineState>()
                .db
                .lock()
                .record_open(&result.path);
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn open_about_link(target: String) -> Result<(), String> {
    let url = match target.as_str() {
        "website" => "https://seekdash.com.tr",
        "github" => "https://github.com/caganbalkaya",
        _ => return Err("Unknown About link".into()),
    };
    tauri::async_runtime::spawn_blocking(move || launcher::open_external_link(url))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn copy_path(app: AppHandle, path: String) -> Result<(), String> {
    if path.len() > 32_768 {
        return Err("Path is too long".into());
    }
    clipboard::copy_text(app, path).await
}

#[tauri::command]
fn set_favorite(
    path: String,
    favorite: bool,
    state: tauri::State<EngineState>,
) -> Result<(), String> {
    state
        .db
        .lock()
        .set_favorite(&path, favorite)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings(state: tauri::State<EngineState>) -> AppSettings {
    state.settings.lock().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    mut settings: AppSettings,
    state: tauri::State<EngineState>,
) -> Result<(), String> {
    settings.clipboard_limit = settings.clipboard_limit.clamp(1, 100);
    settings.language = supported_language(&settings.language)
        .unwrap_or("en")
        .to_string();
    let previous = state.settings.lock().clone();
    if settings
        .hotkey
        .eq_ignore_ascii_case(&settings.clipboard_hotkey)
    {
        let error = "i18n:backend.hotkeyConflict".to_string();
        *state.hotkey_error.lock() = Some(error.clone());
        return Err(error);
    }
    if previous.hotkey != settings.hotkey || previous.clipboard_hotkey != settings.clipboard_hotkey
    {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())?;
        if let Err(error) = register_hotkeys(&app, &settings) {
            let _ = app.global_shortcut().unregister_all();
            let _ = register_hotkeys(&app, &previous);
            let error = format!("i18n:backend.hotkeyUnavailable|{error}");
            *state.hotkey_error.lock() = Some(error.clone());
            return Err(error);
        }
        *state.hotkey_error.lock() = None;
    }
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(settings_path(&app)?, content).map_err(|e| e.to_string())?;
    let autostart = app.autolaunch();
    if settings.launch_at_startup {
        autostart.enable().map_err(|e| e.to_string())?;
    } else {
        autostart.disable().map_err(|e| e.to_string())?;
    }
    let roots_changed = previous.indexed_directories != settings.indexed_directories
        || previous.excluded_directories != settings.excluded_directories;
    *state.settings.lock() = settings;
    let current = state.settings.lock().clone();
    clipboard::apply_settings(&app, &previous, &current);
    if previous.language != current.language {
        if let Some(window) = app.get_webview_window("settings") {
            let _ = window.set_title(native_text(&current.language, "settings"));
        }
        if let Some(tray) = app.tray_by_id("main-tray") {
            if let Ok(menu) = tray_menu(&app, &current.language) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
    let _ = app.emit("settings-updated", current);
    if roots_changed {
        indexer::rebuild(app);
    }
    Ok(())
}

#[tauri::command]
fn rebuild_index(app: AppHandle) {
    indexer::rebuild(app);
}

#[tauri::command]
fn clear_history(state: tauri::State<EngineState>) -> Result<(), String> {
    state.db.lock().clear_history().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_index_status(state: tauri::State<EngineState>) -> IndexStatus {
    state.status.lock().clone()
}

#[tauri::command]
fn get_backdrop_status(state: tauri::State<BackdropStatus>) -> BackdropStatus {
    state.inner().clone()
}

#[tauri::command]
fn get_hotkey_error(state: tauri::State<EngineState>) -> Option<String> {
    state.hotkey_error.lock().clone()
}

#[tauri::command]
async fn get_icon(app: AppHandle, path: String) -> Option<String> {
    if let Some(cached) = app
        .state::<EngineState>()
        .icon_cache
        .lock()
        .get(&path)
        .cloned()
    {
        return cached;
    }
    let icon_path = path.clone();
    let icon = tauri::async_runtime::spawn_blocking(move || launcher::associated_icon(&icon_path))
        .await
        .ok()
        .flatten();
    let state = app.state::<EngineState>();
    let mut cache = state.icon_cache.lock();
    if cache.len() >= 256 && !cache.contains_key(&path) {
        if let Some(evicted) = cache.keys().next().cloned() {
            cache.remove(&evicted);
        }
    }
    cache.insert(path, icon.clone());
    icon
}

fn register_hotkeys(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    app.global_shortcut()
        .register(settings.hotkey.as_str())
        .map_err(|e| format!("{}: {e}", settings.hotkey))?;
    app.global_shortcut()
        .register(settings.clipboard_hotkey.as_str())
        .map_err(|e| format!("{}: {e}", settings.clipboard_hotkey))
}

fn log_timing(label: &str, elapsed: Duration) {
    #[cfg(debug_assertions)]
    eprintln!("SeekDash timing: {label} {elapsed:?}");
    if elapsed > Duration::from_millis(500) {
        eprintln!("SeekDash slow-operation: {label} took {elapsed:?}");
    }
}

fn hide_launcher_window(app: &AppHandle) {
    let state = app.state::<EngineState>();
    let was_clipboard = {
        let mut runtime = state.windows.lock();
        if runtime.phase == WindowPhase::Hidden || runtime.phase == WindowPhase::Closing {
            return;
        }
        runtime.phase = WindowPhase::Closing;
        runtime.clipboard_visible
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let mut runtime = state.windows.lock();
    runtime.phase = WindowPhase::Hidden;
    runtime.sidebar_state = SidebarState::Hidden;
    runtime.clipboard_visible = false;
    drop(runtime);
    if was_clipboard {
        let _ = app.emit("clipboard-closed", ());
    }
}

#[tauri::command]
fn hide_launcher(app: AppHandle) {
    hide_launcher_window(&app);
}

fn position_launcher(
    window: &tauri::WebviewWindow,
    settings: &AppSettings,
    sidebar_state: SidebarState,
) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        let vertical = settings.layout_mode == "vertical";
        let logical_work_height = work.size.height as f64 / scale;
        let (width, height) = if vertical {
            let width = if sidebar_state == SidebarState::Collapsed {
                56.0
            } else {
                400.0
            };
            (
                width,
                (logical_work_height * 0.85).clamp(420.0, logical_work_height),
            )
        } else {
            (720.0, 82.0)
        };
        let _ = window.set_size(LogicalSize::new(width, height));
        let physical_width = (width * scale).round() as u32;
        let physical_height = (height * scale).round() as u32;
        let margin = (12.0 * scale).round() as u32;
        let x_offset = if vertical {
            if settings.sidebar_edge == "right" {
                work.size.width.saturating_sub(physical_width + margin)
            } else {
                margin
            }
        } else {
            work.size.width.saturating_sub(physical_width) / 2
        };
        let y_offset = if vertical {
            work.size.height.saturating_sub(physical_height) / 2
        } else {
            (work.size.height as f64 * 0.12) as u32
        };
        let x = work.position.x + x_offset as i32;
        let y = work.position.y + y_offset as i32;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

fn position_clipboard(window: &tauri::WebviewWindow, settings: &AppSettings) {
    if settings.layout_mode == "vertical" {
        position_launcher(window, settings, SidebarState::Expanded);
        return;
    }
    if let Ok(Some(monitor)) = window.current_monitor() {
        let work = monitor.work_area();
        let scale = monitor.scale_factor();
        let width = 720.0;
        let height = 520.0;
        let physical_width = (width * scale).round() as u32;
        let x = work.position.x + work.size.width.saturating_sub(physical_width) as i32 / 2;
        let y = work.position.y + (work.size.height as f64 * 0.12) as i32;
        let _ = window.set_size(LogicalSize::new(width, height));
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

fn set_sidebar_state_internal(app: &AppHandle, target: SidebarState) -> Result<(), String> {
    let settings = app.state::<EngineState>().settings.lock().clone();
    if settings.layout_mode != "vertical" {
        return Ok(());
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "i18n:backend.launcherMissing".to_string())?;
    if target == SidebarState::Hidden {
        let _ = window.hide();
        let state = app.state::<EngineState>();
        let mut runtime = state.windows.lock();
        runtime.sidebar_state = target;
        runtime.phase = WindowPhase::Hidden;
        runtime.clipboard_visible = false;
        let _ = app.emit("sidebar-state-changed", target.as_str());
        let _ = app.emit("clipboard-closed", ());
        return Ok(());
    }
    {
        let state = app.state::<EngineState>();
        let mut runtime = state.windows.lock();
        runtime.phase = WindowPhase::Opening;
        runtime.sidebar_state = target;
        runtime.clipboard_visible = false;
    }
    position_launcher(&window, &settings, target);
    let _ = windows::apply_rounded_region(&window, 22.0);
    window.show().map_err(|error| error.to_string())?;
    let _ = app.emit("sidebar-state-changed", target.as_str());
    if target == SidebarState::Collapsed {
        let _ = app.emit("clipboard-closed", ());
    }
    if target == SidebarState::Expanded {
        window.set_focus().map_err(|error| error.to_string())?;
        let _ = app.emit("launcher-opened", ());
    }
    app.state::<EngineState>().windows.lock().phase = WindowPhase::Visible;
    Ok(())
}

#[tauri::command]
fn set_sidebar_state(app: AppHandle, state: String) -> Result<(), String> {
    let target = match state.as_str() {
        "hidden" => SidebarState::Hidden,
        "collapsed" => SidebarState::Collapsed,
        "expanded" => SidebarState::Expanded,
        _ => return Err("Geçersiz sidebar durumu".into()),
    };
    set_sidebar_state_internal(&app, target)
}

#[tauri::command]
fn get_sidebar_state(state: tauri::State<EngineState>) -> String {
    state.windows.lock().sidebar_state.as_str().into()
}

fn toggle_launcher(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let state = app.state::<EngineState>();
    let settings = state.settings.lock().clone();
    let (phase, sidebar_state) = {
        let runtime = state.windows.lock();
        (runtime.phase, runtime.sidebar_state)
    };
    if phase == WindowPhase::SettingsVisible {
        if let Some(settings_window) = app.get_webview_window("settings") {
            let _ = settings_window.hide();
        }
    }
    state.windows.lock().clipboard_visible = false;
    if settings.layout_mode == "vertical" {
        let target = match (phase, sidebar_state) {
            (WindowPhase::SettingsVisible, _) | (_, SidebarState::Hidden) => {
                SidebarState::Collapsed
            }
            (_, SidebarState::Collapsed) => SidebarState::Hidden,
            (_, SidebarState::Expanded) => SidebarState::Collapsed,
        };
        let _ = set_sidebar_state_internal(app, target);
        return;
    }
    if matches!(phase, WindowPhase::Visible | WindowPhase::Opening)
        || window.is_visible().unwrap_or(false)
    {
        hide_launcher_window(app);
        return;
    }
    state.windows.lock().phase = WindowPhase::Opening;
    state.windows.lock().clipboard_visible = false;
    position_launcher(&window, &settings, SidebarState::Expanded);
    let _ = windows::apply_rounded_region(&window, 22.0);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("launcher-opened", ());
    state.windows.lock().phase = WindowPhase::Visible;
}

fn toggle_clipboard(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let state = app.state::<EngineState>();
    let settings = state.settings.lock().clone();
    let (phase, clipboard_visible) = {
        let runtime = state.windows.lock();
        (runtime.phase, runtime.clipboard_visible)
    };
    if phase == WindowPhase::SettingsVisible {
        if let Some(settings_window) = app.get_webview_window("settings") {
            let _ = settings_window.hide();
        }
    }
    if clipboard_visible && window.is_visible().unwrap_or(false) {
        hide_launcher_window(app);
        return;
    }
    {
        let mut runtime = state.windows.lock();
        runtime.phase = WindowPhase::Opening;
        runtime.clipboard_visible = true;
        runtime.sidebar_state = SidebarState::Expanded;
    }
    position_clipboard(&window, &settings);
    let _ = windows::apply_rounded_region(&window, 22.0);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("clipboard-opened", ());
    state.windows.lock().phase = WindowPhase::Visible;
}

#[tauri::command]
fn show_settings(app: AppHandle) -> Result<(), String> {
    open_settings(&app)
}

fn create_settings_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("settings").is_some() {
        return Ok(());
    }
    let started = Instant::now();
    app.state::<EngineState>()
        .windows
        .lock()
        .settings_create_started = Some(started);
    let result = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?settings".into()),
    )
    .title(native_text(
        &app.state::<EngineState>().settings.lock().language,
        "settings",
    ))
    .inner_size(850.0, 600.0)
    .min_inner_size(720.0, 520.0)
    .resizable(true)
    .decorations(true)
    .transparent(true)
    .center()
    .visible(false)
    .build()
    .map_err(|error| error.to_string());
    if result.is_err() {
        app.state::<EngineState>()
            .windows
            .lock()
            .settings_create_started = None;
    }
    let window = result?;
    let _ = windows::apply_settings_backdrop(&window);
    log_timing("settings-webview-create", started.elapsed());
    Ok(())
}

fn open_settings(app: &AppHandle) -> Result<(), String> {
    let started = Instant::now();
    #[cfg(debug_assertions)]
    eprintln!("SeekDash timing: settings-open-request");
    let state = app.state::<EngineState>();
    {
        let mut runtime = state.windows.lock();
        if runtime.phase == WindowPhase::SettingsVisible {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.set_focus();
                return Ok(());
            }
        }
        runtime.phase = WindowPhase::SettingsVisible;
        runtime.settings_open_started = Some(started);
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }
    let reused = app.get_webview_window("settings").is_some();
    let result = (|| {
        create_settings_window(app)?;
        if reused {
            log_timing("settings-window-reuse", started.elapsed());
        }
        let window = app
            .get_webview_window("settings")
            .ok_or_else(|| "i18n:backend.settingsMissing".to_string())?;
        let show_started = Instant::now();
        window.show().map_err(|error| error.to_string())?;
        log_timing("settings-window-show", show_started.elapsed());
        let focus_started = Instant::now();
        window.set_focus().map_err(|error| error.to_string())?;
        log_timing("settings-focus-acquisition", focus_started.elapsed());
        let _ = app.emit_to("settings", "settings-open-requested", ());
        log_timing("settings-show-and-focus", started.elapsed());
        Ok(())
    })();
    if result.is_err() {
        let mut runtime = state.windows.lock();
        runtime.phase = WindowPhase::Hidden;
        runtime.settings_open_started = None;
    }
    result
}

#[tauri::command]
fn hide_settings(app: AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.hide();
    }
    let state = app.state::<EngineState>();
    let mut runtime = state.windows.lock();
    runtime.phase = WindowPhase::Hidden;
    runtime.sidebar_state = SidebarState::Hidden;
    runtime.clipboard_visible = false;
    runtime.settings_open_started = None;
}

#[tauri::command]
fn settings_webview_ready(app: AppHandle) {
    let state = app.state::<EngineState>();
    let mut runtime = state.windows.lock();
    runtime.settings_ready = true;
    if let Some(started) = runtime.settings_create_started.take() {
        log_timing("settings-webview-ready", started.elapsed());
    }
}

#[tauri::command]
fn settings_interactive(app: AppHandle) -> Option<u64> {
    let started = app
        .state::<EngineState>()
        .windows
        .lock()
        .settings_open_started
        .take()?;
    let elapsed = started.elapsed();
    log_timing("settings-interactive", elapsed);
    Some(elapsed.as_millis() as u64)
}

fn tray_image() -> tauri::image::Image<'static> {
    tauri::image::Image::from_app_icon_resource(32).expect("SeekDash tray icon could not be loaded")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            toggle_launcher(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("SeekDash")
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let settings = app.state::<EngineState>().settings.lock().clone();
                        let clipboard_id = settings
                            .clipboard_hotkey
                            .parse::<Shortcut>()
                            .ok()
                            .map(|value| value.id());
                        if clipboard_id == Some(shortcut.id()) {
                            toggle_clipboard(app);
                        } else {
                            toggle_launcher(app);
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let settings = load_settings(&handle);
            let data_dir = handle.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            let database = database::Database::open(&data_dir.join("glass.db"))?;
            let count = database.count().unwrap_or(0);
            let search_database = database::Database::open(&data_dir.join("glass.db"))?;
            handle.manage(EngineState {
                db: parking_lot::Mutex::new(database),
                search_db: parking_lot::Mutex::new(search_database),
                settings: parking_lot::Mutex::new(settings.clone()),
                status: parking_lot::Mutex::new(IndexStatus {
                    message: format!("i18n:index.complete|{count}|0"),
                    indexed: count,
                    ..Default::default()
                }),
                latest_request: AtomicU64::new(0),
                watcher: parking_lot::Mutex::new(None),
                icon_cache: parking_lot::Mutex::new(std::collections::HashMap::new()),
                shortcut_cache: parking_lot::Mutex::new(std::collections::HashMap::new()),
                clipboard: Default::default(),
                hotkey_error: parking_lot::Mutex::new(None),
                windows: parking_lot::Mutex::new(Default::default()),
            });
            clipboard::initialize(&handle);

            let backdrop = handle
                .get_webview_window("main")
                .map(|window| windows::apply_rounded_region(&window, 22.0))
                .unwrap_or_else(|| BackdropStatus {
                    mode: "css_fallback".into(),
                    native: false,
                    detail: "Ana pencere bulunamadı".into(),
                });
            let _ = fs::write(
                data_dir.join("backdrop-status.json"),
                serde_json::to_vec_pretty(&backdrop)?,
            );
            handle.manage(backdrop);
            if let Err(error) = create_settings_window(&handle) {
                eprintln!("Ayarlar penceresi önceden oluşturulamadı: {error}");
            }
            let _ = handle.global_shortcut().unregister_all();
            if let Err(error) = register_hotkeys(&handle, &settings) {
                *handle.state::<EngineState>().hotkey_error.lock() =
                    Some(format!("Global kısayol kaydedilemedi: {error}"));
                eprintln!("{error}");
            }

            let menu = tray_menu(&handle, &settings.language)?;
            TrayIconBuilder::with_id("main-tray")
                .icon(tray_image())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        toggle_launcher(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => toggle_launcher(app),
                    "settings" => {
                        if let Err(error) = open_settings(app) {
                            eprintln!("Ayarlar açılamadı: {error}");
                        }
                    }
                    "rebuild" => indexer::rebuild(app.clone()),
                    "exit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if count == 0 {
                indexer::rebuild(handle.clone());
            } else {
                indexer::start_watcher(handle.clone());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                    api.prevent_close();
                    hide_launcher_window(app);
                }
                WindowEvent::CloseRequested { api, .. } if window.label() == "settings" => {
                    api.prevent_close();
                    let _ = window.hide();
                    let state = app.state::<EngineState>();
                    let mut runtime = state.windows.lock();
                    runtime.phase = WindowPhase::Hidden;
                    runtime.sidebar_state = SidebarState::Hidden;
                    runtime.clipboard_visible = false;
                    runtime.settings_open_started = None;
                }
                WindowEvent::Focused(false) if window.label() == "main" => {
                    let state = app.state::<EngineState>();
                    let settings = state.settings.lock().clone();
                    let (phase, sidebar_state) = {
                        let runtime = state.windows.lock();
                        (runtime.phase, runtime.sidebar_state)
                    };
                    if settings.layout_mode == "vertical" {
                        if settings.sidebar_auto_hide && sidebar_state == SidebarState::Expanded {
                            let _ = set_sidebar_state_internal(app, SidebarState::Collapsed);
                        }
                    } else if matches!(phase, WindowPhase::Visible | WindowPhase::Opening) {
                        hide_launcher_window(app);
                    }
                }
                WindowEvent::Resized(_) if window.label() == "main" => {
                    if let Some(main) = app.get_webview_window("main") {
                        let _ = windows::apply_rounded_region(&main, 22.0);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            search,
            launch_result,
            open_about_link,
            copy_path,
            set_favorite,
            show_settings,
            hide_launcher,
            set_sidebar_state,
            get_sidebar_state,
            hide_settings,
            settings_webview_ready,
            settings_interactive,
            get_settings,
            save_settings,
            rebuild_index,
            clear_history,
            get_index_status,
            get_backdrop_status,
            get_hotkey_error,
            get_icon,
            clipboard::search_clipboard,
            clipboard::copy_clipboard_item,
            clipboard::delete_clipboard_item,
            clipboard::clear_clipboard_history,
            quick_actions::execute_quick_action,
            quick_actions::copy_command_value
        ])
        .run(tauri::generate_context!())
        .expect("failed to run SeekDash");
}
