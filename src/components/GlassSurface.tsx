import type { PropsWithChildren } from "react";

export function GlassSurface({ children }: PropsWithChildren) {
  return (
    <main className="glass-shell" data-tauri-drag-region>
      <div className="glass-refraction" aria-hidden="true" />
      <div className="glass-content">{children}</div>
    </main>
  );
}
