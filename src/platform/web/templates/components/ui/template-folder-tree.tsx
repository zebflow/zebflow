import HierarchyTree, { HierarchyTreeItem } from "@/components/ui/hierarchy-tree";

function cx(...parts) {
  return parts.filter(Boolean).join(" ");
}

function dirname(relPath) {
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

  if (item?.kind === "folder") {
    return <FolderIcon />;
  }
  if (isTsx || item?.file_kind === "page" || item?.file_kind === "component") {
    return <ReactFileIcon />;
  }
  if (isTs || item?.file_kind === "script") {
    return <TsFileIcon />;
  }
  if (isCss || item?.file_kind === "style") {
    return <i className="zf-devicon devicon-css3-plain colored" aria-hidden="true"></i>;
  }
  return <i className="zf-devicon zf-icon-file" aria-hidden="true"></i>;
}

function toTemplateHierarchy(items, selectedFile, selectedFolder) {
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
    const isOpenFile = !isFolder && relPath === selectedFile;
    const isSelected = isFolder ? relPath === selectedFolder : isOpenFile;
    const rowClassName = cx(
      "template-tree-item",
      isFolder ? "is-folder" : "",
      item?.is_protected ? "is-protected" : "",
      isOpenFile ? "is-open" : "",
      isSelected ? "is-selected" : ""
    );

    const node: HierarchyTreeItem = {
      id: relPath || `${item?.kind || "node"}-${String(item?.name || "")}`,
      icon: <span className="template-tree-icon">{iconForTemplateItem(item)}</span>,
      label: <span className="template-tree-label">{item?.name || "item"}</span>,
      badge: <span className="template-tree-git"></span>,
      href: !isFolder ? item?.href || "#" : undefined,
      className: rowClassName,
      rowClassName,
      expanded: isFolder,
      attrs: {
        "data-template-rel-path": relPath,
        "data-template-protected": item?.is_protected ? "true" : "false",
        ...(isFolder ? { "data-template-folder-item": "true" } : { "data-template-file-item": "true" }),
        draggable: "true",
      },
      children: [],
    };

    byPath.set(relPath, node);
  });

  sorted.forEach((item) => {
    const relPath = String(item?.rel_path || "");
    const parentPath = dirname(relPath);
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
  selectedFile?: string;
  selectedFolder?: string;
}

export default function TemplateFolderTree(props: TemplateFolderTreeProps) {
  const nodes = toTemplateHierarchy(
    props?.items || [],
    props?.selectedFile || "",
    props?.selectedFolder || ""
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
