import HierarchyTree, { HierarchyTreeItem } from "@/components/ui/hierarchy-tree";

function iconFolder() {
  return (
    <svg viewBox="0 0 24 24" fill="none" width="16" height="16" aria-hidden="true">
      <path d="M4 7h6l2 2h8v8H4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
    </svg>
  );
}

function iconRoute() {
  return (
    <svg viewBox="0 0 24 24" fill="none" width="16" height="16" aria-hidden="true">
      <circle cx="6" cy="12" r="2" stroke="currentColor" strokeWidth="1.7" />
      <circle cx="18" cy="6" r="2" stroke="currentColor" strokeWidth="1.7" />
      <circle cx="18" cy="18" r="2" stroke="currentColor" strokeWidth="1.7" />
      <path d="M8 12h4l4-6M12 12h4l-4 6" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function normalizePath(raw) {
  const path = String(raw || "").trim();
  if (!path || path === "/") {
    return "/";
  }
  return `/${path.replace(/^\/+|\/+$/g, "")}`;
}

function splitPath(path) {
  const normalized = normalizePath(path);
  if (normalized === "/") {
    return [];
  }
  return normalized.slice(1).split("/").filter(Boolean);
}

function methodClass(method) {
  return `project-tree-method is-${String(method || "GET").toLowerCase()}`;
}

function buildRouteTree(items) {
  const root = {
    key: "__root__",
    segment: "/",
    children: new Map(),
    endpoints: [],
  };

  (Array.isArray(items) ? items : []).forEach((item, index) => {
    const href = String(item?.editor_href || "#");
    if (href.includes("{{")) return;
    const route = normalizePath(item?.webhook_path || "/");
    if (!route || route.includes("{{")) return;

    const segments = splitPath(route);
    let cursor = root;
    segments.forEach((segment) => {
      if (!cursor.children.has(segment)) {
        cursor.children.set(segment, {
          key: `${cursor.key}/${segment}`,
          segment,
          children: new Map(),
          endpoints: [],
        });
      }
      cursor = cursor.children.get(segment);
    });
    cursor.endpoints.push({
      ...item,
      webhook_path: route,
      __index: index,
    });
  });

  const finalize = (node) => ({
    key: node.key,
    segment: node.segment,
    endpoints: node.endpoints.sort((a, b) =>
      String(a?.webhook_path || "").localeCompare(String(b?.webhook_path || ""))
    ),
    children: Array.from(node.children.values())
      .map(finalize)
      .sort((a, b) => String(a.segment || "").localeCompare(String(b.segment || ""))),
  });

  return finalize(root);
}

function endpointNode(item, index) {
  const method = String(item?.webhook_method || "GET").toUpperCase();
  const route = String(item?.webhook_path || "/");
  const title = String(item?.title || item?.name || "pipeline");
  const name = String(item?.name || "");
  const metaLabel = name && title !== name ? `${title} - ${name}` : title;
  return {
    id: `webhook-endpoint-${index}-${route}`,
    className: "project-tree-leaf",
    content: (
      <a href={item?.editor_href || "#"} className="project-tree-leaf-link">
        <span className="project-tree-icon">{iconRoute()}</span>
        <span className={methodClass(method)}>{method}</span>
        <span className="project-tree-route">{route}</span>
        <span className="project-tree-meta">{metaLabel}</span>
      </a>
    ),
  } as HierarchyTreeItem;
}

function branchNode(node, depth = 1) {
  const children = [];
  (Array.isArray(node?.endpoints) ? node.endpoints : []).forEach((endpoint, index) => {
    children.push(endpointNode(endpoint, index));
  });
  (Array.isArray(node?.children) ? node.children : []).forEach((child) => {
    children.push(branchNode(child, depth + 1));
  });
  return {
    id: `webhook-branch-${node?.key || node?.segment || "node"}`,
    label: String(node?.segment || "/"),
    icon: iconFolder(),
    expanded: depth <= 2,
    children,
  } as HierarchyTreeItem;
}

interface WebhookRouteTreeProps {
  items?: any[];
}

export default function WebhookRouteTree(props: WebhookRouteTreeProps) {
  const tree = buildRouteTree(props?.items || []);
  const nodes: HierarchyTreeItem[] = [];
  (Array.isArray(tree?.endpoints) ? tree.endpoints : []).forEach((endpoint, index) => {
    nodes.push(endpointNode(endpoint, index));
  });
  (Array.isArray(tree?.children) ? tree.children : []).forEach((child) => {
    nodes.push(branchNode(child, 1));
  });
  return (
    <HierarchyTree
      items={nodes}
      className="project-webhook-tree-root"
      emptyLabel="No webhook routes found."
      defaultExpandedDepth={2}
    />
  );
}
