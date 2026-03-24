import { cx } from "zeb";
import Checkbox from "@/components/ui/checkbox";

function gitStatusChar(code) {
  if (code === "??") return "U";
  const trimmed = String(code || "").replace(/\s/g, "");
  return trimmed[0] || "M";
}

function buildGitFileTree(files) {
  const byPath = new Map();
  const roots = [];

  files.forEach((f) => {
    const parts = String(f.rel_path || "").split("/").filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const dirPath = parts.slice(0, i).join("/");
      if (!byPath.has(dirPath)) {
        byPath.set(dirPath, { id: dirPath, name: parts[i - 1], isDir: true, children: [] });
      }
    }
    byPath.set(f.rel_path, {
      id: f.rel_path,
      name: parts[parts.length - 1] || f.rel_path,
      isDir: false,
      file: f,
      children: [],
    });
  });

  byPath.forEach((node, path) => {
    const lastSlash = path.lastIndexOf("/");
    if (lastSlash > 0) {
      const parentPath = path.slice(0, lastSlash);
      if (byPath.has(parentPath)) {
        byPath.get(parentPath).children.push(node);
        return;
      }
    }
    roots.push(node);
  });

  return roots;
}

function sortGitNodes(nodes) {
  return [...nodes]
    .sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    })
    .map((n) => ({ ...n, children: sortGitNodes(n.children) }));
}

function GitTreeNodes({ nodes, setFiles }) {
  return (
    <>
      {nodes.map((node) =>
        node.isDir ? (
          <li key={node.id} className="project-tree-branch">
            <details className="project-tree-details" open>
              <summary className="project-tree-summary git-tree-dir">
                <span className="project-tree-caret">
                  <svg viewBox="0 0 24 24" fill="none" width="12" height="12" aria-hidden="true">
                    <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </span>
                <span className="project-tree-segment">{node.name}</span>
              </summary>
              <div className="project-tree-body">
                <ul className="project-tree-list">
                  <GitTreeNodes nodes={sortGitNodes(node.children)} setFiles={setFiles} />
                </ul>
              </div>
            </details>
          </li>
        ) : (
          <li key={node.id} className="project-tree-leaf">
            <div className="project-tree-leaf-link git-tree-file">
              <Checkbox
                checked={node.file.checked}
                onChange={(e) =>
                  setFiles((prev) =>
                    prev.map((x) =>
                      x.rel_path === node.file.rel_path ? { ...x, checked: e.target.checked } : x
                    )
                  )
                }
                className="git-tree-check"
              />
              <span className="git-tree-file-name" title={node.file.rel_path}>{node.name}</span>
              <code className={cx(
                "git-tree-code",
                gitStatusChar(node.file.code) === "A" && "is-added",
                gitStatusChar(node.file.code) === "D" && "is-deleted",
                gitStatusChar(node.file.code) === "M" && "is-modified",
                gitStatusChar(node.file.code) === "U" && "is-untracked",
              )}>
                {gitStatusChar(node.file.code)}
              </code>
            </div>
          </li>
        )
      )}
    </>
  );
}

export function GitFileTree({ files, setFiles }) {
  const roots = sortGitNodes(buildGitFileTree(files));
  if (roots.length === 0) return null;
  return (
    <ul className="project-tree-root git-tree-root">
      <GitTreeNodes nodes={roots} setFiles={setFiles} />
    </ul>
  );
}
