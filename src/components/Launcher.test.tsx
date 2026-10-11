import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { vi, describe, expect, it } from "vitest";
import { SearchInput } from "./SearchInput";
import { SearchResults } from "./SearchResults";

describe("launcher controls", () => {
  it("reports search input changes", () => {
    const change = vi.fn();
    render(<SearchInput value="" onChange={change} onClear={() => {}} onSettings={() => {}} />);
    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "terminal" } });
    expect(change).toHaveBeenCalledWith("terminal");
  });

  it("renders selected result", () => {
    render(<SearchResults loading={false} selected={0} onSelect={() => {}} onOpen={() => {}} onFavorite={() => {}} onAction={() => {}} results={[{ id: "1", title: "Documents", subtitle: "C:\\Users\\Me\\Documents", path: "C:\\Users\\Me\\Documents", kind: "folder", score: 100, favorite: false }]} />);
    expect(screen.getByRole("option")).toHaveAttribute("aria-selected", "true");
  });

  it("uses a dedicated image icon and exposes favorite control", () => {
    render(<SearchResults loading={false} selected={0} onSelect={() => {}} onOpen={() => {}} onFavorite={() => {}} onAction={() => {}} results={[{ id: "2", title: "photo.png", subtitle: "C:\\Pictures", path: "C:\\Pictures\\photo.png", kind: "file", score: 90, favorite: false }]} />);
    expect(document.querySelector('[data-icon-category="image"]')).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add photo.png to favorites" })).toBeInTheDocument();
  });

  it("opens the custom context menu for the right-clicked result", () => {
    render(<SearchResults loading={false} selected={0} onSelect={() => {}} onOpen={() => {}} onFavorite={() => {}} onAction={() => {}} results={[{ id: "3", title: "tool.exe", subtitle: "C:\\Tools", path: "C:\\Tools\\tool.exe", kind: "application", score: 80, favorite: false }]} />);
    fireEvent.contextMenu(screen.getAllByRole("option").at(-1)!);
    expect(screen.getByRole("menuitem", { name: "Open" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Run as administrator" })).toBeInTheDocument();
  });
});
