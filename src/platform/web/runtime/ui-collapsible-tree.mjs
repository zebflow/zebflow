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

export function buildPathTree(items, pathSelector = (item) => item?.path) {
  const root = {
    key: "__root__",
    segment: "/",
    children: new Map(),
    endpoints: [],
  };

  (Array.isArray(items) ? items : []).forEach((item, index) => {
    const path = normalizePath(pathSelector(item));
    const segments = splitPath(path);
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
      path,
      __index: index,
    });
  });

  const finalize = (node) => ({
    key: node.key,
    segment: node.segment,
    endpoints: node.endpoints.sort((a, b) => String(a.path).localeCompare(String(b.path))),
    children: Array.from(node.children.values())
      .map(finalize)
      .sort((a, b) => a.segment.localeCompare(b.segment)),
  });

  return finalize(root);
}

function renderEndpoint(item, renderLeaf) {
  const li = document.createElement("li");
  li.className = "project-tree-leaf";

  if (typeof renderLeaf === "function") {
    const custom = renderLeaf(item);
    if (custom instanceof Node) {
      li.appendChild(custom);
      return li;
    }
  }

  const link = document.createElement("a");
  link.href = String(item?.href || "#");
  link.className = "project-tree-leaf-link";
  link.textContent = String(item?.path || "/");
  li.appendChild(link);
  return li;
}

function renderBranch(node, options, depth) {
  const li = document.createElement("li");
  li.className = "project-tree-branch";

  const hasChildren = node.children.length > 0;
  const hasEndpoints = node.endpoints.length > 0;

  if (!hasChildren && !hasEndpoints) {
    return li;
  }

  if (!hasChildren && hasEndpoints) {
    const ul = document.createElement("ul");
    ul.className = "project-tree-list";
    node.endpoints.forEach((endpoint) => ul.appendChild(renderEndpoint(endpoint, options.renderLeaf)));
    li.appendChild(ul);
    return li;
  }

  const details = document.createElement("details");
  details.className = "project-tree-details";
  details.open = depth <= (options.openDepth ?? 1);

  const summary = document.createElement("summary");
  summary.className = "project-tree-summary";
  summary.innerHTML = `<span class="project-tree-caret">▾</span><span class="project-tree-segment"></span>`;
  summary.querySelector(".project-tree-segment").textContent = node.segment;
  details.appendChild(summary);

  const body = document.createElement("div");
  body.className = "project-tree-body";

  if (hasEndpoints) {
    const endpoints = document.createElement("ul");
    endpoints.className = "project-tree-list";
    node.endpoints.forEach((endpoint) => {
      endpoints.appendChild(renderEndpoint(endpoint, options.renderLeaf));
    });
    body.appendChild(endpoints);
  }

  if (hasChildren) {
    const children = document.createElement("ul");
    children.className = "project-tree-list";
    node.children.forEach((child) => {
      children.appendChild(renderBranch(child, options, depth + 1));
    });
    body.appendChild(children);
  }

  details.appendChild(body);
  li.appendChild(details);
  return li;
}

export function renderCollapsibleTree(container, tree, options = {}) {
  if (!(container instanceof Element)) {
    return;
  }
  const branchNodes = Array.isArray(tree?.children) ? tree.children : [];
  if (branchNodes.length === 0 && (!Array.isArray(tree?.endpoints) || tree.endpoints.length === 0)) {
    container.innerHTML = '<p class="project-tree-empty">No routes.</p>';
    return;
  }

  const root = document.createElement("ul");
  root.className = "project-tree-root";

  (Array.isArray(tree?.endpoints) ? tree.endpoints : []).forEach((endpoint) => {
    root.appendChild(renderEndpoint(endpoint, options.renderLeaf));
  });

  branchNodes.forEach((node) => {
    root.appendChild(renderBranch(node, options, 1));
  });

  container.innerHTML = "";
  container.appendChild(root);
}
