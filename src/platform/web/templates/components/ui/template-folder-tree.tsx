import HierarchyTree, { HierarchyTreeItem } from "@/components/ui/hierarchy-tree";

function getParentDir(relPath) {
  if (!relPath || !String(relPath).includes("/")) {
    return "";
  }
  const path = String(relPath);
  return path.slice(0, path.lastIndexOf("/"));
}

function ReactFileIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="template-tech-icon is-react" aria-hidden="true">
      <circle cx="12" cy="12" r="1.6" fill="currentColor" />
      <ellipse cx="12" cy="12" rx="8.4" ry="3.4" stroke="currentColor" strokeWidth="1.6" />
      <ellipse cx="12" cy="12" rx="8.4" ry="3.4" stroke="currentColor" strokeWidth="1.6" transform="rotate(60 12 12)" />
      <ellipse cx="12" cy="12" rx="8.4" ry="3.4" stroke="currentColor" strokeWidth="1.6" transform="rotate(-60 12 12)" />
    </svg>
  );
}

function TsFileIcon() {
  return (
    <span className="template-tech-icon is-ts" aria-hidden="true">
      TS
    </span>
  );
}

function CssFileIcon() {
  return (
    <span className="template-tech-icon is-css" aria-hidden="true">
      CSS
    </span>
  );
}

function MdFileIcon() {
  return (
    <span className="template-tech-icon is-md" aria-hidden="true">
      MD
    </span>
  );
}

function FolderIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="template-tech-icon is-folder" aria-hidden="true">
      <path d="M4 7h6l2 2h8v8H4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
    </svg>
  );
}

function iconForTemplateItem(item) {
  const relPath = String(item?.rel_path || "").toLowerCase();
  const isTsx = relPath.endsWith(".tsx");
  const isTs = relPath.endsWith(".ts");
  const isCss = relPath.endsWith(".css");
  const isMd = relPath.endsWith(".md") || relPath.endsWith(".mdx");

  if (item?.kind === "folder") return <FolderIcon />;
  if (isTsx || item?.file_kind === "page" || item?.file_kind === "component") return <ReactFileIcon />;
  if (isTs || item?.file_kind === "script") return <TsFileIcon />;
  if (isCss || item?.file_kind === "style") return <CssFileIcon />;
  if (isMd) return <MdFileIcon />;
  return <i className="zf-devicon zf-icon-file" aria-hidden="true"></i>;
}

function toTemplateHierarchy(items, selectedPath, onSelectFile, onSelectFolder) {
  const safeItems = Array.isArray(items) ? items : [];
  const byPath = new Map();
  const roots = [];

  const sorted = [...safeItems].sort((a, b) => {
    const aKind = String(a?.kind || "");
    const bKind = String(b?.kind || "");
    if (aKind !== bKind) {
      if (aKind === "folder") return -1;
      if (bKind === "folder") return 1;
    }
    return String(a?.rel_path || "").localeCompare(String(b?.rel_path || ""));
  });

  sorted.forEach((item) => {
    const relPath = String(item?.rel_path || "");
    const isFolder = String(item?.kind || "") === "folder";
    const isSelected = relPath === selectedPath;
    const rowClassName = cx(
      "template-tree-item",
      isFolder ? "is-folder" : "",
      item?.is_protected ? "is-protected" : "",
      isSelected ? "is-selected" : ""
    );

    const node: HierarchyTreeItem = {
      id: relPath || `${item?.kind || "node"}-${String(item?.name || "")}`,
      icon: <span className="template-tree-icon">{iconForTemplateItem(item)}</span>,
      label: <span className="template-tree-label">{item?.name || "item"}</span>,
      badge: <span className="template-tree-git"></span>,
      className: "",
      rowClassName,
      expanded: isFolder,
      children: [],
      onClick: isFolder ? () => onSelectFolder(relPath) : undefined,
    };

    if (!isFolder) {
      node.content = (
        <div
          className={cx("project-tree-leaf-link", rowClassName)}
          onClick={() => onSelectFile(relPath)}
        >
          <span className="template-tree-icon">{iconForTemplateItem(item)}</span>
          <span className="template-tree-label">{item?.name || "item"}</span>
          {item?.is_protected ? (
            <span className="template-tree-lock" title="Protected">
              <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
                <path d="M8 11V8a4 4 0 118 0v3M7 11h10v9H7z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
              </svg>
            </span>
          ) : null}
          <span className="template-tree-git"></span>
        </div>
      );
    }

    byPath.set(relPath, node);
  });

  sorted.forEach((item) => {
    const relPath = String(item?.rel_path || "");
    const parentPath = getParentDir(relPath);
    const node = byPath.get(relPath);
    if (!node) return;
    if (parentPath && byPath.has(parentPath)) {
      const parent = byPath.get(parentPath);
      if (parent) {
        parent.children = [...(parent.children || []), node];
        return;
      }
    }
    roots.push(node);
  });

  const sortNodeChildren = (nodes) =>
    (Array.isArray(nodes) ? nodes : [])
      .map((node) => ({
        ...node,
        children: sortNodeChildren(node.children),
      }))
      .sort((a, b) => {
        const aFolder = Array.isArray(a?.children) && a.children.length > 0;
        const bFolder = Array.isArray(b?.children) && b.children.length > 0;
        if (aFolder !== bFolder) {
          return aFolder ? -1 : 1;
        }
        return String(a?.id || "").localeCompare(String(b?.id || ""));
      });

  return sortNodeChildren(roots);
}

interface TemplateFolderTreeProps {
  items?: any[];
  selectedPath?: string;
  onSelectFile?: (relPath: string) => void;
  onSelectFolder?: (relPath: string) => void;
}

export default function TemplateFolderTree(props: TemplateFolderTreeProps) {
  const nodes = toTemplateHierarchy(
    props?.items || [],
    props?.selectedPath || "",
    props?.onSelectFile || (() => {}),
    props?.onSelectFolder || (() => {})
  );
  return (
    <HierarchyTree
      items={nodes}
      className="template-tree-hierarchy"
      emptyLabel="No templates yet."
      defaultExpandedDepth={3}
    />
  );
}
