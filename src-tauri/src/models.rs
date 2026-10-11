use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Application,
    File,
    Folder,
    Shortcut,
    Calculator,
    WebSearch,
}

impl ResultKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::File => "file",
            Self::Folder => "folder",
            Self::Shortcut => "shortcut",
            Self::Calculator => "calculator",
            Self::WebSearch => "web_search",
        }
    }
    pub fn from_db(value: &str) -> Self {
        match value {
            "application" => Self::Application,
            "folder" => Self::Folder,
            "shortcut" => Self::Shortcut,
            _ => Self::File,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub path: String,
    pub kind: ResultKind,
    pub score: f64,
    pub icon: Option<String>,
    #[serde(default)]
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: i64,
    pub text: String,
    pub copied_at: i64,
}

#[derive(Debug, Clone)]
pub struct IndexedItem {
    pub name: String,
    pub path: String,
    pub parent: String,
    pub extension: String,
    pub kind: ResultKind,
    pub modified: i64,
    pub normalized_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub hotkey: String,
    pub launch_at_startup: bool,
    pub indexed_directories: Vec<String>,
    pub excluded_directories: Vec<String>,
    pub reduced_motion: bool,
    pub theme: String,
    pub layout_mode: String,
    pub sidebar_edge: String,
    pub sidebar_auto_hide: bool,
    pub clipboard_enabled: bool,
    pub clipboard_persist: bool,
    pub clipboard_paused: bool,
    pub clipboard_limit: usize,
    pub clipboard_hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let dirs = [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Videos",
            "Music",
        ]
        .into_iter()
        .map(|name| format!(r"{}\{}", home, name))
        .filter(|p| std::path::Path::new(p).exists())
        .collect();
        Self {
            language: String::new(),
            hotkey: "Alt+Space".into(),
            launch_at_startup: false,
            indexed_directories: dirs,
            excluded_directories: vec!["node_modules".into(), ".git".into(), "$Recycle.Bin".into()],
            reduced_motion: false,
            theme: "dark".into(),
            layout_mode: "horizontal".into(),
            sidebar_edge: "left".into(),
            sidebar_auto_hide: false,
            clipboard_enabled: false,
            clipboard_persist: false,
            clipboard_paused: false,
            clipboard_limit: 100,
            clipboard_hotkey: "Alt+V".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn older_settings_receive_layout_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"hotkey":"Alt+Space","launchAtStartup":false,"indexedDirectories":[],"excludedDirectories":[],"reducedMotion":false,"theme":"dark"}"#,
        )
        .unwrap();
        assert_eq!(settings.layout_mode, "horizontal");
        assert_eq!(settings.sidebar_edge, "left");
        assert!(!settings.sidebar_auto_hide);
        assert!(!settings.clipboard_enabled);
        assert!(!settings.clipboard_persist);
        assert_eq!(settings.clipboard_limit, 100);
        assert_eq!(settings.clipboard_hotkey, "Alt+V");
        assert!(settings.language.is_empty());
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub running: bool,
    pub scanned: u64,
    pub indexed: u64,
    pub errors: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackdropStatus {
    pub mode: String,
    pub native: bool,
    pub detail: String,
}
