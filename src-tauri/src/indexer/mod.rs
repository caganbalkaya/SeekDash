use crate::{
    models::{AppSettings, IndexStatus, IndexedItem, ResultKind},
    search::normalize,
    state::EngineState,
};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::HashSet,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    time::{Duration, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

enum WatcherSignal {
    Paths(Vec<PathBuf>, bool),
    Rescan,
}

pub fn rebuild(app: AppHandle) {
    let state = app.state::<EngineState>();
    if state.status.lock().running {
        return;
    }
    let settings = state.settings.lock().clone();
    *state.status.lock() = IndexStatus {
        running: true,
        message: "i18n:index.indexing".into(),
        ..Default::default()
    };
    let _ = app.emit("index-status", state.status.lock().clone());
    tauri::async_runtime::spawn_blocking(move || run_rebuild(&app, &settings));
}

fn run_rebuild(app: &AppHandle, settings: &AppSettings) {
    let started = std::time::Instant::now();
    let state = app.state::<EngineState>();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut errors = 0u64;
    let mut next_progress = 1_000usize;
    let mut roots: Vec<PathBuf> = settings
        .indexed_directories
        .iter()
        .map(PathBuf::from)
        .collect();
    roots.extend(application_roots());

    for root in roots {
        if !root.exists() {
            continue;
        }
        let walker = WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_excluded(entry.path(), &settings.excluded_directories));
        for entry in walker {
            let entry = match entry {
                Ok(value) => value,
                Err(_) => {
                    errors += 1;
                    continue;
                }
            };
            if let Some(item) = item_from_path(entry.path()) {
                let key = item.path.to_lowercase();
                if seen.insert(key) {
                    items.push(item);
                }
            }
            if items.len() >= next_progress {
                let mut status = state.status.lock();
                status.scanned = items.len() as u64 + errors;
                status.indexed = items.len() as u64;
                status.errors = errors;
                let _ = app.emit("index-status", status.clone());
                next_progress += 1_000;
            }
        }
    }
    for item in discover_uwp_apps() {
        if seen.insert(item.path.to_lowercase()) {
            items.push(item);
        }
    }
    let result = state.db.lock().replace_all(&items);
    let elapsed = started.elapsed().as_secs_f64();
    let mut status = state.status.lock();
    status.running = false;
    status.scanned = items.len() as u64 + errors;
    status.indexed = items.len() as u64;
    status.errors = errors;
    status.message = match result {
        Ok(_) => format!(
            "i18n:index.complete|{}|{:.0}",
            items.len(),
            items.len() as f64 / elapsed.max(0.001)
        ),
        Err(error) => format!("i18n:index.error|{error}"),
    };
    let _ = app.emit("index-status", status.clone());
    let _ = app.emit("index-updated", ());
    drop(status);
    start_watcher(app.clone());
}

pub fn start_watcher(app: AppHandle) {
    let (sender, receiver) = mpsc::channel::<WatcherSignal>();
    let watcher_result =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event) if event.need_rescan() => {
                let _ = sender.send(WatcherSignal::Rescan);
            }
            Ok(event) => {
                let recursive = matches!(
                    event.kind,
                    notify::EventKind::Create(_)
                        | notify::EventKind::Modify(notify::event::ModifyKind::Name(_))
                );
                let _ = sender.send(WatcherSignal::Paths(event.paths, recursive));
            }
            Err(_) => {
                let _ = sender.send(WatcherSignal::Rescan);
            }
        });
    let Ok(mut watcher) = watcher_result else {
        return;
    };
    let worker_app = app.clone();
    let exclusions = app
        .state::<EngineState>()
        .settings
        .lock()
        .excluded_directories
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        while let Ok(signal) = receiver.recv() {
            let WatcherSignal::Paths(paths, recursive) = signal else {
                rebuild(worker_app.clone());
                break;
            };
            let mut pending: std::collections::HashMap<PathBuf, bool> =
                paths.into_iter().map(|path| (path, recursive)).collect();
            let mut rescan = false;
            while pending.len() < 2048 {
                match receiver.recv_timeout(Duration::from_millis(120)) {
                    Ok(WatcherSignal::Paths(more, recursive)) => {
                        for path in more {
                            pending
                                .entry(path)
                                .and_modify(|value| *value |= recursive)
                                .or_insert(recursive);
                        }
                    }
                    Ok(WatcherSignal::Rescan) => {
                        rescan = true;
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
            if rescan {
                rebuild(worker_app.clone());
                break;
            }
            let started = std::time::Instant::now();
            let mut upserts = Vec::new();
            let mut removals = Vec::new();
            for (path, recursive) in pending {
                if !path.exists() {
                    removals.push(path.to_string_lossy().into_owned());
                } else if recursive && path.is_dir() {
                    upserts.extend(
                        WalkDir::new(&path)
                            .follow_links(false)
                            .into_iter()
                            .filter_entry(|entry| !is_excluded(entry.path(), &exclusions))
                            .filter_map(Result::ok)
                            .filter_map(|entry| item_from_path(entry.path())),
                    );
                } else if let Some(item) = item_from_path(&path) {
                    upserts.push(item);
                }
            }
            {
                let state = worker_app.state::<EngineState>();
                let mut db = state.db.lock();
                if let Err(error) = db.apply_index_changes(&upserts, &removals) {
                    eprintln!("SeekDash watcher index update failed: {error}");
                    drop(db);
                    rebuild(worker_app.clone());
                    break;
                }
            }
            let elapsed = started.elapsed();
            if elapsed.as_millis() > 500 {
                eprintln!("SeekDash slow-operation: watcher batch took {elapsed:?}");
            }
            let _ = worker_app.emit("index-updated", ());
        }
    });
    let mut roots = app
        .state::<EngineState>()
        .settings
        .lock()
        .indexed_directories
        .clone();
    roots.extend(
        application_roots()
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned()),
    );
    let mut watched = HashSet::new();
    for root in roots {
        let path = PathBuf::from(root);
        if path.exists() && watched.insert(path.to_string_lossy().to_lowercase()) {
            let _ = watcher.watch(&path, RecursiveMode::Recursive);
        }
    }
    *app.state::<EngineState>().watcher.lock() = Some(watcher);
}

pub fn item_from_path(path: &Path) -> Option<IndexedItem> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() {
        return None;
    }
    let raw_name = path.file_name()?.to_string_lossy().into_owned();
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is_shortcut = matches!(extension.as_str(), "lnk" | "url");
    let is_application = matches!(extension.as_str(), "exe" | "appref-ms")
        || (is_shortcut && is_application_location(path));
    let kind = if metadata.is_dir() {
        ResultKind::Folder
    } else if is_application {
        ResultKind::Application
    } else if is_shortcut {
        ResultKind::Shortcut
    } else {
        ResultKind::File
    };
    let name = if is_shortcut {
        path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    } else {
        raw_name
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|v| v.as_secs() as i64)
        .unwrap_or(0);
    Some(IndexedItem {
        normalized_name: normalize(&name),
        name,
        path: path.to_string_lossy().into_owned(),
        parent: path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        extension,
        kind,
        modified,
    })
}

fn application_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots.push(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(programdata) = std::env::var("PROGRAMDATA") {
        roots.push(PathBuf::from(programdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(public) = std::env::var("PUBLIC") {
        roots.push(PathBuf::from(public).join("Desktop"));
    }
    roots
}

fn is_application_location(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    lower.contains("start menu") || lower.contains("\\desktop")
}
fn is_excluded(path: &Path, excluded: &[String]) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    excluded
        .iter()
        .any(|part| !part.is_empty() && lower.contains(&part.to_lowercase()))
}

fn discover_uwp_apps() -> Vec<IndexedItem> {
    #[derive(serde::Deserialize)]
    struct StartApp {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "AppID")]
        app_id: String,
    }
    let output = Command::new("powershell.exe")
        .creation_flags(0x08000000)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-StartApps | ConvertTo-Json -Compress",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();
    let apps: Vec<StartApp> = if trimmed.starts_with('[') {
        serde_json::from_str(trimmed).unwrap_or_default()
    } else {
        serde_json::from_str::<StartApp>(trimmed)
            .map(|a| vec![a])
            .unwrap_or_default()
    };
    apps.into_iter()
        .filter(|a| !a.name.trim().is_empty())
        .map(|app| IndexedItem {
            normalized_name: normalize(&app.name),
            name: app.name,
            path: format!(r"shell:AppsFolder\{}", app.app_id),
            parent: "i18n:search.windowsApps".into(),
            extension: String::new(),
            kind: ResultKind::Application,
            modified: 0,
        })
        .collect()
}
