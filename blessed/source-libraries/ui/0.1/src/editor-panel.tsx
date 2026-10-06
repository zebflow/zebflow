import { readAttrPath, writeAttrPath } from "zeb/ui/editor-extension";

/**
 * EditorNodePanel — edits the node under the caret when its extension
 * declares `fields` (attrs, dotted paths allowed: `snapshot.title`) or
 * `actions` (`{ label, run(editor, node) }`). Each keystroke writes the
 * attr straight into the document; Escape closes the panel.
 *
 * A field's `type` is text (default), `url`, `number`, `checkbox` or
 * `select` (with `options`).
 *
 * `node` is the engine's report: `{ extension, type, pos, attrs, left, bottom }`.
 */

const INPUT = "h-8 w-full rounded-md border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

export function EditorNodePanel({ ext, node, editor, onClose }) {
  const set = (path, value) => editor.setNodeAttrs(node.pos, writeAttrPath(node.attrs, path, value));
  const left = Math.max(8, Math.min(node.left, (typeof window === "undefined" ? 1280 : window.innerWidth) - 296));
  return (
    <div
      data-slot="editor-panel"
      data-extension={ext.name}
      className="fixed z-40 w-72 rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-md"
      style={{ left: `${left}px`, top: `${node.bottom + 8}px` }}
      onKeyDown={(e) => { if (e.key === "Escape") { e.preventDefault(); onClose(); editor.focus(); } }}
    >
      <div className="mb-2 flex items-center justify-between">
        <span className="font-mono text-[0.65rem] uppercase tracking-wider text-muted-foreground">{ext.label || ext.name}</span>
        <button type="button" aria-label="Close" onClick={onClose} className="rounded px-1 text-muted-foreground hover:text-foreground">×</button>
      </div>
      {(ext.fields || []).map((field) => {
        const value = readAttrPath(node.attrs, field.name);
        if (field.type === "checkbox") {
          return (
            <label key={field.name} className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
              <input type="checkbox" name={field.name} checked={value === true || value === "true"} onChange={(e) => set(field.name, e.target.checked)} className="size-4 accent-primary" />
              <span>{field.label || field.name}</span>
            </label>
          );
        }
        return (
          <label key={field.name} className="mb-2 block text-xs text-muted-foreground">
            <span className="mb-1 block">{field.label || field.name}</span>
            {field.type === "select" ? (
              <select value={value == null ? "" : String(value)} onChange={(e) => set(field.name, e.target.value)} className={INPUT}>
                {(field.options || []).map((option) => <option key={option} value={option}>{option}</option>)}
              </select>
            ) : (
              <input type={field.type === "url" || field.type === "number" ? field.type : "text"} value={value == null ? "" : String(value)} placeholder={field.placeholder || ""} onInput={(e) => set(field.name, field.type === "number" && e.target.value !== "" ? Number(e.target.value) : e.target.value)} name={field.name} className={INPUT} />
            )}
          </label>
        );
      })}
      {ext.actions && ext.actions.length ? (
        <div className="flex flex-wrap gap-1">
          {ext.actions.map((action) => (
            <button key={action.label} type="button" onMouseDown={(e) => e.preventDefault()} onClick={() => action.run(editor, node)} className="h-7 rounded-md border border-border bg-background px-2 text-xs hover:bg-accent hover:text-accent-foreground">{action.label}</button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export default EditorNodePanel;
