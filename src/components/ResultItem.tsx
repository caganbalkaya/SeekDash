import {
  AppWindow, Box, Calculator, Database, File, FileArchive, FileAudio, FileCode2, FileCog,
  FileImage, FileJson, FileSpreadsheet, FileText, FileType2, FileVideo, Folder,
  Globe2, Presentation, SquareArrowOutUpRight, SquareTerminal, Star,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { memo } from "react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SearchResult } from "../types";
import { translateTagged, useI18n } from "../i18n";

const fallbackIcons = {
  application: AppWindow,
  file: File,
  folder: Folder,
  shortcut: SquareArrowOutUpRight,
  calculator: Calculator,
  web_search: Globe2,
};

const extensionIcons: Array<[Set<string>, LucideIcon, string]> = [
  [new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "tif", "tiff", "heic", "avif", "raw"]), FileImage, "image"],
  [new Set(["mp4", "mkv", "mov", "avi", "webm", "wmv", "m4v", "mpeg", "mpg"]), FileVideo, "video"],
  [new Set(["mp3", "wav", "flac", "aac", "m4a", "ogg", "wma"]), FileAudio, "audio"],
  [new Set(["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "cab"]), FileArchive, "archive"],
  [new Set(["xls", "xlsx", "xlsm", "csv", "tsv", "ods"]), FileSpreadsheet, "sheet"],
  [new Set(["ppt", "pptx", "pps", "ppsx", "odp"]), Presentation, "presentation"],
  [new Set(["js", "jsx", "ts", "tsx", "html", "htm", "css", "scss", "less", "rs", "py", "java", "cs", "c", "cpp", "h", "hpp", "go", "php", "rb", "swift", "kt", "vue", "svelte"]), FileCode2, "code"],
  [new Set(["ps1", "bat", "cmd", "sh", "bash", "zsh"]), SquareTerminal, "script"],
  [new Set(["json", "jsonc", "xml", "yaml", "yml", "toml"]), FileJson, "data"],
  [new Set(["db", "sqlite", "sqlite3", "sql", "mdb", "accdb"]), Database, "database"],
  [new Set(["ttf", "otf", "woff", "woff2", "eot"]), FileType2, "font"],
  [new Set(["stl", "obj", "fbx", "blend", "3ds", "dwg", "dxf"]), Box, "model"],
  [new Set(["ini", "cfg", "conf", "config", "reg"]), FileCog, "config"],
  [new Set(["txt", "md", "rtf", "pdf", "doc", "docx", "odt", "epub"]), FileText, "document"],
];

function iconFor(result: SearchResult): { Icon: LucideIcon; category: string } {
  if (result.kind === "file") {
    const name = result.path.split(/[\\/]/).pop() ?? "";
    const extension = name.includes(".") ? name.split(".").pop()!.toLowerCase() : "";
    const match = extensionIcons.find(([extensions]) => extensions.has(extension));
    if (match) return { Icon: match[1], category: match[2] };
  }
  if (result.kind === "shortcut") return { Icon: SquareArrowOutUpRight, category: "shortcut" };
  return { Icon: fallbackIcons[result.kind], category: result.kind };
}

interface Props {
  result: SearchResult;
  selected: boolean;
  loadNativeIcon?: boolean;
  animationDelayMs?: number;
  onSelect(): void;
  onOpen(): void;
  onFavorite(): void;
  onContextMenu(event: React.MouseEvent): void;
}

export const ResultItem = memo(function ResultItem({ result, selected, loadNativeIcon = false, animationDelayMs = 0, onSelect, onOpen, onFavorite, onContextMenu }: Props) {
  const { t } = useI18n();
  const { Icon, category } = iconFor(result);
  const [icon, setIcon] = useState(result.icon);
  useEffect(() => {
    setIcon(result.icon);
    if (!loadNativeIcon || result.icon || !("__TAURI_INTERNALS__" in window)) return;
    let active = true;
    void invoke<string | null>("get_icon", { path: result.path }).then((value) => { if (active && value) setIcon(value); }).catch(() => {});
    return () => { active = false; };
  }, [loadNativeIcon, result.icon, result.path]);
  return (
    <div
      className={`result-item${selected ? " selected" : ""}`}
      role="option"
      aria-selected={selected}
      style={{ animationDelay: `${animationDelayMs}ms` }}
      onMouseMove={onSelect}
      onClick={onSelect}
      onDoubleClick={onOpen}
      onContextMenu={onContextMenu}
    >
      <div className={`result-icon icon-${category}`} data-icon-category={category}>
        {icon ? <img src={icon} alt="" /> : <Icon size={22} strokeWidth={1.55} />}
      </div>
      <div className="result-copy">
        <div className="result-title">{translateTagged(t, result.title)}</div>
        <div className="result-subtitle">{translateTagged(t, result.subtitle)}</div>
      </div>
      <span className="result-kind">{t(`kind.${result.kind}`)}</span>
      {result.kind !== "calculator" && result.kind !== "web_search" && (
        <button
          type="button"
          className={`favorite-button${result.favorite ? " is-favorite" : ""}`}
          aria-label={result.favorite ? t("favorite.remove", { name: result.title }) : t("favorite.add", { name: result.title })}
          title={result.favorite ? t("favorite.remove", { name: result.title }) : t("favorite.add", { name: result.title })}
          onClick={(event) => { event.stopPropagation(); onFavorite(); }}
        >
          <Star size={16} strokeWidth={1.7} fill={result.favorite ? "currentColor" : "none"} />
        </button>
      )}
    </div>
  );
});
