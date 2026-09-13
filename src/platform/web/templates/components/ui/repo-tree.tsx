import { useEffect, useState } from "zeb/react";
import RepoTreeRow from "@/components/ui/repo-tree-row";
import { ancestorsOf, repoTree } from "@/components/lib/repo-tree";

/** The repository itself, drawn as the first row so it is not a special case. */
function rootItem(label) {
  return { name: label || "repository", rel_path: "", kind: "root" };
}

/**
 * One folder's contents, and the folders opened inside it.
 *
 * Recursive, and each level fetches only when it is opened — so drawing the
 * tree costs what the reader has actually looked at, not the whole repository.
 */
function Level({ path, depth, state, actions }) {
  const items = state.folders[path];

  useEffect(() => {
    actions.load(path);
  }, [path]);

  if (!items) {
    return (
      <p className="py-1 text-[11px] text-muted-foreground" style={{ paddingLeft: `${depth * 12 + 26}px` }}>
        Loading…
      </p>
    );
  }
  if (!items.length) {
    return (
      <p className="py-1 text-[11px] text-muted-foreground" style={{ paddingLeft: `${depth * 12 + 26}px` }}>
        Empty
      </p>
    );
  }

  return (
    <>
      {items.map((item) => {
        const open = !!state.expanded[item.rel_path];
        return (
          <div key={item.rel_path}>
            <RepoTreeRow
              item={item}
              depth={depth}
              open={open}
              active={state.selected === item.rel_path}
              status={state.gitStatus[item.rel_path]}
              menu={actions.menuFor(item)}
              onActivate={actions.activate}
            />
            {item.kind === "folder" && open ? (
              <Level path={item.rel_path} depth={depth + 1} state={state} actions={actions} />
            ) : null}
          </div>
        );
      })}
    </>
  );
}

/**
 * The repository as a tree the reader opens a folder at a time.
 *
 * Every row — the root included — carries the same `⋯`, so "what can I do
 * here" has one answer and one place to find it. Selecting a folder is how you
 * say where the next thing should be made.
 *
 * `refreshToken` is how a caller says something was written: change it and the
 * folders already on screen are re-read. Nothing else is.
 */
export default function RepoTree({ owner, project, rootLabel, selected, onSelect, onAction, refreshToken }) {
  const tree = repoTree(owner, project);
  const [folders, setFolders] = useState({});
  const [expanded, setExpanded] = useState({ "": true });
  const [gitStatus, setGitStatus] = useState({});

  const root = rootItem(rootLabel);

  async function load(path: string, force = false) {
    if (force) tree.invalidate(path);
    else if (folders[path]) return;
    const items = await tree.children(path);
    setFolders((prev) => ({ ...prev, [path]: items }));
  }

  // Opening a file deep in the tree should show it rather than leave the
  // reader to go looking: every folder on the way down is opened.
  useEffect(() => {
    if (!selected) return;
    const parents = ancestorsOf(selected);
    if (!parents.length) return;
    setExpanded((prev) => {
      const next = { ...prev };
      for (const parent of parents) next[parent] = true;
      return next;
    });
  }, [selected]);

  // Which files git considers changed. One request for the repository, because
  // that is the only shape the question has — git does not answer it per folder.
  useEffect(() => {
    let live = true;
    tree
      .gitStatus()
      .then((map) => {
        if (live) setGitStatus(map);
      })
      .catch(() => {
        // A repository without git is not an error worth showing in a tree.
      });
    return () => {
      live = false;
    };
  }, [refreshToken]);

  // Something was written. Re-read the folders already on screen — at most what
  // the reader has opened, never the whole repository.
  useEffect(() => {
    if (!refreshToken) return;
    tree.invalidateAll();
    for (const path of Object.keys(folders)) load(path, true);
  }, [refreshToken]);

  function refreshAll() {
    tree.invalidateAll();
    for (const path of Object.keys(folders)) load(path, true);
  }

  const CREATE = [
    { kind: "new-file", label: "New file" },
    { kind: "new-pipeline", label: "New pipeline" },
    { kind: "new-folder", label: "New folder" },
  ];

  const actions = {
    load,
    /** A click picks the row; a folder or the root also opens or closes. */
    activate: (item) => {
      onSelect?.(item);
      if (item.kind === "folder" || item.kind === "root") {
        setExpanded((prev) => ({ ...prev, [item.rel_path]: !prev[item.rel_path] }));
      }
    },
    menuFor: (item) => {
      const inHere = (kind: string) => () => {
        // Open first: a new file appearing inside a closed folder looks like
        // nothing happened.
        setExpanded((prev) => ({ ...prev, [item.rel_path]: true }));
        onAction?.(kind, item);
      };

      // Opening a folder is the first thing offered, because seeing what is
      // inside is what a reader most often wants from one — and the panel
      // beside the tree already knows how to list a folder.
      const open = { label: "Open folder", onSelect: () => onAction?.("open-folder", item) };
      const add = { label: "Add from hub…", onSelect: inHere("add") };

      if (item.kind === "root") {
        return [
          open,
          { separator: true },
          ...CREATE.map((entry) => ({ label: entry.label, onSelect: inHere(entry.kind) })),
          add,
          { separator: true },
          { label: "Refresh", onSelect: refreshAll },
          { label: "Collapse all", onSelect: () => setExpanded({ "": true }) },
        ];
      }

      if (item.kind === "folder") {
        return [
          open,
          { separator: true },
          ...CREATE.map((entry) => ({ label: `${entry.label} here`, onSelect: inHere(entry.kind) })),
          add,
          { separator: true },
          { label: "Rename…", onSelect: () => onAction?.("rename", item) },
          {
            label: item.is_protected ? "Declared by the layout" : "Delete folder",
            variant: item.is_protected ? undefined : "destructive",
            disabled: !!item.is_protected,
            onSelect: () => onAction?.("delete", item),
          },
        ];
      }

      return [
        { label: "Open", onSelect: () => onSelect?.(item) },
        { separator: true },
        { label: "Rename…", onSelect: () => onAction?.("rename", item) },
        { label: "Duplicate", onSelect: () => onAction?.("duplicate", item) },
        { label: "Delete file", variant: "destructive", onSelect: () => onAction?.("delete", item) },
      ];
    },
  };

  const state = { folders, expanded, selected, gitStatus };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto py-1" data-repo-tree="true">
      <RepoTreeRow
        item={root}
        depth={0}
        open={!!expanded[""]}
        active={selected === ""}
        menu={actions.menuFor(root)}
        onActivate={actions.activate}
      />
      {expanded[""] ? <Level path="" depth={1} state={state} actions={actions} /> : null}
    </div>
  );
}
