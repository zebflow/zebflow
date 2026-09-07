import { cx } from "zeb/react";
import ContextMenu from "@/components/ui/context-menu";
import FileKindIcon from "@/components/ui/file-kind-icon";

/** An open or closed folder. Files get the studio's own technology marks. */
export function EntryIcon({ kind, name, open }) {
  if (kind !== "folder" && kind !== "root") {
    return (
      <span className="flex h-4 w-4 shrink-0 items-center justify-center text-ui-text-soft">
        <FileKindIcon name={name} />
      </span>
    );
  }
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinejoin="round"
      className="h-4 w-4 shrink-0"
    >
      {open ? (
        <path d="M4 20a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.7.9l.8 1.2a2 2 0 0 0 1.7.9H20a2 2 0 0 1 2 2v1H6.5L4 20Z" />
      ) : (
        <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
      )}
    </svg>
  );
}

/** A caret that points down once the folder is open. */
function Caret({ open, hidden }) {
  return (
    <svg
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      className={cx(
        "h-3 w-3 shrink-0 transition-transform",
        open && "rotate-90",
        hidden && "invisible",
      )}
    >
      <path d="M4.5 2.5 8 6l-3.5 3.5" />
    </svg>
  );
}

/**
 * One line of the tree: the root, a folder, or a file.
 *
 * Every row is the same shape and carries the same `⋯`, which is the whole
 * point — "what can I do here" has one answer and one place to find it,
 * including at the repository root, which is a row like any other.
 */
export default function RepoTreeRow({ item, depth, open, active, status, menu, onActivate }) {
  const expandable = item.kind === "folder" || item.kind === "root";

  return (
    <ContextMenu
      items={menu}
      trigger="⋯"
      className={cx(
        "flex w-full items-center gap-1.5 py-[3px] pr-1 text-[12px] transition-colors",
        active
          ? "bg-ui-bg-muted text-ui-text"
          : "text-ui-text-soft hover:bg-ui-bg-muted/60 hover:text-ui-text",
      )}
    >
      <button
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          type="button"
          title={item.rel_path || item.name}
          data-repo-tree-row={item.rel_path}
          data-repo-tree-kind={item.kind}
          data-selected={active ? "true" : undefined}
          className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
          onClick={() => onActivate(item)}
        >
          <Caret open={open} hidden={!expandable} />
          <EntryIcon kind={item.kind} name={item.name} open={open} />
          <span className={cx("truncate", item.kind === "root" && "font-medium text-ui-text")}>
            {item.name}
          </span>
          {status ? (
            <span className="shrink-0 font-mono text-[10px] text-dark-accent3" title={`git: ${status}`}>
              {status}
            </span>
          ) : null}
      </button>
    </ContextMenu>
  );
}
