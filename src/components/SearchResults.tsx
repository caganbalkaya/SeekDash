import type { SearchResult } from "../types";
import { ResultItem } from "./ResultItem";
import { useI18n } from "../i18n";
import { Copy, ExternalLink, FolderOpen, Play, Settings2, Star } from "lucide-react";
import { useState } from "react";
import { ContextMenu, type ContextMenuItem } from "./ContextMenu";

interface Props {
  results: SearchResult[];
  selected: number;
  loading: boolean;
  onSelect(index: number): void;
  onOpen(index: number): void;
  onFavorite(index: number): void;
  onAction(index: number, action: "reveal" | "copy_path" | "properties" | "runas"): void;
  emptyQuery?: boolean;
}

export function SearchResults({ results, selected, loading, onSelect, onOpen, onFavorite, onAction, emptyQuery = false }: Props) {
  const { t } = useI18n();
  const [context, setContext] = useState<{ index:number; x:number; y:number }>();
  if (!results.length) {
    return loading ? <div className="empty-state">{t("search.searching")}</div> : null;
  }
  const contextResult = context ? results[context.index] : undefined;
  const physical = contextResult && !["calculator","web_search"].includes(contextResult.kind);
  const lowerPath = contextResult?.path.toLowerCase() ?? "";
  const canElevate = !!contextResult && ["application","shortcut"].includes(contextResult.kind) && (lowerPath.endsWith(".exe") || lowerPath.endsWith(".lnk"));
  const menuItems: ContextMenuItem[] = contextResult ? [
    { id:"open", label:t("context.open"), icon:Play, onSelect:()=>onOpen(context!.index) },
    ...(physical ? [
      { id:"reveal", label:t("context.reveal"), icon:FolderOpen, onSelect:()=>onAction(context!.index,"reveal") },
      { id:"copy", label:t("context.copyPath"), icon:Copy, onSelect:()=>onAction(context!.index,"copy_path") },
      { id:"properties", label:t("context.properties"), icon:Settings2, onSelect:()=>onAction(context!.index,"properties") },
      { id:"favorite", label:t(contextResult.favorite?"context.favoriteRemove":"context.favoriteAdd"), icon:Star, separator:true, onSelect:()=>onFavorite(context!.index) },
    ] as ContextMenuItem[] : []),
    ...(canElevate ? [{ id:"runas", label:t("context.runAsAdmin"), icon:ExternalLink, separator:true, onSelect:()=>onAction(context!.index,"runas") }] : []),
  ] : [];
  return (
    <div className="results" role="listbox" aria-label={t("search.results")}>
      {emptyQuery && <div className="results-context">{t("search.recents")}</div>}
      {results.map((result, index) => (
        <ResultItem key={result.path} result={result} selected={index === selected}
          loadNativeIcon={result.kind === "application" || result.kind === "shortcut"}
          animationDelayMs={Math.min(index, 7) * 9}
          onSelect={() => onSelect(index)} onOpen={() => onOpen(index)}
          onFavorite={() => onFavorite(index)} onContextMenu={(event) => { event.preventDefault(); event.stopPropagation(); setContext({index,x:event.clientX,y:event.clientY}); }} />
      ))}
      {context&&contextResult&&<ContextMenu x={context.x} y={context.y} items={menuItems} onClose={()=>setContext(undefined)}/>}
    </div>
  );
}
