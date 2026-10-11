import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Clipboard, Copy, Pause, Search, Settings2, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { AppSettings, ClipboardItem } from "../types";
import { useI18n } from "../i18n";
import { ContextMenu } from "./ContextMenu";

interface Props {
  onClose(): void;
  onSettings(): void;
}

export function ClipboardPanel({ onClose, onSettings }: Props) {
  const { t, formatDate } = useI18n();
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [selected, setSelected] = useState(0);
  const [context, setContext] = useState<{ item:ClipboardItem; x:number; y:number }>();
  const input = useRef<HTMLInputElement>(null);
  const queryRef = useRef("");
  queryRef.current = query;

  const load = useCallback(async (value: string) => {
    const next = await invoke<ClipboardItem[]>("search_clipboard", { query: value });
    setItems(next);
  }, []);

  useEffect(() => {
    input.current?.focus();
    void invoke<AppSettings>("get_settings").then(setSettings);
    void load("");
    let cleanup: (() => void) | undefined;
    let disposed = false;
    void listen("clipboard-updated", () => void load(queryRef.current)).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    return () => { disposed = true; cleanup?.(); };
  }, [load]);

  useEffect(() => {
    setSelected(0);
    void load(query);
  }, [load, query]);

  const copy = async (item: ClipboardItem | undefined) => {
    if (!item) return;
    await invoke("copy_clipboard_item", { id: item.id });
    onClose();
  };

  const remove = async (item: ClipboardItem | undefined) => {
    if (!item) return;
    await invoke("delete_clipboard_item", { id: item.id });
    await load(query);
    setSelected((value) => Math.max(0, Math.min(value, items.length - 2)));
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") { event.preventDefault(); onClose(); }
    else if (event.key === "ArrowDown") { event.preventDefault(); setSelected((value) => Math.min(value + 1, items.length - 1)); }
    else if (event.key === "ArrowUp") { event.preventDefault(); setSelected((value) => Math.max(value - 1, 0)); }
    else if (event.key === "Enter") { event.preventDefault(); void copy(items[selected]); }
    else if (event.key === "Delete") { event.preventDefault(); void remove(items[selected]); }
  };

  return <div className="clipboard-panel" onKeyDown={onKeyDown}>
    <div className="clipboard-search-row">
      <Clipboard size={21} strokeWidth={1.7}/>
      <div className="clipboard-search-box">
        <Search size={17}/>
        <input ref={input} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("clipboard.search")} autoComplete="off" spellCheck={false}/>
        {query && <button onClick={() => setQuery("")} aria-label={t("clipboard.clearSearch")}><X size={14}/></button>}
      </div>
      <button className="settings-button" onClick={onSettings} aria-label={t("clipboard.openSettings")}><Settings2 size={17}/></button>
      <kbd>ESC</kbd>
    </div>

    {settings?.clipboardPaused && <div className="clipboard-state"><Pause size={14}/> {t("clipboard.paused")}</div>}
    <div className="clipboard-list" role="listbox" aria-label={t("clipboard.history")}>
      {!settings?.clipboardEnabled ? <div className="clipboard-empty"><Clipboard size={24}/><strong>{t("clipboard.disabled")}</strong><span>{t("clipboard.enableHint")}</span></div>
        : !items.length ? <div className="clipboard-empty"><Clipboard size={24}/><strong>{t(query ? "clipboard.noMatch" : "clipboard.empty")}</strong><span>{t(query ? "clipboard.tryAnother" : "clipboard.emptyHint")}</span></div>
        : items.map((item, index) => <div key={item.id} className={`clipboard-item${index === selected ? " selected" : ""}`} role="option" aria-selected={index === selected} onMouseMove={() => setSelected(index)} onClick={() => setSelected(index)} onDoubleClick={() => void copy(item)} onContextMenu={(event)=>{event.preventDefault();event.stopPropagation();setContext({item,x:event.clientX,y:event.clientY});}}>
          <div><p>{item.text}</p><time>{formatDate(new Date(item.copiedAt * 1000))}</time></div>
          <button onClick={(event) => { event.stopPropagation(); void remove(item); }} aria-label={t("clipboard.delete")} title={t("common.delete")}><Trash2 size={15}/></button>
        </div>)}
    </div>
    {context&&<ContextMenu x={context.x} y={context.y} onClose={()=>setContext(undefined)} items={[{id:"copy",label:t("context.copy"),icon:Copy,onSelect:()=>void copy(context.item)},{id:"delete",label:t("context.delete"),icon:Trash2,danger:true,separator:true,onSelect:()=>void remove(context.item)}]}/>}
    {items.length > 0 && <footer className="clipboard-footer"><span><kbd>↑</kbd><kbd>↓</kbd> {t("common.navigate")}</span><span><kbd>↵</kbd> {t("common.copy")}</span><span><kbd>DEL</kbd> {t("common.delete")}</span></footer>}
  </div>;
}
