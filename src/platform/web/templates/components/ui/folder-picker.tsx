import { cx, useEffect, useState } from "zeb/react";
import { repoTree } from "@/components/lib/repo-tree";

/** A folder in the picker: caret, icon, name. No actions — this only chooses. */
function PickerRow({ item, depth, open, selected, onToggle, onChoose }) {
  return (
    <div
      className={cx(
        "flex w-full items-center gap-1 py-[3px] pr-2 text-[12px] transition-colors",
        selected ? "bg-ui-bg-muted text-ui-text" : "text-ui-text-soft hover:bg-ui-bg-muted/60",
      )}
      style={{ paddingLeft: `${depth * 12 + 6}px` }}
    >
      <button
        type="button"
        aria-label={open ? "Collapse" : "Expand"}
        className="shrink-0 rounded p-[1px] hover:bg-ui-bg-muted"
        onClick={() => onToggle(item.rel_path)}
      >
        <svg
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          className={cx("h-3 w-3 transition-transform", open && "rotate-90")}
        >
          <path d="M4.5 2.5 8 6l-3.5 3.5" />
        </svg>
      </button>
      <button
        type="button"
        data-folder-option={item.rel_path}
        className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
        onClick={() => onChoose(item.rel_path)}
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" className="h-3.5 w-3.5 shrink-0">
          <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
        </svg>
        <span className="truncate">{item.name}</span>
      </button>
    </div>
  );
}

/** One level of folders, fetched when it is opened. */
function PickerLevel({ path, depth, state, actions }) {
  const items = state.folders[path];

  useEffect(() => {
    actions.load(path);
  }, [path]);

  if (!items) {
    return (
      <p className="py-1 text-[11px] text-ui-text-soft" style={{ paddingLeft: `${depth * 12 + 24}px` }}>
        Loading…
      </p>
    );
  }
  const folders = items.filter((item) => item.kind === "folder");
  if (!folders.length) return null;

  return (
    <>
      {folders.map((item) => {
        const open = !!state.expanded[item.rel_path];
        return (
          <div key={item.rel_path}>
            <PickerRow
              item={item}
              depth={depth}
              open={open}
              selected={state.value === item.rel_path}
              onToggle={actions.toggle}
              onChoose={actions.choose}
            />
            {open ? (
              <PickerLevel path={item.rel_path} depth={depth + 1} state={state} actions={actions} />
            ) : null}
          </div>
        );
      })}
    </>
  );
}

/**
 * Choosing a folder in this project's repository.
 *
 * Its own component because "where should this go" is asked in several places —
 * adding a package, moving a file, saving something new — and a text box that
 * expects a path typed correctly is a worse answer every time.
 *
 * Folders only, fetched one level at a time through the same cache the
 * repository tree uses, so opening a folder here costs nothing if the tree has
 * already been there. `multiple` is accepted and reserved: the shape below
 * returns a list so a multi-select caller needs no different component, but
 * only single selection is implemented today.
 */
export default function FolderPicker({ owner, project, value, onChange, rootLabel, multiple }) {
  const tree = repoTree(owner, project);
  const [folders, setFolders] = useState({});
  const [expanded, setExpanded] = useState({ "": true });

  async function load(path: string) {
    if (folders[path]) return;
    const items = await tree.children(path);
    setFolders((prev) => ({ ...prev, [path]: items }));
  }

  const state = { folders, expanded, value: String(value ?? "") };
  const actions = {
    load,
    toggle: (path: string) => setExpanded((prev) => ({ ...prev, [path]: !prev[path] })),
    choose: (path: string) => onChange(multiple ? [path] : path),
  };

  return (
    <div className="max-h-56 overflow-y-auto rounded-md border border-ui-border bg-ui-bg py-1" data-folder-picker="true">
      <div
        className={cx(
          "flex w-full items-center gap-1.5 py-[3px] pl-[6px] pr-2 text-[12px]",
          state.value === "" ? "bg-ui-bg-muted text-ui-text" : "text-ui-text-soft hover:bg-ui-bg-muted/60",
        )}
      >
        <button
          type="button"
          data-folder-option=""
          className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
          onClick={() => actions.choose("")}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinejoin="round" className="h-3.5 w-3.5 shrink-0">
            <path d="M4 20a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.7.9l.8 1.2a2 2 0 0 0 1.7.9H20a2 2 0 0 1 2 2v1H6.5L4 20Z" />
          </svg>
          <span className="truncate font-medium">{rootLabel || "Project root"}</span>
        </button>
      </div>
      <PickerLevel path="" depth={1} state={state} actions={actions} />
    </div>
  );
}
