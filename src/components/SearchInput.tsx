import { ChevronsLeft, Search, Settings2, X } from "lucide-react";
import { forwardRef } from "react";
import { useI18n } from "../i18n";

interface Props {
  value: string;
  onChange(value: string): void;
  onClear(): void;
  onSettings(): void;
  onCollapse?(): void;
  sidebarEdge?: "left" | "right";
}

export const SearchInput = forwardRef<HTMLInputElement, Props>(function SearchInput({ value, onChange, onClear, onSettings, onCollapse, sidebarEdge = "left" }, ref) {
  const { t } = useI18n();
  return (
    <div className="search-row">
      <Search className="search-icon" size={23} strokeWidth={1.75} aria-hidden="true" />
      <input
        ref={ref}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        autoFocus
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        aria-label={t("search.label")}
        placeholder={t("search.placeholder")}
      />
      {value && <button className="clear-button" onClick={onClear} aria-label={t("search.clear")}><X size={15} /></button>}
      <button className="settings-button" onClick={onSettings} aria-label={t("common.settings")} title={t("common.settings")}><Settings2 size={16} /></button>
      {onCollapse && <button className="sidebar-collapse-button" onClick={onCollapse} aria-label={t("sidebar.hide")} title={t("sidebar.hide")}><ChevronsLeft className={sidebarEdge === "right" ? "flip" : ""} size={17}/></button>}
      <kbd>ESC</kbd>
    </div>
  );
});
