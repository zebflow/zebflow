import { buildPathTree, renderCollapsibleTree } from "/assets/platform/ui-collapsible-tree.mjs";

function readWebhookItems(root) {
  return Array.from(root.querySelectorAll("[data-webhook-item='true']"))
    .map((node) => ({
      path: node.getAttribute("data-webhook-route") || "/",
      method: node.getAttribute("data-webhook-method") || "GET",
      title: node.getAttribute("data-webhook-title") || "",
      name: node.getAttribute("data-webhook-name") || "",
      kind: node.getAttribute("data-webhook-kind") || "webhook",
      href: node.getAttribute("href") || "#",
    }))
    .filter((item) =>
      !String(item.path || "").includes("{{") &&
      !String(item.href || "").includes("{{")
    );
}

function renderWebhookLeaf(item) {
  const wrapper = document.createElement("a");
  wrapper.href = String(item?.href || "#");
  wrapper.className = "project-tree-leaf-link";

  const method = document.createElement("span");
  method.className = `project-tree-method is-${String(item?.method || "GET").toLowerCase()}`;
  method.textContent = String(item?.method || "GET").toUpperCase();
  wrapper.appendChild(method);

  const route = document.createElement("span");
  route.className = "project-tree-route";
  route.textContent = String(item?.path || "/");
  wrapper.appendChild(route);

  const meta = document.createElement("span");
  meta.className = "project-tree-meta";
  const title = String(item?.title || item?.name || "pipeline");
  const name = String(item?.name || "");
  meta.textContent = name && title !== name ? `${title} • ${name}` : title;
  wrapper.appendChild(meta);

  return wrapper;
}

function initWebhookTree(root) {
  const target = root.querySelector("[data-webhook-tree-root='true']");
  if (!target) {
    return;
  }

  const items = readWebhookItems(root)
    .filter((item) => String(item.path || "").trim().length > 0)
    .sort((a, b) => String(a.path).localeCompare(String(b.path)));

  if (items.length === 0) {
    target.innerHTML = '<p class="project-tree-empty">No webhook routes found.</p>';
    return;
  }

  const tree = buildPathTree(items, (item) => item.path);
  renderCollapsibleTree(target, tree, {
    openDepth: 2,
    renderLeaf: renderWebhookLeaf,
  });
}

document.querySelectorAll("[data-webhook-tree='true']").forEach((root) => {
  initWebhookTree(root);
});
