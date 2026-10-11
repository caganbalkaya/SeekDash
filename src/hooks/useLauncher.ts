import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SearchResult } from "../types";

const isTauri = () => "__TAURI_INTERNALS__" in window;

export function useLauncher(query: string) {
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const request = useRef(0);
  const latestQuery = useRef(query);
  latestQuery.current = query;

  const search = useCallback(async (value: string) => {
    const token = ++request.current;
    setLoading(true);
    try {
      if (!isTauri()) { setResults([]); return; }
      const next = await invoke<SearchResult[]>("search", { query: value, requestId: token });
      if (token === request.current) setResults(next);
    } catch (error) {
      console.error("Search failed", error);
      if (token === request.current) setResults([]);
    } finally {
      if (token === request.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (query.startsWith(">")) {
      ++request.current;
      setResults([]);
      setLoading(false);
      return;
    }
    const timer = window.setTimeout(() => void search(query), query ? 24 : 0);
    return () => window.clearTimeout(timer);
  }, [query, search]);

  useEffect(() => {
    if (!isTauri()) return;
    let cleanup: (() => void) | undefined;
    let disposed = false;
    void listen("index-updated", () => {
      if (!latestQuery.current.startsWith(">")) void search(latestQuery.current);
    }).then((fn) => {
      if (disposed) fn(); else cleanup = fn;
    });
    return () => { disposed = true; cleanup?.(); };
  }, [search]);

  return { results, loading, refresh: () => search(query) };
}
