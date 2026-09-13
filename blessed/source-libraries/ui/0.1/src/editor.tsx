import { cx, useEffect, useRef, useState, createPortal } from "zeb/react";
import { createEditor } from "zeb/prosemirror";
import { DocumentView, EDITOR_CLASSES } from "zeb/ui/editor-render";
import { Toggle } from "zeb/ui/toggle";

/**
 * Editor — a Notion-style block editor: type `/` for blocks, select text for
 * the bubble toolbar, Markdown shortcuts (`# `, `- `, `[] `, `> `, ```` ``` ````,
 * `---`), drag the ⋮⋮ handle to move a block, paste or drop an image.
 *
 * The value is a ProseMirror document (JSON). Store that; render it with
 * `<DocumentView doc>` or `renderDocumentHtml(doc)` from `zeb/ui/editor-render`.
 *
 * This is the one zeb/ui component with a runtime dependency: `zeb/prosemirror`
 * is the engine (schema, commands, plugins) and draws nothing; every visible
 * piece here is zeb/ui on the theme. Uploads go through `uploadImage(file)`,
 * which the page supplies — usually a webhook running `n.fs.save` — and
 * which returns a URL or `{ src, ref, alt }`. Without it, images are refused.
 *
 *   <Editor value={doc} onChange={setDoc} placeholder="Write…" uploadImage={upload} />
 */

const SLASH_ITEMS = [
  { id: "paragraph", label: "Text", hint: "Plain paragraph", keys: "text p", run: (ed) => ed.exec("paragraph") },
  { id: "h1", label: "Heading 1", hint: "Section title", keys: "h1 title", run: (ed) => ed.exec("heading", 1) },
  { id: "h2", label: "Heading 2", hint: "Subsection", keys: "h2", run: (ed) => ed.exec("heading", 2) },
  { id: "h3", label: "Heading 3", hint: "Small heading", keys: "h3", run: (ed) => ed.exec("heading", 3) },
  { id: "bullet", label: "Bulleted list", hint: "• item", keys: "ul bullet list", run: (ed) => ed.exec("list", "bullet") },
  { id: "ordered", label: "Numbered list", hint: "1. item", keys: "ol number", run: (ed) => ed.exec("list", "ordered") },
  { id: "todo", label: "To-do list", hint: "☐ task", keys: "todo task check", run: (ed) => ed.exec("list", "todo") },
  { id: "quote", label: "Quote", hint: "A pulled line", keys: "quote blockquote", run: (ed) => ed.exec("blockquote") },
  { id: "callout", label: "Callout", hint: "A box with an icon", keys: "callout note tip", run: (ed) => ed.exec("callout") },
  { id: "code", label: "Code", hint: "A code block", keys: "code pre", run: (ed) => ed.exec("codeBlock") },
  { id: "divider", label: "Divider", hint: "A horizontal rule", keys: "hr divider rule", run: (ed) => ed.exec("horizontalRule") },
  { id: "image", label: "Image", hint: "Upload a picture", keys: "image photo picture", needsUpload: true, run: null },
];

const MARK_BUTTONS = [
  ["bold", "B", "font-bold"],
  ["italic", "I", "italic"],
  ["underline", "U", "underline"],
  ["strike", "S", "line-through"],
  ["code", "</>", "font-mono text-xs"],
];

// The menus are fixed-position portals placed at caret coordinates. Near
// the top the bubble goes below the selection; near the bottom the slash
// menu opens upward. Neither may leave the viewport sideways.
function viewportWidth() {
  return typeof window === "undefined" ? 1280 : window.innerWidth;
}
function flipUp(bottom, height) {
  return typeof window !== "undefined" && bottom + 6 + height > window.innerHeight && bottom > height;
}

function isEmptyDoc(doc) {
  return !doc || !doc.content || (doc.content.length === 1 && doc.content[0].type === "paragraph" && !(doc.content[0].content || []).length);
}

export function Editor({ value, defaultValue, onChange, placeholder = "Type '/' for blocks, or just write…", uploadImage, readOnly = false, className, minHeight = "12rem" }) {
  const mountRef = useRef(null);
  const editorRef = useRef(null);
  const fileRef = useRef(null);
  // Every document the engine has emitted (or been given), by identity. The
  // controlled effect below sees each render's `value` in turn, and under
  // fast typing that is a stale one — A, after AB was already emitted.
  // Anything we produced ourselves is never written back into the engine.
  const emitted = useRef(new WeakSet());
  const lastEmitted = useRef(null);
  const [empty, setEmpty] = useState(isEmptyDoc(value ?? defaultValue));
  const [slash, setSlash] = useState(null);
  const [slashIndex, setSlashIndex] = useState(0);
  const [selection, setSelection] = useState(null);
  const [linkDraft, setLinkDraft] = useState(null);
  const [mounted, setMounted] = useState(false);

  // Mount the engine once; it owns the DOM under mountRef from here on.
  useEffect(() => {
    const mount = mountRef.current;
    if (!mount || typeof createEditor !== "function") return;
    const initial = value ?? defaultValue;
    if (initial && typeof initial === "object") emitted.current.add(initial);
    const editor = createEditor(mount, {
      doc: initial,
      classes: EDITOR_CLASSES,
      placeholder,
      editable: !readOnly,
      uploadImage,
      onChange: (json) => {
        emitted.current.add(json);
        lastEmitted.current = JSON.stringify(json);
        setEmpty(isEmptyDoc(json));
        onChange?.(json);
      },
      onSlash: (state) => { setSlash(state); setSlashIndex(0); },
      onSelection: setSelection,
    });
    editorRef.current = editor;
    setMounted(true);
    return () => { editor.destroy(); editorRef.current = null; };
  }, []);

  // Controlled: a value the page changed (not one we just emitted) replaces the document.
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || value === undefined || value === null) return;
    if (typeof value === "object" && emitted.current.has(value)) return;
    const next = JSON.stringify(value);
    if (next === lastEmitted.current || next === JSON.stringify(editor.getJSON())) return;
    editor.setJSON(value);
    setEmpty(isEmptyDoc(value));
  }, [value]);

  const slashItems = slash
    ? SLASH_ITEMS.filter((item) => (!item.needsUpload || uploadImage) && (item.label + " " + item.keys).toLowerCase().includes(slash.query.toLowerCase()))
    : [];

  function runSlash(item) {
    const editor = editorRef.current;
    if (!editor || !slash) return;
    editor.deleteSlash(slash);
    setSlash(null);
    if (item.id === "image") fileRef.current?.click();
    else item.run(editor);
  }

  // The slash menu takes the arrow keys and Enter while it is open.
  useEffect(() => {
    const mount = mountRef.current;
    if (!mount || !slash) return;
    const onKey = (event) => {
      if (event.key === "ArrowDown") { event.preventDefault(); setSlashIndex((i) => (i + 1) % Math.max(slashItems.length, 1)); }
      else if (event.key === "ArrowUp") { event.preventDefault(); setSlashIndex((i) => (i - 1 + Math.max(slashItems.length, 1)) % Math.max(slashItems.length, 1)); }
      else if (event.key === "Enter") { event.preventDefault(); if (slashItems[slashIndex]) runSlash(slashItems[slashIndex]); }
      else if (event.key === "Escape") { event.preventDefault(); setSlash(null); }
    };
    mount.addEventListener("keydown", onKey, true);
    return () => mount.removeEventListener("keydown", onKey, true);
  }, [slash, slashItems.length, slashIndex]);

  async function onFilePicked(event) {
    const file = event.target.files && event.target.files[0];
    event.target.value = "";
    const editor = editorRef.current;
    if (!file || !editor || !uploadImage) return;
    const result = await uploadImage(file);
    if (!result) return;
    const attrs = typeof result === "string" ? { src: result, alt: file.name } : { src: result.src || result.url, ref: result.ref || null, alt: result.alt || file.name };
    editor.exec("image", attrs);
  }

  const overlays = mounted && typeof document !== "undefined" ? createPortal(
    <>
      {slash && slashItems.length > 0 ? (
        <div role="listbox" data-slot="editor-slash" className={cx("fixed z-50 w-64 overflow-hidden rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-md", flipUp(slash.bottom, slashItems.length * 44 + 8) ? "-translate-y-full" : "")} style={{ left: `${Math.max(8, Math.min(slash.left, viewportWidth() - 264))}px`, top: `${flipUp(slash.bottom, slashItems.length * 44 + 8) ? slash.top - 6 : slash.bottom + 6}px` }}>
          {slashItems.map((item, i) => (
            <button
              key={item.id}
              type="button"
              role="option"
              aria-selected={i === slashIndex ? "true" : "false"}
              onMouseDown={(e) => { e.preventDefault(); runSlash(item); }}
              onMouseEnter={() => setSlashIndex(i)}
              className={cx("flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left text-sm", i === slashIndex ? "bg-accent text-accent-foreground" : "")}
            >
              <span className="w-8 shrink-0 text-center font-mono text-[0.65rem] text-muted-foreground">{item.hint.slice(0, 2)}</span>
              <span className="min-w-0 flex-1"><span className="block font-medium">{item.label}</span><span className="block truncate text-xs text-muted-foreground">{item.hint}</span></span>
            </button>
          ))}
        </div>
      ) : null}
      {selection && !readOnly ? (
        <div data-slot="editor-bubble" className={cx("fixed z-50 flex -translate-x-1/2 items-center gap-0.5 rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-md", selection.top < 56 ? "" : "-translate-y-full")} style={{ left: `${Math.max(170, Math.min(selection.left, viewportWidth() - 170))}px`, top: `${selection.top < 56 ? selection.bottom + 8 : selection.top - 8}px` }} onMouseDown={(e) => e.preventDefault()}>
          {MARK_BUTTONS.map(([mark, label, cls]) => (
            <Toggle key={mark} size="sm" pressed={!!selection.marks[mark]} onPressedChange={() => editorRef.current?.exec(`toggle${mark[0].toUpperCase()}${mark.slice(1)}`)} aria-label={mark} className={cls}>{label}</Toggle>
          ))}
          <span className="mx-1 h-5 w-px bg-border" />
          <Toggle size="sm" pressed={selection.block === "heading" && selection.attrs?.level === 1} onPressedChange={() => editorRef.current?.exec(selection.block === "heading" && selection.attrs?.level === 1 ? "paragraph" : "heading", 1)} aria-label="Heading 1" className="font-mono text-xs">H1</Toggle>
          <Toggle size="sm" pressed={selection.block === "heading" && selection.attrs?.level === 2} onPressedChange={() => editorRef.current?.exec(selection.block === "heading" && selection.attrs?.level === 2 ? "paragraph" : "heading", 2)} aria-label="Heading 2" className="font-mono text-xs">H2</Toggle>
          <span className="mx-1 h-5 w-px bg-border" />
          {linkDraft === null ? (
            <Toggle size="sm" pressed={!!selection.marks.link} onPressedChange={() => (selection.marks.link ? editorRef.current?.exec("setLink", "") : setLinkDraft(""))} aria-label="Link" className="font-mono text-xs">link</Toggle>
          ) : (
            <form className="flex items-center gap-1" onSubmit={(e) => { e.preventDefault(); editorRef.current?.exec("setLink", linkDraft.trim()); setLinkDraft(null); }}>
              <input autoFocus value={linkDraft} onInput={(e) => setLinkDraft(e.target.value)} onKeyDown={(e) => { if (e.key === "Escape") setLinkDraft(null); }} placeholder="https://" className="h-7 w-44 rounded-md border border-input bg-background px-2 text-xs outline-none focus-visible:ring-2 focus-visible:ring-ring/50" />
              <button type="submit" className="h-7 rounded-md bg-primary px-2 text-xs text-primary-foreground">Set</button>
            </form>
          )}
        </div>
      ) : null}
    </>,
    document.body,
  ) : null;

  return (
    <div data-slot="editor" className={cx("relative rounded-lg border border-input bg-background px-4 py-3 focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/30", className)} style={{ minHeight }}>
      {empty && !readOnly ? <div aria-hidden="true" className="pointer-events-none absolute left-11 top-3 select-none text-base leading-7 text-muted-foreground">{placeholder}</div> : null}
      {/* The engine owns everything under mountRef; it is never given React
          children, so a re-render cannot patch ProseMirror's DOM. The static
          view is a sibling for the server (and the moment before mount). */}
      <div ref={mountRef} />
      {!mounted ? <DocumentView doc={value ?? defaultValue} className={EDITOR_CLASSES.root} /> : null}
      {uploadImage ? <input ref={fileRef} type="file" accept="image/*" className="hidden" onChange={onFilePicked} /> : null}
      {overlays}
    </div>
  );
}

export default Editor;
