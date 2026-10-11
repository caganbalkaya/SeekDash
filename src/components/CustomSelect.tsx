import { Check, ChevronDown } from "lucide-react";
import { createPortal } from "react-dom";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

export interface SelectOption {
  value: string;
  label: string;
}

interface Props {
  value: string;
  options: SelectOption[];
  onChange(value: string): void;
  ariaLabel?: string;
}

export function CustomSelect({ value, options, onChange, ariaLabel }: Props) {
  const id = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const [open, setOpen] = useState(false);
  const selectedIndex = Math.max(0, options.findIndex((option) => option.value === value));
  const [activeIndex, setActiveIndex] = useState(selectedIndex);
  const [position, setPosition] = useState({ top: 0, left: 0, width: 180, upward: false });

  const updatePosition = () => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;
    const panelHeight = Math.min(options.length * 40 + 12, 260);
    const upward = rect.bottom + panelHeight + 8 > window.innerHeight && rect.top > panelHeight;
    setPosition({ top: upward ? rect.top - panelHeight - 6 : rect.bottom + 6, left: Math.min(rect.left, window.innerWidth - rect.width - 12), width: rect.width, upward });
  };

  const show = (index = selectedIndex) => {
    setActiveIndex(index);
    setOpen(true);
  };
  const choose = (index: number) => {
    const option = options[index];
    if (option && option.value !== value) onChange(option.value);
    setOpen(false);
    buttonRef.current?.focus();
  };

  useLayoutEffect(() => { if (open) updatePosition(); }, [open, options.length]);
  useEffect(() => {
    if (!open) return;
    optionRefs.current[activeIndex]?.focus();
    const close = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!buttonRef.current?.contains(target) && !(target instanceof Element && target.closest(`[data-select-menu="${id}"]`))) setOpen(false);
    };
    const reposition = () => updatePosition();
    document.addEventListener("pointerdown", close);
    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => { document.removeEventListener("pointerdown", close); window.removeEventListener("resize", reposition); window.removeEventListener("scroll", reposition, true); };
  }, [activeIndex, id, open]);

  const move = (delta: number) => {
    const next = (activeIndex + delta + options.length) % options.length;
    setActiveIndex(next);
    optionRefs.current[next]?.focus();
  };
  const onButtonKeyDown = (event: React.KeyboardEvent) => {
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
      event.preventDefault();
      show(event.key === "ArrowUp" ? (selectedIndex - 1 + options.length) % options.length : selectedIndex);
    }
  };
  const onMenuKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); move(event.key === "ArrowDown" ? 1 : -1); }
    else if (event.key === "Enter" || event.key === " ") { event.preventDefault(); choose(activeIndex); }
    else if (event.key === "Escape" || event.key === "Tab") { if (event.key === "Escape") event.preventDefault(); setOpen(false); buttonRef.current?.focus(); }
    else if (event.key === "Home" || event.key === "End") { event.preventDefault(); const next = event.key === "Home" ? 0 : options.length - 1; setActiveIndex(next); optionRefs.current[next]?.focus(); }
  };

  const selected = options[selectedIndex] ?? options[0];
  return <div className={`custom-select${open ? " is-open" : ""}`}>
    <button ref={buttonRef} type="button" className="custom-select-trigger" aria-label={ariaLabel} aria-haspopup="listbox" aria-expanded={open} aria-controls={`${id}-menu`} onClick={() => open ? setOpen(false) : show()} onKeyDown={onButtonKeyDown}>
      <span>{selected?.label}</span><ChevronDown size={15}/>
    </button>
    {open && createPortal(<div id={`${id}-menu`} data-select-menu={id} className={`custom-select-menu${position.upward ? " opens-up" : ""}`} role="listbox" aria-label={ariaLabel} style={{ top:position.top, left:position.left, width:position.width }} onKeyDown={onMenuKeyDown}>
      {options.map((option,index) => <button key={option.value} ref={(node) => { optionRefs.current[index]=node; }} type="button" role="option" aria-selected={option.value===value} className={`${index===activeIndex ? "is-active" : ""}${option.value===value ? " is-selected" : ""}`} onPointerMove={() => setActiveIndex(index)} onClick={() => choose(index)}><span>{option.label}</span>{option.value===value&&<Check size={15}/>}</button>)}
    </div>, document.body)}
  </div>;
}
