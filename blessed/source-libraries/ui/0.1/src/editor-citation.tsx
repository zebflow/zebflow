import { defineExtension, routeSearch } from "zeb/ui/editor-extension";

/**
 * Citation extension — cite a work inline; the rendered document numbers
 * each citation by first appearance and lists the references at the end.
 *
 *   citationExtension({ route: "/api/works/search" })
 *
 * `/cite` opens a picker fed by the route (`GET route?q=…` → a JSON array of
 * `{ id, label, href, snapshot }`). The route decides what a query means — a
 * DOI to resolve, a title to search in the project's own library — and
 * answers a snapshot `{ authors, year, title, container, doi, url }`. The
 * node keeps it, so rendering never asks again. In the editor a citation
 * shows its label (`[Example 2020]`); on the page it is `[1]`, linked to
 * its entry under the references heading.
 */

function authorsOf(snapshot) {
  const list = Array.isArray(snapshot.authors) ? snapshot.authors : snapshot.authors ? [snapshot.authors] : [];
  return list.join(", ");
}

function reference(item, r) {
  const s = item.snapshot || {};
  const head = [authorsOf(s), s.year ? `(${s.year})` : ""].filter(Boolean).join(" ");
  const link = s.doi ? { href: `https://doi.org/${s.doi}`, text: `doi:${s.doi}` } : s.url || item.href ? { href: s.url || item.href, text: s.url || item.href } : null;
  return [
    head ? `${head}. ` : "",
    s.title ? r.h("cite", {}, s.title) : item.label || item.id,
    s.container ? `. ${s.container}` : "",
    ". ",
    link ? r.h("a", { class: "text-info underline underline-offset-4", href: link.href }, link.text) : null,
  ];
}

export function citationExtension({ name = "citation", route, search, label = "Citation", hint = "Cite a work by DOI or title", heading = "References" } = {}) {
  if (!route && !search) throw new Error(`citationExtension "${name}" needs a route (or a search function) for its picker`);
  return defineExtension({
    name,
    node: {
      group: "inline", inline: true, atom: true, selectable: true,
      attrs: { id: { default: "" }, label: { default: "" }, href: { default: null }, snapshot: { default: null } },
    },
    render: (node, r) => {
      const a = node.attrs;
      const n = r.collect(name, a.id, a);
      if (n === null) return r.h("span", { class: "rounded bg-muted px-1 text-sm text-muted-foreground", "data-citation": name, "data-id": a.id }, `[${a.label || a.id}]`);
      return r.h("sup", { "data-citation": name, "data-id": a.id }, r.h("a", { class: "text-info no-underline", href: `#${name}-${n}` }, `[${n}]`));
    },
    parse: [{
      tag: `[data-citation="${name}"]`,
      getAttrs: (dom) => ({ id: dom.getAttribute("data-id") || "", label: (dom.textContent || "").replace(/^\[|\]$/g, "") }),
    }],
    text: (node) => `[${node.attrs.label || node.attrs.id}]`,
    footer: (r) => {
      const items = r.collected(name);
      if (items.length === 0) return null;
      return r.h(
        "section",
        { class: "mt-10 border-t border-border pt-4", "data-references": name },
        r.h("h2", { class: "mb-2 text-lg font-semibold" }, heading),
        r.h("ol", { class: "list-decimal space-y-1 pl-6 text-sm text-muted-foreground" }, items.map(({ n, item }) => r.h("li", { id: `${name}-${n}` }, reference(item, r)))),
      );
    },
    picker: { search: search || routeSearch(route), toAttrs: (item) => ({ id: String(item.id), label: item.label || String(item.id), href: item.href || null, snapshot: item.snapshot || null }) },
    insert: [{ id: name, label, hint, keys: `cite citation doi reference ${name}`, run: (api) => api.pick() }],
  });
}

export default citationExtension;
