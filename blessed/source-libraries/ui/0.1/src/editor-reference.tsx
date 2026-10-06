import { defineExtension, routeSearch } from "zeb/ui/editor-extension";

/**
 * Reference extension — a link to another document of the project (an
 * article, a page, any collection the project has), found by searching from
 * the editor. Like a mention it is configured, not built in: the project
 * names the kind, the route that searches it and, if it likes, how it looks.
 *
 *   referenceExtension({ name: "article", label: "Link to article", route: "/api/articles/search" })
 *   referenceExtension({ name: "page_card", variant: "card", route: "/api/pages/search" })
 *
 * `/` + the label opens a picker with its own search box (or type the
 * `trigger` character, when one is given). The route answers `GET route?q=…`
 * with a JSON array of `{ id, label, href, snapshot }`; the node stores
 * `{ key: id, label, href, snapshot }`, so a page renders the link without
 * asking again, and a pipeline that owns the documents may rewrite the
 * snapshot (a renamed title, a moved address) of every node with that type
 * and key. `variant: "link"` (default) is an inline link; `"card"` is a small
 * block card with the snapshot's `description` and `meta`.
 */

export function referenceExtension({ name = "reference", variant = "link", trigger = null, route, search, render, label = "Link to document", hint = "Search the project's documents" } = {}) {
  if (!route && !search) throw new Error(`referenceExtension "${name}" needs a route (or a search function) for its picker`);
  if (variant !== "link" && variant !== "card") throw new Error(`referenceExtension "${name}": variant is "link" or "card"`);
  const card = variant === "card";
  const attrs = { key: { default: "" }, label: { default: "" }, href: { default: null }, snapshot: { default: null } };
  return defineExtension({
    name,
    node: card
      ? { group: "block", atom: true, draggable: true, attrs }
      : { group: "inline", inline: true, atom: true, selectable: true, attrs },
    render: render || ((node, r) => {
      const a = node.attrs;
      const s = a.snapshot || {};
      const title = a.label || s.title || a.key;
      const base = { "data-reference": name, "data-key": a.key, title: s.description || undefined };
      const tag = a.href ? "a" : "span";
      if (!card) return r.h(tag, { ...base, href: a.href || undefined, class: "text-info underline underline-offset-4" }, title);
      return r.h(
        tag,
        { ...base, href: a.href || undefined, class: "my-3 block rounded-md border border-border bg-card px-3 py-2 text-card-foreground no-underline hover:bg-accent" },
        r.h("strong", { class: "block truncate text-sm font-semibold" }, title),
        s.description ? r.h("span", { class: "block truncate text-xs text-muted-foreground" }, s.description) : null,
        s.meta ? r.h("small", { class: "block text-xs text-muted-foreground" }, s.meta) : null,
      );
    }),
    parse: card ? undefined : [{
      tag: `[data-reference="${name}"]`,
      getAttrs: (dom) => ({ key: dom.getAttribute("data-key") || "", label: dom.textContent || "", href: dom.getAttribute("href") }),
    }],
    text: (node) => node.attrs.label || node.attrs.key,
    trigger: trigger || undefined,
    picker: { search: search || routeSearch(route), toAttrs: (item) => ({ key: String(item.id), label: item.label || String(item.id), href: item.href || null, snapshot: item.snapshot || null }) },
    insert: [{ id: name, label, hint, keys: `link reference ${name} ${variant}`, run: (api) => api.pick() }],
  });
}

export default referenceExtension;
