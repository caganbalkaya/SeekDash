import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ClipboardPanel } from "./ClipboardPanel";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

const settings = {
  hotkey: "Alt+Space", launchAtStartup: false, indexedDirectories: [], excludedDirectories: [],
  reducedMotion: false, theme: "dark", layoutMode: "horizontal", sidebarEdge: "left",
  sidebarAutoHide: false, clipboardEnabled: true, clipboardPersist: false,
  clipboardPaused: false, clipboardLimit: 100, clipboardHotkey: "Alt+V",
  language: "en",
};

describe("clipboard panel", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return settings;
      if (command === "search_clipboard") return [
        { id: 7, text: "Örnek pano metni", copiedAt: 1_700_000_000 },
      ];
      return undefined;
    });
  });

  it("searches and copies the selected item with Enter", async () => {
    const close = vi.fn();
    render(<ClipboardPanel onClose={close} onSettings={() => {}}/>);
    expect(await screen.findByText("Örnek pano metni")).toBeInTheDocument();
    fireEvent.keyDown(screen.getByPlaceholderText("Search clipboard history..."), { key: "Enter" });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("copy_clipboard_item", { id: 7 }));
    expect(close).toHaveBeenCalled();
  });

  it("shows the disabled privacy state", async () => {
    invoke.mockImplementation(async (command: string) => {
      if (command === "get_settings") return { ...settings, clipboardEnabled: false };
      if (command === "search_clipboard") return [];
      return undefined;
    });
    render(<ClipboardPanel onClose={() => {}} onSettings={() => {}}/>);
    expect(await screen.findByText("Clipboard history is off")).toBeInTheDocument();
  });
});
