import { cx, useEffect, useRef, useState } from "zeb/react";

/**
 * EditorPicker — the list an extension's picker shows while the editor asks
 * it for suggestions: after a trigger (`@ad…`, the query is what follows
 * it) or from the slash menu (`searchable`: the list has its own search box).
 *
 * `picker.search(query)` answers `{ id, label, href, snapshot }[]`; an error
 * is shown in the list, never swallowed. Arrow keys move, Enter picks,
 * Escape closes — wherever the focus is.
 */

/** Fixed placement for a menu at a caret: below it, or above when the viewport ends. */
export function editorMenuPlacement(anchor, height, width = 264) {
  const vw = typeof window === "undefined" ? 1280 : window.innerWidth;
  const up = typeof window !== "undefined" && anchor.bottom + 6 + height > window.innerHeight && anchor.bottom > height;
  return { up, style: { left: `${Math.max(8, Math.min(anchor.left, vw - width))}px`, top: `${up ? anchor.top - 6 : anchor.bottom + 6}px` } };
}

export function EditorPicker({ picker, query, anchor, searchable, onPick, onClose }) {
  const [own, setOwn] = useState("");
  const [items, setItems] = useState([]);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(true);
  const [index, setIndex] = useState(0);
  const q = searchable ? own : query || "";
  // The key listener reads the latest handlers: the caller's range moves with every keystroke.
  const handlers = useRef(null);
  handlers.current = { onPick, onClose };

  useEffect(() => {
    let live = true;
    setLoading(true);
    const timer = setTimeout(() => {
      Promise.resolve()
        .then(() => picker.search(q))
        .then((list) => { if (live) { setItems(list); setError(null); setIndex(0); setLoading(false); } })
        .catch((err) => { if (live) { setItems([]); setError(String((err && err.message) || err)); setLoading(false); } });
    }, 120);
    return () => { live = false; clearTimeout(timer); };
  }, [q]);

  useEffect(() => {
    const onKey = (event) => {
      if (event.key === "Escape") { event.preventDefault(); handlers.current.onClose(); return; }
      if (items.length === 0) return;
      if (event.key === "ArrowDown") { event.preventDefault(); setIndex((i) => (i + 1) % items.length); }
      else if (event.key === "ArrowUp") { event.preventDefault(); setIndex((i) => (i - 1 + items.length) % items.length); }
      else if (event.key === "Enter") { event.preventDefault(); event.stopPropagation(); handlers.current.onPick(items[index]); }
    };
    document.addEventListener("keydown", onKey, true);
    return () => document.removeEventListener("keydown", onKey, true);
  }, [items, index]);

  const place = editorMenuPlacement(anchor, Math.min(items.length, 6) * 44 + (searchable ? 52 : 8));
  return (
    <div role="listbox" data-slot="editor-picker" className={cx("fixed z-50 max-h-80 w-64 overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-md", place.up ? "-translate-y-full" : "")} style={place.style} onMouseDown={(e) => { if (e.target.tagName !== "INPUT") e.preventDefault(); }}>
      {searchable ? (
        <input autoFocus value={own} onInput={(e) => setOwn(e.target.value)} placeholder="Search…" aria-label="Search" className="mb-1 h-8 w-full rounded-md border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
      ) : null}
      {error ? <div className="px-2 py-1.5 text-xs text-destructive">{error}</div> : null}
      {!error && loading && items.length === 0 ? <div className="px-2 py-1.5 text-xs text-muted-foreground">Searching…</div> : null}
      {!error && !loading && items.length === 0 ? <div className="px-2 py-1.5 text-xs text-muted-foreground">No matches</div> : null}
      {items.map((item, i) => (
        <button
          key={String(item.id)}
          type="button"
          role="option"
          aria-selected={i === index ? "true" : "false"}
          onMouseDown={(e) => { e.preventDefault(); onPick(item); }}
          onMouseEnter={() => setIndex(i)}
          className={cx("flex w-full flex-col rounded-md px-2 py-1.5 text-left text-sm", i === index ? "bg-accent text-accent-foreground" : "")}
        >
          <span className="block truncate font-medium">{item.label || String(item.id)}</span>
          {item.snapshot && item.snapshot.description ? <span className="block truncate text-xs text-muted-foreground">{item.snapshot.description}</span> : null}
        </button>
      ))}
    </div>
  );
}

export default EditorPicker;
