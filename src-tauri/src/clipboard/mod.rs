use crate::{
    models::{AppSettings, ClipboardItem},
    state::EngineState,
};
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    iter,
    os::windows::ffi::OsStrExt,
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc::{sync_channel, SyncSender},
        OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GlobalFree, HANDLE, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            DataExchange::{
                AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData,
                IsClipboardFormatAvailable, OpenClipboard, RemoveClipboardFormatListener,
                SetClipboardData,
            },
            LibraryLoader::GetModuleHandleW,
            Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
            Ole::CF_UNICODETEXT,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
            TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
            WNDCLASSW,
        },
    },
};

const MAX_ITEMS: usize = 100;
const MAX_CHARS: usize = 20_000;
static CLIPBOARD_EVENTS: OnceLock<SyncSender<()>> = OnceLock::new();

pub struct ClipboardStore {
    items: Mutex<VecDeque<ClipboardItem>>,
    suppress_next: Mutex<Option<String>>,
    next_id: AtomicI64,
}

impl Default for ClipboardStore {
    fn default() -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            suppress_next: Mutex::new(None),
            next_id: AtomicI64::new(now_millis()),
        }
    }
}

impl ClipboardStore {
    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn replace(&self, items: Vec<ClipboardItem>) {
        if let Some(maximum) = items.iter().map(|item| item.id).max() {
            self.next_id.fetch_max(maximum + 1, Ordering::Relaxed);
        }
        *self.items.lock() = items.into();
    }

    fn trim(&self, limit: usize) {
        let mut items = self.items.lock();
        items.truncate(valid_limit(limit));
    }
}

pub fn initialize(app: &AppHandle) {
    let settings = app.state::<EngineState>().settings.lock().clone();
    if settings.clipboard_persist {
        let items = app
            .state::<EngineState>()
            .db
            .lock()
            .load_clipboard_history(valid_limit(settings.clipboard_limit))
            .unwrap_or_default();
        app.state::<EngineState>().clipboard.replace(items);
    } else {
        let _ = app
            .state::<EngineState>()
            .db
            .lock()
            .clear_clipboard_history();
    }

    let (sender, receiver) = sync_channel(4);
    if CLIPBOARD_EVENTS.set(sender).is_err() {
        return;
    }
    let processor_app = app.clone();
    thread::spawn(move || {
        while receiver.recv().is_ok() {
            capture_update(&processor_app);
        }
    });
    thread::spawn(message_loop);
}

pub fn apply_settings(app: &AppHandle, previous: &AppSettings, current: &AppSettings) {
    app.state::<EngineState>()
        .clipboard
        .trim(current.clipboard_limit);

    if previous.clipboard_persist == current.clipboard_persist
        && previous.clipboard_limit == current.clipboard_limit
    {
        return;
    }
    let app = app.clone();
    let persist = current.clipboard_persist;
    let limit = valid_limit(current.clipboard_limit);
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<EngineState>();
        if !persist {
            let _ = state.db.lock().clear_clipboard_history();
            return;
        }
        let items: Vec<_> = state.clipboard.items.lock().iter().cloned().collect();
        let db = state.db.lock();
        let _ = db.clear_clipboard_history();
        for item in items.iter().rev() {
            let _ = db.save_clipboard_item(item, limit);
        }
    });
}

#[tauri::command]
pub fn search_clipboard(query: String, state: tauri::State<EngineState>) -> Vec<ClipboardItem> {
    let query = query.to_lowercase();
    state
        .clipboard
        .items
        .lock()
        .iter()
        .filter(|item| query.is_empty() || item.text.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

#[tauri::command]
pub async fn copy_clipboard_item(app: AppHandle, id: i64) -> Result<(), String> {
    let text = app
        .state::<EngineState>()
        .clipboard
        .items
        .lock()
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.text.clone())
        .ok_or_else(|| "i18n:backend.clipboardMissing".to_string())?;
    copy_text(app, text).await
}

pub async fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    *app.state::<EngineState>().clipboard.suppress_next.lock() = Some(text.clone());
    let clipboard_text = text.clone();
    let write = tauri::async_runtime::spawn_blocking(move || write_text(&clipboard_text))
        .await
        .map_err(|error| error.to_string())?;
    if write.is_err() {
        *app.state::<EngineState>().clipboard.suppress_next.lock() = None;
    } else {
        let cleanup_app = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            let state = cleanup_app.state::<EngineState>();
            let mut suppressed = state.clipboard.suppress_next.lock();
            if suppressed.as_deref() == Some(text.as_str()) {
                *suppressed = None;
            }
        });
    }
    write
}

#[tauri::command]
pub async fn delete_clipboard_item(app: AppHandle, id: i64) -> Result<(), String> {
    {
        let state = app.state::<EngineState>();
        state.clipboard.items.lock().retain(|item| item.id != id);
    }
    let persist = app.state::<EngineState>().settings.lock().clipboard_persist;
    if persist {
        let db_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            db_app
                .state::<EngineState>()
                .db
                .lock()
                .delete_clipboard_item(id)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
    }
    let _ = app.emit("clipboard-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn clear_clipboard_history(app: AppHandle) -> Result<(), String> {
    app.state::<EngineState>().clipboard.items.lock().clear();
    let db_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        db_app
            .state::<EngineState>()
            .db
            .lock()
            .clear_clipboard_history()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    let _ = app.emit("clipboard-updated", ());
    Ok(())
}

fn capture_update(app: &AppHandle) {
    let settings = app.state::<EngineState>().settings.lock().clone();
    if !settings.clipboard_enabled || settings.clipboard_paused {
        return;
    }
    let Some(text) = read_text() else {
        return;
    };
    let text: String = text.chars().take(MAX_CHARS).collect();
    if text.trim().is_empty() {
        return;
    }

    let state = app.state::<EngineState>();
    {
        let mut suppressed = state.clipboard.suppress_next.lock();
        if suppressed.as_deref() == Some(text.as_str()) {
            *suppressed = None;
            return;
        }
    }
    let item = {
        let mut items = state.clipboard.items.lock();
        if items.front().is_some_and(|item| item.text == text) {
            return;
        }
        let item = ClipboardItem {
            id: state.clipboard.next_id(),
            text,
            copied_at: now_seconds(),
        };
        items.push_front(item.clone());
        items.truncate(valid_limit(settings.clipboard_limit));
        item
    };

    if settings.clipboard_persist {
        let _ = state
            .db
            .lock()
            .save_clipboard_item(&item, valid_limit(settings.clipboard_limit));
    }
    let _ = app.emit("clipboard-updated", ());
}

fn read_text() -> Option<String> {
    for attempt in 0..4 {
        if unsafe { OpenClipboard(HWND(std::ptr::null_mut())) }.is_ok() {
            let text = unsafe { read_open_clipboard() };
            unsafe {
                let _ = CloseClipboard();
            }
            return text;
        }
        thread::sleep(Duration::from_millis(8 + attempt * 8));
    }
    None
}

unsafe fn read_open_clipboard() -> Option<String> {
    IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).ok()?;
    let handle = GetClipboardData(CF_UNICODETEXT.0 as u32).ok()?;
    let memory = HGLOBAL(handle.0);
    let size = GlobalSize(memory);
    if size < 2 {
        return None;
    }
    let pointer = GlobalLock(memory) as *const u16;
    if pointer.is_null() {
        return None;
    }
    let units = std::slice::from_raw_parts(pointer, size / 2);
    let length = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    let text = String::from_utf16_lossy(&units[..length]);
    let _ = GlobalUnlock(memory);
    Some(text)
}

fn write_text(text: &str) -> Result<(), String> {
    let wide: Vec<u16> = text.encode_utf16().chain(iter::once(0)).collect();
    for attempt in 0..4 {
        if unsafe { OpenClipboard(HWND(std::ptr::null_mut())) }.is_ok() {
            let result = unsafe { write_open_clipboard(&wide) };
            unsafe {
                let _ = CloseClipboard();
            }
            return result;
        }
        thread::sleep(Duration::from_millis(8 + attempt * 8));
    }
    Err("i18n:backend.clipboardBusy".into())
}

unsafe fn write_open_clipboard(wide: &[u16]) -> Result<(), String> {
    EmptyClipboard().map_err(|error| error.to_string())?;
    let bytes = wide.len() * std::mem::size_of::<u16>();
    let memory = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|error| error.to_string())?;
    let pointer = GlobalLock(memory) as *mut u16;
    if pointer.is_null() {
        let _ = GlobalFree(memory);
        return Err("i18n:backend.clipboardMemory".into());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), pointer, wide.len());
    let _ = GlobalUnlock(memory);
    if let Err(error) = SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(memory.0)) {
        let _ = GlobalFree(memory);
        return Err(error.to_string());
    }
    Ok(())
}

fn message_loop() {
    unsafe {
        let class_name = wide("SeekDash_CLIPBOARD_LISTENER");
        let module = match GetModuleHandleW(None) {
            Ok(module) => module,
            Err(_) => return,
        };
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
        let window = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            None,
        ) {
            Ok(window) => window,
            Err(_) => return,
        };
        if AddClipboardFormatListener(window).is_err() {
            return;
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = RemoveClipboardFormatListener(window);
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_CLIPBOARDUPDATE {
        if let Some(sender) = CLIPBOARD_EVENTS.get() {
            let _ = sender.try_send(());
        }
        return LRESULT(0);
    }
    DefWindowProcW(window, message, wparam, lparam)
}

fn valid_limit(limit: usize) -> usize {
    limit.clamp(1, MAX_ITEMS)
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_and_search_are_bounded() {
        assert_eq!(valid_limit(0), 1);
        assert_eq!(valid_limit(250), 100);
        assert_eq!(valid_limit(50), 50);
    }
}
