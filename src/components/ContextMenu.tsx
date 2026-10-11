import { createPortal } from "react-dom";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { LucideIcon } from "lucide-react";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon: LucideIcon;
  danger?: boolean;
  separator?: boolean;
  onSelect(): void;
}

interface Props { x: number; y: number; items: ContextMenuItem[]; onClose(): void; }

export function ContextMenu({ x, y, items, onClose }: Props) {
  const menu = useRef<HTMLDivElement>(null);
  const buttons = useRef<Array<HTMLButtonElement | null>>([]);
  const [active, setActive] = useState(0);
  const [position, setPosition] = useState({ left:x, top:y, ready:false });

  useLayoutEffect(() => {
    const rect = menu.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({ left:Math.max(8,Math.min(x,window.innerWidth-rect.width-8)), top:Math.max(8,Math.min(y,window.innerHeight-rect.height-8)), ready:true });
    buttons.current[0]?.focus();
  }, [items.length,x,y]);
  useEffect(() => {
    const outside = (event: PointerEvent) => { if (!menu.current?.contains(event.target as Node)) onClose(); };
    const nativeMenu = (event: MouseEvent) => event.preventDefault();
    document.addEventListener("pointerdown",outside);
    document.addEventListener("contextmenu",nativeMenu);
    return () => { document.removeEventListener("pointerdown",outside); document.removeEventListener("contextmenu",nativeMenu); };
  }, [onClose]);

  const move = (delta:number) => { const next=(active+delta+items.length)%items.length; setActive(next); buttons.current[next]?.focus(); };
  const choose = (index:number) => { items[index]?.onSelect(); onClose(); };
  const onKeyDown = (event:React.KeyboardEvent) => {
    if(event.key==="ArrowDown"||event.key==="ArrowUp"){event.preventDefault();move(event.key==="ArrowDown"?1:-1);}
    else if(event.key==="Enter"||event.key===" "){event.preventDefault();choose(active);}
    else if(event.key==="Escape"){event.preventDefault();onClose();}
    else if(event.key==="Home"||event.key==="End"){event.preventDefault();const next=event.key==="Home"?0:items.length-1;setActive(next);buttons.current[next]?.focus();}
  };
  return createPortal(<div ref={menu} className="glass-context-menu" role="menu" style={{left:position.left,top:position.top,visibility:position.ready?"visible":"hidden"}} onKeyDown={onKeyDown}>
    {items.map((item,index)=><button key={item.id} ref={(node)=>{buttons.current[index]=node;}} type="button" role="menuitem" className={`${index===active?"is-active ":""}${item.danger?"is-danger ":""}${item.separator?"has-separator":""}`} onPointerMove={()=>setActive(index)} onClick={()=>choose(index)}><item.icon size={16} strokeWidth={1.7}/><span>{item.label}</span></button>)}
  </div>,document.body);
}
