import { cx } from "zeb/react";

/** The toolbar's glyphs. Named so the buttons below read as actions, not paths. */
const ICONS = {
  save: "M13 14H3a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1h7.586a1 1 0 0 1 .707.293l2.414 2.414a1 1 0 0 1 .293.707V13a1 1 0 0 1-1 1Z M5 14V9h6v5M5 2v3h4",
  cancel: "m4 4 8 8M12 4l-8 8",
  add: "M8 3v10M3 8h10",
  remove: "M3 8h10",
  download: "M8 2v8M4 7l4 4 4-4M2 13h12",
  count: "M13 3H3v10h10V3ZM6 6h4M6 8h4M6 10h2",
  refresh: "M13.5 8A5.5 5.5 0 1 1 8 2.5M13.5 2.5v3h-3",
};

function Glyph({ path, className = "h-3.5 w-3.5" }) {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" className={className}>
      {path.split(" M").map((segment, index) => (
        <path key={index} d={index === 0 ? segment : `M${segment}`} />
      ))}
    </svg>
  );
}

const ACTION =
  "flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text disabled:opacity-30";

function Divider() {
  return <span className="mx-0.5 h-4 w-px bg-ui-border/60" />;
}

/**
 * Everything the reader can do to the rows on screen.
 *
 * Decides nothing itself: writing, deleting and counting are the editor's,
 * and refreshing is the page's, because it is the page that knows what else
 * has to be re-read.
 */
export default function GridToolbar({ editor, canDeleteRow, csv, loadedCount, onRefresh }) {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-ui-border/70 bg-ui-bg-muted/30 px-2 py-1.5">
      <button
        type="button"
        title="Save changes"
        className={cx(
          "flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium disabled:opacity-30",
          editor.hasPendingEdits
            ? "bg-blue-600 text-white hover:bg-blue-700"
            : "text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text",
        )}
        disabled={!editor.hasPendingEdits}
        onClick={() => editor.confirmSave.setOpen(true)}
      >
        <Glyph path={ICONS.save} className="h-3 w-3" />
        Save
      </button>
      <button
        type="button"
        title="Cancel changes"
        className={ACTION}
        disabled={!editor.hasPendingEdits}
        onClick={editor.cancel}
      >
        <Glyph path={ICONS.cancel} className="h-3 w-3" />
        Cancel
      </button>

      <Divider />

      <button
        type="button"
        title="Add row"
        className="flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text hover:bg-ui-bg-muted"
        onClick={editor.addRow}
      >
        <Glyph path={ICONS.add} />
        Row
      </button>
      <button
        type="button"
        title="Delete selected row"
        className={`${ACTION} hover:text-red-500`}
        disabled={!canDeleteRow}
        onClick={() => editor.confirmDeleteRow.setOpen(true)}
      >
        <Glyph path={ICONS.remove} />
        Delete
      </button>

      <Divider />

      <a
        title="Export CSV"
        className={cx(
          "flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium text-ui-text-soft hover:bg-ui-bg-muted hover:text-ui-text",
          !csv.href && "pointer-events-none opacity-30",
        )}
        href={csv.href || undefined}
        download={csv.filename}
      >
        <Glyph path={ICONS.download} />
        CSV
      </a>
      <button
        type="button"
        title="Calculate total row count"
        className={cx(
          "flex items-center gap-1 rounded px-2 py-1 text-[11px] font-medium hover:bg-ui-bg-muted",
          editor.countBusy ? "animate-pulse text-ui-text" : "text-ui-text-soft hover:text-ui-text",
        )}
        onClick={editor.countRows}
      >
        <Glyph path={ICONS.count} />
        Count
      </button>
      <button type="button" title="Refresh" className={ACTION} onClick={onRefresh}>
        <Glyph path={ICONS.refresh} />
        Refresh
      </button>

      <span className="ml-auto text-[10px] tabular-nums text-ui-text-soft">
        {editor.totalRowCount !== null ? `${editor.totalRowCount} rows` : `${loadedCount} loaded`}
      </span>
    </div>
  );
}
