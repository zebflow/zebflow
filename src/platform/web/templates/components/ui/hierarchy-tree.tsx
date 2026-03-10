function toAttrProps(attrs) {
  const out = {};
  if (!attrs || typeof attrs !== "object") {
    return out;
  }
  Object.entries(attrs).forEach(([key, value]) => {
    if (!key) return;
    if (value === null || typeof value === "undefined" || value === false) return;
    out[key] = value === true ? "true" : String(value);
  });
  return out;
}

export interface HierarchyTreeItem {
  id?: string;
  icon?: ReactNode;
  label: ReactNode;
  badge?: ReactNode;
  href?: string;
  active?: boolean;
  expanded?: boolean;
  className?: string;
  rowClassName?: string;
  attrs?: Record<string, string | number | boolean>;
  rowAttrs?: Record<string, string | number | boolean>;
  children?: HierarchyTreeItem[];
  content?: ReactNode;
}

interface HierarchyTreeProps {
  items?: HierarchyTreeItem[];
  className?: string;
  listClassName?: string;
  emptyLabel?: string;
  defaultExpandedDepth?: number;
}

function renderTreeNode(node, depth, defaultExpandedDepth) {
  const children = Array.isArray(node?.children) ? node.children : [];
  const isBranch = children.length > 0;
  const liAttrs = toAttrProps(node?.attrs);
  const rowAttrs = toAttrProps(node?.rowAttrs);

  if (!isBranch) {
    const leafClass = cx("project-tree-leaf-link", node?.rowClassName, node?.active ? "is-active" : "");
    const content = node?.content ? (
      node.content
    ) : node?.href ? (
      <a href={node.href} className={leafClass} {...rowAttrs}>
        {node?.icon ? <span className="project-tree-icon">{node.icon}</span> : null}
        <span className="project-tree-segment">{node?.label}</span>
        {node?.badge ? <span className="project-tree-meta">{node.badge}</span> : null}
      </a>
    ) : (
      <div className={leafClass} {...rowAttrs}>
        {node?.icon ? <span className="project-tree-icon">{node.icon}</span> : null}
        <span className="project-tree-segment">{node?.label}</span>
        {node?.badge ? <span className="project-tree-meta">{node.badge}</span> : null}
      </div>
    );
    return (
      <li key={node?.id ?? `${depth}:${String(node?.label ?? "")}`} className={cx("project-tree-leaf", node?.className)} {...liAttrs}>
        {content}
      </li>
    );
  }

  const open = typeof node?.expanded === "boolean" ? node.expanded : depth <= defaultExpandedDepth;
  return (
    <li key={node?.id ?? `${depth}:${String(node?.label ?? "")}`} className={cx("project-tree-branch", node?.className)} {...liAttrs}>
      <details className="project-tree-details" open={open}>
        <summary className={cx("project-tree-summary", node?.rowClassName)} {...rowAttrs}>
          <span className="project-tree-caret">
            <svg viewBox="0 0 24 24" fill="none" width="14" height="14" aria-hidden="true">
              <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          </span>
          {node?.icon ? <span className="project-tree-icon">{node.icon}</span> : null}
          <span className="project-tree-segment">{node?.label}</span>
          {node?.badge ? <span className="project-tree-meta">{node.badge}</span> : null}
        </summary>
        <div className="project-tree-body">
          <ul className="project-tree-list">
            {children.map((child) => renderTreeNode(child, depth + 1, defaultExpandedDepth))}
          </ul>
        </div>
      </details>
    </li>
  );
}

export default function HierarchyTree(props: HierarchyTreeProps) {
  const items = Array.isArray(props?.items) ? props.items : [];
  if (items.length === 0) {
    return <p className="project-tree-empty">{props?.emptyLabel ?? "No items."}</p>;
  }
  return (
    <ul className={cx("project-tree-root", props?.listClassName, props?.className)}>
      {items.map((item) => renderTreeNode(item, 1, props?.defaultExpandedDepth ?? 1))}
    </ul>
  );
}
