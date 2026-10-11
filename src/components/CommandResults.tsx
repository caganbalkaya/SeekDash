import { Hash, LockKeyhole, Moon, Network, Palette, Power, RotateCw, Settings2, Trash2 } from "lucide-react";
import type { CommandDefinition } from "../commandPalette";
import { translateTagged, useI18n } from "../i18n";
import { useState } from "react";
import { ContextMenu } from "./ContextMenu";

const icons = { lock: LockKeyhole, moon: Moon, restart: RotateCw, power: Power, trash: Trash2, network: Network, hash: Hash, palette: Palette, settings: Settings2 };

interface Props {
  commands: CommandDefinition[];
  addresses: string[];
  selected: number;
  message: string;
  confirming?: CommandDefinition;
  colorPreview?: string | null;
  onSelect(index: number): void;
  onRun(index: number): void;
  onConfirm(): void;
  onCancel(): void;
}

export function CommandResults({ commands, addresses, selected, message, confirming, colorPreview, onSelect, onRun, onConfirm, onCancel }: Props) {
  const { t } = useI18n();
  const [context,setContext]=useState<{index:number;x:number;y:number}>();
  if (confirming) return <div className="command-confirm" role="alertdialog" aria-label={t("command.confirm")}>
    <div className="command-confirm-icon"><Power size={22}/></div>
    <div><strong>{t("command.confirmTitle", { name: t(confirming.titleKey) })}</strong><span>{t("command.confirmBody")}</span></div>
    <button onClick={onCancel}>{t("common.cancel")}</button><button className="danger" onClick={onConfirm}>{t("command.confirm")}</button>
  </div>;

  const rows = addresses.length ? addresses.map((address) => ({ key: address, title: address, description: t("command.copyIp"), Icon: Network }))
    : commands.map((command) => ({ key: command.id, title: t(command.titleKey), description: command.id === "color" && colorPreview ? t("command.copyColor", { value: colorPreview }) : t(command.descriptionKey), Icon: icons[command.icon] }));

  return <>
    {message && <div className="command-message" role="status">{translateTagged(t, message)}</div>}
    <div className="results command-results" role="listbox" aria-label={t("command.results")}>
      {!rows.length && <div className="empty-state">{t("command.none")}</div>}
      {rows.map((row, index) => <div key={row.key} className={`result-item command-item${selected === index ? " selected" : ""}`} role="option" aria-selected={selected === index} onMouseMove={() => onSelect(index)} onClick={() => onSelect(index)} onDoubleClick={() => onRun(index)} onContextMenu={(event)=>{event.preventDefault();event.stopPropagation();setContext({index,x:event.clientX,y:event.clientY});}}>
        <div className="result-icon icon-command"><row.Icon size={21} strokeWidth={1.6}/></div>
        <div className="result-copy"><div className="result-title">{row.title}</div><div className="result-subtitle">{row.description}</div></div>
        <span className="result-kind">{addresses.length ? "IP" : t("command.kind")}</span>
      </div>)}
    </div>
    {context&&rows[context.index]&&<ContextMenu x={context.x} y={context.y} onClose={()=>setContext(undefined)} items={[{id:"run",label:t(addresses.length?"context.copy":"context.run"),icon:addresses.length?Network:Power,onSelect:()=>onRun(context.index)}]}/>}
  </>;
}
