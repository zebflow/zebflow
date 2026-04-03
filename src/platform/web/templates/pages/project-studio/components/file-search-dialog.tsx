import { useState, useEffect, useRef } from "zeb";

export interface FileItem {
  rel_path: string;
  name: string;
}

interface SearchMatch {
  rel_path: string;
  line: number;
  snippet: string;
}

interface ResultItem {
  relPath: string;
  label: string;
  sub?: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onSelect: (relPath: string) => void;
  owner: string;
  project: string;
  scope?: "pages" | "all";
  items: FileItem[];
}

export default function FileSearchDialog({ open, onClose, onSelect, owner, project, scope, items }: Props) {
  const [mode, setMode] = useState<"files" | "search">("files");
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ResultItem[]>([]);
  const [cursor, setCursor] = useState(0);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef(null);
  const listRef = useRef(null);

  // Reset on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setCursor(0);
      setMode("files");
      setLoading(false);
    }
  }, [open]);

  // Auto-focus input
  useEffect(() => {
    if (!open) return;
    const t = setTimeout(() => (inputRef.current as any)?.focus(), 40);
    return () => clearTimeout(t);
  }, [open]);

  // Files mode — filter locally
  useEffect(() => {
    if (mode !== "files") return;
    const q = query.trim().toLowerCase();
    const filtered = items
      .filter((item) => scope !== "pages" || item.rel_path.startsWith("pages/"))
      .filter((item) => !q || item.name.toLowerCase().includes(q) || item.rel_path.toLowerCase().includes(q))
      .slice(0, 60)
      .map((item) => ({
        relPath: item.rel_path,
        label: item.name || item.rel_path,
        sub: item.rel_path,
      }));
    setResults(filtered);
    setCursor(0);
  }, [mode, query, items, scope]);

  // Search mode — debounced grep
  useEffect(() => {
    if (mode !== "search") return;
    const q = query.trim();
    if (!q) { setResults([]); setLoading(false); return; }
    setLoading(true);
    const t = setTimeout(async () => {
      try {
        const params = new URLSearchParams({ q });
        if (scope === "pages") params.set("scope", "pages");
        const resp = await fetch(`/api/projects/${owner}/${project}/templates/search?${params}`);
        const data = await resp.json();
        const matches: SearchMatch[] = Array.isArray(data?.matches) ? data.matches : [];
        setResults(
          matches.slice(0, 60).map((m) => ({
            relPath: m.rel_path,
            label: m.rel_path,
            sub: `line ${m.line}: ${m.snippet.trim().slice(0, 80)}`,
          }))
        );
        setCursor(0);
      } catch {
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 250);
    return () => { clearTimeout(t); setLoading(false); };
  }, [mode, query, owner, project, scope]);

  // Keyboard navigation
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") { e.preventDefault(); onClose(); return; }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setCursor((c) => Math.min(c + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setCursor((c) => Math.max(c - 1, 0));
      } else if (e.key === "Enter") {
        const r = results[cursor];
        if (r) { e.preventDefault(); onSelect(r.relPath); onClose(); }
      }
    }
    window.addEventListener("keydown", onKey, { capture: true });
    return () => window.removeEventListener("keydown", onKey, { capture: true });
  }, [open, results, cursor, onClose, onSelect]);

  // Scroll active result into view
  useEffect(() => {
    if (!listRef.current) return;
    const el = (listRef.current as HTMLElement).querySelector<HTMLElement>(`[data-idx="${cursor}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [cursor]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[200] flex items-start justify-center pt-[10vh]"
      onClick={onClose}
    >
      <div className="absolute inset-0 bg-black/60" />
      <div
        className="relative z-10 w-full max-w-xl mx-4 rounded-xl border border-dark-border bg-dark-background shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Input */}
        <div className="flex items-center gap-2 px-3 py-2.5 border-b border-dark-border">
          <DlgSearchIcon />
          <input
            ref={inputRef}
            value={query}
            onInput={(e: any) => setQuery(e.target.value)}
            placeholder={mode === "files" ? "Find file by name…" : "Search file contents…"}
            className="flex-1 min-w-0 bg-transparent text-sm text-dark-text1 placeholder-dark-text1/40 outline-none"
          />
          {loading && <DlgSpinIcon />}
        </div>
        {/* Mode tabs */}
        <div className="flex gap-1.5 px-3 py-2 border-b border-dark-border">
          <button
            onClick={() => { setMode("files"); setQuery(""); }}
            className={`text-xs px-2.5 py-0.5 rounded-full transition-colors ${
              mode === "files"
                ? "bg-blue-600 text-white"
                : "text-dark-text1/60 hover:bg-dark-accent3 hover:text-dark-text1"
            }`}
          >
            Files
          </button>
          <button
            onClick={() => { setMode("search"); setQuery(""); }}
            className={`text-xs px-2.5 py-0.5 rounded-full transition-colors ${
              mode === "search"
                ? "bg-blue-600 text-white"
                : "text-dark-text1/60 hover:bg-dark-accent3 hover:text-dark-text1"
            }`}
          >
            Search
          </button>
        </div>
        {/* Results */}
        <div ref={listRef} className="max-h-64 overflow-y-auto">
          {results.length === 0 && !loading && (
            <p className="py-5 text-center text-xs text-dark-text1/40">
              {mode === "files"
                ? query.trim() ? "No files match" : "Type to filter files"
                : query.trim() ? "No matches found" : "Type to search file contents"}
            </p>
          )}
          {results.map((r, i) => {
            const isPipeline = r.relPath.endsWith(".zf.json");
            return (
              <button
                key={`result-${i}`}
                data-idx={i}
                onClick={() => { onSelect(r.relPath); onClose(); }}
                className={`w-full text-left px-3 py-2 flex flex-col gap-0.5 transition-colors ${
                  i === cursor
                    ? "bg-blue-600/20 text-dark-text1"
                    : "text-dark-text1/80 hover:bg-dark-accent3 hover:text-dark-text1"
                }`}
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-sm font-medium truncate flex-1">{r.label}</span>
                  <span className={`shrink-0 text-[10px] px-1.5 py-0.5 rounded font-medium ${
                    isPipeline
                      ? "bg-violet-500/20 text-violet-400"
                      : "bg-sky-500/20 text-sky-400"
                  }`}>
                    {isPipeline ? "pipeline" : "template"}
                  </span>
                </div>
                {r.sub && r.sub !== r.label && (
                  <span className="text-xs text-dark-text1/40 truncate">{r.sub}</span>
                )}
              </button>
            );
          })}
        </div>
        {/* Footer hints */}
        <div className="flex items-center gap-3 px-3 py-1.5 border-t border-dark-border text-[11px] text-dark-text1/30 select-none">
          <span>↑↓ navigate</span>
          <span>↵ select</span>
          <span>Esc close</span>
        </div>
      </div>
    </div>
  );
}

function DlgSearchIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4 shrink-0 text-dark-text1/40">
      <circle cx="11" cy="11" r="7" stroke="currentColor" strokeWidth="1.8" />
      <path d="M17 17l4 4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function DlgSpinIcon() {
  return (
    <svg className="w-3.5 h-3.5 animate-spin text-dark-text1/40" viewBox="0 0 24 24" fill="none">
      <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="3" opacity="0.3" />
      <path fill="currentColor" opacity="0.8" d="M4 12a8 8 0 018-8V0C5.4 0 0 5.4 0 12h4z" />
    </svg>
  );
}
