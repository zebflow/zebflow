import { defineExtension, routeSearch } from "zeb/ui/editor-extension";

/**
 * Mention extension — a trigger character opens a picker fed by a project
 * route, and the chosen item becomes an inline link that keeps its own
 * snapshot. Zebflow ships the mechanism only: what a mention points at, where
 * the suggestions come from and how it looks are the project's.
 *
 *   mentionExtension({ name: "person", trigger: "@", route: "/api/people/search" })
 *   mentionExtension({ name: "org", trigger: "+", route: "/api/orgs/search" })
 *
 * The route answers `GET route?q=…` with a JSON array of
 * `{ id, label, href, snapshot }`. The node stores all four, so a page
 * renders the mention without asking the route again. Each kind is its own
 * extension: its own `name` (the node type), its own `trigger` (one
 * character, unique among the list; `null` for slash-menu only) and its own
 * source. `render(node, r)` replaces the default chip.
 */

export function mentionExtension({ name = "mention", trigger = "@", route, search, render, label = "Mention", hint = "Link a person or a page" } = {}) {
  if (!route && !search) throw new Error(`mentionExtension "${name}" needs a route (or a search function) for its picker`);
  if (trigger !== null && (typeof trigger !== "string" || trigger.length !== 1 || /\s/.test(trigger))) {
    throw new Error(`mentionExtension "${name}": trigger is one character, or null`);
  }
  const prefix = trigger || "";
  return defineExtension({
    name,
    node: {
      group: "inline", inline: true, atom: true, selectable: true,
      attrs: { id: { default: "" }, label: { default: "" }, href: { default: null }, snapshot: { default: null } },
    },
    render: render || ((node, r) => {
      const a = node.attrs;
      const attrs = { class: "rounded bg-accent px-1 font-medium text-accent-foreground no-underline", "data-mention": name, "data-id": a.id };
      const text = `${prefix}${a.label || a.id}`;
      return a.href ? r.h("a", { ...attrs, href: a.href }, text) : r.h("span", attrs, text);
    }),
    parse: [{
      tag: `[data-mention="${name}"]`,
      getAttrs: (dom) => ({ id: dom.getAttribute("data-id") || "", label: (dom.textContent || "").replace(prefix, ""), href: dom.getAttribute("href") }),
    }],
    text: (node) => `${prefix}${node.attrs.label || node.attrs.id}`,
    trigger: trigger || undefined,
    picker: { search: search || routeSearch(route), toAttrs: (item) => ({ id: String(item.id), label: item.label || String(item.id), href: item.href || null, snapshot: item.snapshot || null }) },
    insert: [{ id: name, label, hint, keys: `mention ${name} ${prefix}`, run: (api) => api.pick() }],
  });
}

export default mentionExtension;
