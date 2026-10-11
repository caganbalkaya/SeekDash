import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Launcher } from "./components/Launcher";
import { Settings } from "./components/Settings";
import type { AppSettings } from "./types";

function applyAppearance(settings: AppSettings, prefersLight: boolean) {
  const resolved = settings.theme === "system" ? (prefersLight ? "light" : "dark") : settings.theme;
  document.documentElement.dataset.theme = settings.theme;
  document.documentElement.dataset.resolvedTheme = resolved;
  document.documentElement.dataset.layout = settings.layoutMode;
  document.documentElement.classList.toggle("reduce-motion", settings.reducedMotion);
}

export default function App() {
  const settingsWindow = new URLSearchParams(location.search).has("settings");
  const [settings, setSettings] = useState(settingsWindow);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    const media = window.matchMedia("(prefers-color-scheme: light)");
    let current: AppSettings | undefined;
    let cleanupSettings: (() => void) | undefined;
    let cleanupOpen: (() => void) | undefined;
    let disposed = false;
    const update = (next: AppSettings) => { current = next; applyAppearance(next, media.matches); };
    const onSystemTheme = () => { if (current) applyAppearance(current, media.matches); };
    media.addEventListener("change", onSystemTheme);
    void invoke<AppSettings>("get_settings").then(update);
    void listen<AppSettings>("settings-updated", ({ payload }) => update(payload)).then((fn) => {
      if (disposed) fn(); else cleanupSettings = fn;
    });
    void listen("open-settings", () => setSettings(true)).then((fn) => {
      if (disposed) fn(); else cleanupOpen = fn;
    });
    return () => {
      disposed = true;
      cleanupSettings?.();
      cleanupOpen?.();
      media.removeEventListener("change", onSystemTheme);
    };
  }, []);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window) || !settingsWindow) return;
    let cleanup: (() => void) | undefined;
    let disposed = false;
    void invoke("settings_webview_ready");
    void listen("settings-open-requested", () => {
      window.requestAnimationFrame(() => void invoke("settings_interactive"));
    }).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    return () => { disposed = true; cleanup?.(); };
  }, [settingsWindow]);

  const closeSettings = async () => {
    if (settingsWindow && "__TAURI_INTERNALS__" in window) await invoke("hide_settings");
    else setSettings(false);
  };
  return settings ? <Settings onClose={() => void closeSettings()} /> : <Launcher />;
}
