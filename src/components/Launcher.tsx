import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLauncher } from "../hooks/useLauncher";
import type { AppSettings } from "../types";
import { commandArgument, filterCommands, hexToRgb, type CommandDefinition } from "../commandPalette";
import { GlassSurface } from "./GlassSurface";
import { CommandResults } from "./CommandResults";
import { SearchInput } from "./SearchInput";
import { SearchResults } from "./SearchResults";
import { SidebarDock } from "./SidebarDock";
import { ClipboardPanel } from "./ClipboardPanel";
import type { SidebarState } from "../types";
import { useI18n } from "../i18n";

const isTauri = () => "__TAURI_INTERNALS__" in window;

export function Launcher() {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [action, setAction] = useState<"open" | "reveal" | "properties">("open");
  const [closing, setClosing] = useState(false);
  const [opening, setOpening] = useState(true);
  const [layoutMode, setLayoutMode] = useState<AppSettings["layoutMode"]>("horizontal");
  const [sidebarEdge, setSidebarEdge] = useState<AppSettings["sidebarEdge"]>("left");
  const [sidebarState, setSidebarState] = useState<SidebarState>("expanded");
  const [view, setView] = useState<"search" | "clipboard">("search");
  const [commandMessage, setCommandMessage] = useState("");
  const [confirming, setConfirming] = useState<CommandDefinition>();
  const [ipAddresses, setIpAddresses] = useState<string[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const windowHeight = useRef(82);
  const { results, loading, refresh } = useLauncher(query);
  const commandMode = query.startsWith(">");
  const commands = useMemo(() => filterCommands(query), [query]);
  const paletteLength = ipAddresses.length || commands.length;
  const colorPreview = commands[0]?.id === "color" ? hexToRgb(commandArgument(query)) : null;

  useEffect(() => {
    if (!isTauri()) return;
    let cleanup: (() => void) | undefined;
    let disposed = false;
    let cleanupSidebar: (() => void) | undefined;
    void invoke<AppSettings>("get_settings").then((value) => { setLayoutMode(value.layoutMode); setSidebarEdge(value.sidebarEdge); });
    void invoke<SidebarState>("get_sidebar_state").then(setSidebarState);
    void listen<AppSettings>("settings-updated", ({ payload }) => { setLayoutMode(payload.layoutMode); setSidebarEdge(payload.sidebarEdge); }).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    void listen<SidebarState>("sidebar-state-changed", ({ payload }) => setSidebarState(payload)).then((fn) => {
      if (disposed) fn(); else cleanupSidebar = fn;
    });
    return () => { disposed = true; cleanup?.(); cleanupSidebar?.(); };
  }, []);

  useEffect(() => { setSelected(0); setAction("open"); setCommandMessage(""); setConfirming(undefined); setIpAddresses([]); }, [query, results.length]);
  useEffect(() => {
    if (!isTauri()) return;
    if (layoutMode === "vertical" || view === "clipboard") return;
    const hasQuery = query.trim().length > 0;
    const visibleRows = Math.min(commandMode ? paletteLength : results.length, 6);
    const height = hasQuery ? 560 : visibleRows ? Math.min(560, 80 + 10 + visibleRows * 64 + 34) : 82;
    if (windowHeight.current === height) return;
    windowHeight.current = height;
    void getCurrentWindow().setSize(new LogicalSize(720, height));
  }, [query, results.length, layoutMode, view, commandMode, paletteLength]);
  useEffect(() => {
    if (!isTauri()) return;
    let cleanup: (() => void) | undefined;
    let cleanupClosed: (() => void) | undefined;
    let disposed = false;
    void listen("launcher-opened", () => {
      setView("search");
      setSidebarState("expanded");
      setClosing(false);
      setOpening(false);
      window.requestAnimationFrame(() => setOpening(true));
      window.setTimeout(() => setOpening(false), 190);
      setQuery("");
      window.requestAnimationFrame(() => input.current?.focus());
    }).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    void listen("clipboard-closed", () => setView("search")).then((fn) => {
      if (disposed) fn(); else cleanupClosed = fn;
    });
    return () => { disposed = true; cleanup?.(); cleanupClosed?.(); };
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let cleanup: (() => void) | undefined;
    let disposed = false;
    void listen("clipboard-opened", () => {
      setView("clipboard");
      setSidebarState("expanded");
      setClosing(false);
      setOpening(false);
    }).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    return () => { disposed = true; cleanup?.(); };
  }, []);

  const hide = async () => {
    if (layoutMode === "vertical") {
      setQuery("");
      await invoke("set_sidebar_state", { state: "collapsed" });
      return;
    }
    setClosing(true);
    await new Promise((resolve) => window.setTimeout(resolve, 105));
    if (isTauri()) await invoke("hide_launcher");
    setQuery("");
  };

  const open = async (index: number, action: "open" | "reveal" | "properties" | "runas" = "open") => {
    const result = results[index];
    if (!result || !isTauri()) return;
    try {
      await invoke("launch_result", { result, action });
      if (action === "open" || action === "runas") {
        setQuery("");
        if (layoutMode === "vertical") await invoke("set_sidebar_state", { state: "collapsed" });
        else await invoke("hide_launcher");
      }
    } catch (error) {
      console.error("Open failed", error);
      finishCommand(String(error));
    }
  };

  const openDockResult = async (result: typeof results[number]) => {
    if (!isTauri()) return;
    try {
      await invoke("launch_result", { result, action: "open" });
    } catch (error) {
      console.error("Open failed", error);
      finishCommand(String(error));
    }
  };

  const dockContextAction = async (result:typeof results[number], action:"reveal"|"copy_path"|"properties"|"runas") => {
    if(!isTauri())return;
    if(action==="copy_path"){await invoke("copy_path",{path:result.path});return;}
    await invoke("launch_result",{result,action});
    if(action==="runas")void invoke("set_sidebar_state",{state:"collapsed"});
  };

  const dockFavorite = async (result:typeof results[number]) => { if(!isTauri())return;await invoke("set_favorite",{path:result.path,favorite:!result.favorite});await refresh(); };

  const toggleFavorite = async (index: number) => {
    const result = results[index];
    if (!result || !isTauri()) return;
    await invoke("set_favorite", { path: result.path, favorite: !result.favorite });
    await refresh();
  };

  const contextAction = async (index:number, action:"reveal"|"copy_path"|"properties"|"runas") => {
    const result=results[index];
    if(!result||!isTauri())return;
    if(action==="copy_path") { try { await invoke("copy_path",{path:result.path}); finishCommand(t("backend.copied")); } catch(error){ finishCommand(String(error)); } return; }
    await open(index,action);
  };

  const finishCommand = (message: string) => {
    setCommandMessage(message);
    window.setTimeout(() => setCommandMessage(""), 1800);
  };

  const runCommand = async (command: CommandDefinition, confirmed = false) => {
    if (!isTauri()) return;
    if (command.destructive && !confirmed) { setConfirming(command); return; }
    if (command.id === "settings") {
      await invoke("show_settings").catch((error) => finishCommand(String(error)));
      return;
    }
    const argument = command.id === "color" ? commandArgument(query) : undefined;
    if (command.id === "color" && !hexToRgb(argument ?? "")) {
      finishCommand(t("command.invalidColor"));
      return;
    }
    setConfirming(undefined);
    try {
      const response = await invoke<{ message: string; values: string[] }>("execute_quick_action", { action: command.id, argument });
      if (response.values.length) { setIpAddresses(response.values); setSelected(0); }
      finishCommand(response.message);
      if (["lock", "sleep", "restart", "shutdown", "recycle"].includes(command.id)) void hide();
    } catch (error) {
      finishCommand(String(error));
    }
  };

  const runPaletteIndex = async (index: number) => {
    if (ipAddresses.length) {
      const value = ipAddresses[index];
      if (!value || !isTauri()) return;
      try {
        const response = await invoke<{ message: string; values: string[] }>("copy_command_value", { value });
        finishCommand(response.message);
      } catch (error) { finishCommand(String(error)); }
      return;
    }
    const command = commands[index];
    if (command) await runCommand(command);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") { event.preventDefault(); void hide(); }
    else if (event.key === "ArrowDown") { event.preventDefault(); setSelected((v) => Math.min(v + 1, (commandMode ? paletteLength : results.length) - 1)); }
    else if (event.key === "ArrowUp") { event.preventDefault(); setSelected((v) => Math.max(v - 1, 0)); }
    else if (event.key === "Tab" && !commandMode) {
      event.preventDefault();
      const actions = ["open", "reveal", "properties"] as const;
      setAction((current) => actions[(actions.indexOf(current) + (event.shiftKey ? 2 : 1)) % actions.length]);
    }
    else if (event.key === "Enter") {
      event.preventDefault();
      if (confirming) void runCommand(confirming, true);
      else if (commandMode) void runPaletteIndex(selected);
      else void open(selected, event.ctrlKey ? "reveal" : event.altKey ? "properties" : action);
    }
  };

  const closeClipboard = () => {
    setView("search");
    if (layoutMode === "vertical") void invoke("set_sidebar_state", { state: "collapsed" });
    else void invoke("hide_launcher");
  };

  if (view === "clipboard") {
    return <div className={`launcher-stage layout-${layoutMode} sidebar-expanded edge-${sidebarEdge}`} onContextMenu={(event)=>event.preventDefault()}>
      <GlassSurface><ClipboardPanel onClose={closeClipboard} onSettings={() => { setView("search"); void invoke("show_settings"); }}/></GlassSurface>
    </div>;
  }

  if (layoutMode === "vertical" && sidebarState === "hidden") return null;

  if (layoutMode === "vertical" && sidebarState === "collapsed") {
    return <div className={`launcher-stage layout-vertical is-collapsed edge-${sidebarEdge}`} onContextMenu={(event)=>event.preventDefault()}>
      <GlassSurface><SidebarDock favorites={results.filter((item) => item.favorite)} edge={sidebarEdge} onExpand={() => void invoke("set_sidebar_state", { state: "expanded" })} onHide={() => void invoke("set_sidebar_state", { state: "hidden" })} onSettings={() => void invoke("show_settings")} onOpen={openDockResult} onAction={(result,name)=>void dockContextAction(result,name)} onFavorite={(result)=>void dockFavorite(result)}/></GlassSurface>
    </div>;
  }

  return (
    <div className={`launcher-stage layout-${layoutMode} sidebar-${sidebarState} edge-${sidebarEdge}${opening ? " is-opening" : ""}${closing ? " is-closing" : ""}`} onKeyDown={onKeyDown} onContextMenu={(event)=>event.preventDefault()}>
      <GlassSurface>
        <SearchInput ref={input} value={query} onChange={setQuery} onClear={() => setQuery("")} onSettings={() => void invoke("show_settings")} onCollapse={layoutMode === "vertical" ? () => void invoke("set_sidebar_state", { state: "collapsed" }) : undefined} sidebarEdge={sidebarEdge}/>
        {commandMode
          ? <CommandResults commands={commands} addresses={ipAddresses} selected={selected} message={commandMessage} confirming={confirming} colorPreview={colorPreview} onSelect={setSelected} onRun={(i) => void runPaletteIndex(i)} onConfirm={() => confirming && void runCommand(confirming, true)} onCancel={() => setConfirming(undefined)}/>
          : <SearchResults results={results} selected={selected} loading={loading} emptyQuery={layoutMode === "vertical" && !query.trim()} onSelect={setSelected} onOpen={(i) => void open(i)} onFavorite={(i) => void toggleFavorite(i)} onAction={(i,contextActionName)=>void contextAction(i,contextActionName)} />}
        {(commandMode ? paletteLength > 0 : results.length > 0) && <footer><span><kbd>↑</kbd><kbd>↓</kbd> {t("common.navigate")}</span>{commandMode ? <span className="active-action"><kbd>↵</kbd> {t("command.run")}</span> : <><span className={action === "open" ? "active-action" : ""}><kbd>↵</kbd> {t("common.open")}</span><span className={action === "reveal" ? "active-action" : ""}><kbd>⌃↵</kbd> {t("search.location")}</span><span className={action === "properties" ? "active-action" : ""}><kbd>⌥↵</kbd> {t("search.properties")}</span></>}</footer>}
      </GlassSurface>
    </div>
  );
}
