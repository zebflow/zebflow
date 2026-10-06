import { defineExtension, routeSearch } from "zeb/ui/editor-extension";

/**
 * Embed extension — a card for a record that lives elsewhere, stored as its
 * key plus a snapshot, so the page renders without fetching anything.
 *
 *   embedExtension({ name: "product", label: "Product", route: "/api/products/search" })
 *
 * `/product` opens a picker fed by the route (`GET route?q=…` → a JSON array
 * of `{ id, label, href, snapshot }`); the chosen item is stored as
 * `{ key: id, snapshot: { title, description, image, href, meta } }`. A
 * pipeline that owns the records may rewrite the snapshot of every node with
 * that `type` and `key` when the record changes. Without a route the card is
 * filled in by hand in its panel.
 */

export function embedExtension({ name = "embed", label = "Embed", hint = "A card for a linked record", route, search } = {}) {
  return defineExtension({
    name,
    node: { group: "block", atom: true, draggable: true, attrs: { key: { default: "" }, snapshot: { default: {} } } },
    render: (node, r) => {
      const s = node.attrs.snapshot || {};
      const body = r.h(
        "span",
        { class: "min-w-0 flex-1" },
        r.h("strong", { class: "block truncate font-semibold" }, s.title || node.attrs.key || "Untitled"),
        s.description ? r.h("span", { class: "mt-0.5 block text-sm text-muted-foreground" }, s.description) : null,
        s.meta ? r.h("small", { class: "mt-1 block text-xs text-muted-foreground" }, s.meta) : null,
      );
      const image = s.image ? r.h("img", { class: "h-20 w-28 shrink-0 rounded-md object-cover", src: s.image, alt: "" }) : null;
      const attrs = { class: "my-4 flex gap-4 rounded-lg border border-border bg-card p-3 text-card-foreground no-underline", "data-embed": name, "data-key": node.attrs.key };
      return s.href ? r.h("a", { ...attrs, href: s.href }, image, body) : r.h("div", attrs, image, body);
    },
    text: (node) => (node.attrs.snapshot && node.attrs.snapshot.title) || node.attrs.key,
    picker: route || search ? { search: search || routeSearch(route), toAttrs: (item) => ({ key: String(item.id), snapshot: { title: item.label, href: item.href || "", ...(item.snapshot || {}) } }) } : null,
    insert: [{ id: name, label, hint, keys: `embed card ${name}`, run: (api) => (route || search ? api.pick() : api.insert(name)) }],
    fields: [
      { name: "key", label: "Key" },
      { name: "snapshot.title", label: "Title" },
      { name: "snapshot.description", label: "Description" },
      { name: "snapshot.href", label: "Link", type: "url" },
      { name: "snapshot.image", label: "Image", type: "url" },
      { name: "snapshot.meta", label: "Meta" },
    ],
  });
}

export default embedExtension;
