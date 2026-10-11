import { invoke } from "@tauri-apps/api/core";
import { AppWindow, ChevronsRight, Copy, ExternalLink, EyeOff, File, Folder, FolderOpen, Play, Search, Settings2, Star } from "lucide-react";
import { useEffect, useState } from "react";
import type { SearchResult } from "../types";
import { useI18n } from "../i18n";
import { ContextMenu, type ContextMenuItem } from "./ContextMenu";

function DockIcon({ result }: { result: SearchResult }) {
  const [icon, setIcon] = useState(result.icon);
  useEffect(() => {
    if (icon || !["application", "shortcut"].includes(result.kind)) return;
    let active = true;
    void invoke<string | null>("get_icon", { path: result.path }).then((value) => { if (active && value) setIcon(value); }).catch(() => {});
    return () => { active = false; };
  }, [icon, result.kind, result.path]);
  if (icon) return <img src={icon} alt=""/>;
  const Icon = result.kind === "folder" ? Folder : result.kind === "application" || result.kind === "shortcut" ? AppWindow : File;
  return <Icon size={19} strokeWidth={1.6}/>;
}

interface Props {
  favorites: SearchResult[];
  edge: "left" | "right";
  onExpand(): void;
  onHide(): void;
  onSettings(): void;
  onOpen(result: SearchResult): void;
  onAction(result: SearchResult, action:"reveal"|"copy_path"|"properties"|"runas"): void;
  onFavorite(result: SearchResult): void;
}

export function SidebarDock({ favorites, edge, onExpand, onHide, onSettings, onOpen, onAction, onFavorite }: Props) {
  const { t } = useI18n();
  const [context,setContext]=useState<{result:SearchResult;x:number;y:number}>();
  const lower=context?.result.path.toLowerCase()??"";
  const canElevate=!!context&&["application","shortcut"].includes(context.result.kind)&&(lower.endsWith(".exe")||lower.endsWith(".lnk"));
  const items:ContextMenuItem[]=context?[
    {id:"open",label:t("context.open"),icon:Play,onSelect:()=>onOpen(context.result)},
    {id:"reveal",label:t("context.reveal"),icon:FolderOpen,onSelect:()=>onAction(context.result,"reveal")},
    {id:"copy",label:t("context.copyPath"),icon:Copy,onSelect:()=>onAction(context.result,"copy_path")},
    {id:"properties",label:t("context.properties"),icon:Settings2,onSelect:()=>onAction(context.result,"properties")},
    {id:"favorite",label:t("context.favoriteRemove"),icon:Star,separator:true,onSelect:()=>onFavorite(context.result)},
    ...(canElevate?[{id:"runas",label:t("context.runAsAdmin"),icon:ExternalLink,separator:true,onSelect:()=>onAction(context.result,"runas")}]:[]),
  ]:[];
  return <div className="sidebar-dock">
    <button className="dock-search" onClick={onExpand} aria-label={t("sidebar.search")}><Search size={20}/></button>
    <div className="dock-divider"/>
    <div className="dock-favorites">
      {favorites.slice(0, 5).map((result) => <button key={result.path} onClick={() => onOpen(result)} onContextMenu={(event)=>{event.preventDefault();event.stopPropagation();setContext({result,x:event.clientX,y:event.clientY});}} title={result.title} aria-label={result.title}><DockIcon result={result}/></button>)}
    </div>
    <div className="dock-spacer"/>
    <button onClick={onSettings} aria-label={t("common.settings")}><Settings2 size={18}/></button>
    <button onClick={onHide} aria-label={t("sidebar.hide")}><EyeOff size={17}/></button>
    <button className="dock-expand" onClick={onExpand} aria-label={t("sidebar.expand")}><ChevronsRight className={edge === "right" ? "flip" : ""} size={18}/></button>
    {context&&<ContextMenu x={context.x} y={context.y} items={items} onClose={()=>setContext(undefined)}/>}
  </div>;
}
