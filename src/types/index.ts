export type ResultKind = "application" | "file" | "folder" | "shortcut" | "calculator" | "web_search";

export interface SearchResult {
  id: string;
  title: string;
  subtitle: string;
  path: string;
  kind: ResultKind;
  score: number;
  icon?: string | null;
  favorite: boolean;
}

export interface IndexStatus {
  running: boolean;
  scanned: number;
  indexed: number;
  errors: number;
  message: string;
}

export interface AppSettings {
  language: "en" | "tr" | "de" | "fr";
  hotkey: string;
  launchAtStartup: boolean;
  indexedDirectories: string[];
  excludedDirectories: string[];
  reducedMotion: boolean;
  theme: "dark" | "light" | "system";
  layoutMode: "horizontal" | "vertical";
  sidebarEdge: "left" | "right";
  sidebarAutoHide: boolean;
  clipboardEnabled: boolean;
  clipboardPersist: boolean;
  clipboardPaused: boolean;
  clipboardLimit: number;
  clipboardHotkey: string;
}

export interface ClipboardItem {
  id: number;
  text: string;
  copiedAt: number;
}

export type SidebarState = "hidden" | "collapsed" | "expanded";

export interface BackdropStatus {
  mode: "transparent_webview_css_clip" | "dwm_rounded_css_clip" | "rounded_native_region" | "transparent_window" | "css_fallback";
  native: boolean;
  detail: string;
}
