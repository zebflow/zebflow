/**
 * Editor extension — how a project adds a block, an inline node or a mark to
 * `zeb/ui/editor`, and how the same declaration renders it everywhere.
 *
 * One `render(node, r)` serves three places: the editor's live DOM (through
 * the engine's toDOM), `<DocumentView>` elements, and the `renderDocumentHtml`
 * string. All three build through `r.h`, which keeps only an allowlist of
 * tags and attributes — a script, a style, an `on…` handler or a
 * `javascript:` link never leaves this file — so the published HTML is
 * sanitized whatever an extension or a stored document says.
 *
 *   defineExtension({
 *     name: "note",
 *     node: { group: "block", content: "inline*", attrs: { tone: { default: "info" } } },
 *     render: (node, r) => r.h("p", { class: "my-2 border-l-2 pl-3", "data-note": node.attrs.tone }, r.content),
 *     parse: [{ tag: "p[data-note]", getAttrs: (dom) => ({ tone: dom.getAttribute("data-note") }) }],
 *     insert: [{ label: "Note", hint: "A short aside", keys: "note", run: (api) => api.insert("note") }],
 *   })
 *
 * A node may be drawn by a Zeb React component instead of `render` — see
 * `component` and `editComponent` below.
 *
 * This file is plain JavaScript (no JSX) on purpose: the server, the browser
 * and the engine load the same object.
 */

// Engine-defined names an extension may not reuse.
const BASE_NAMES = [
  "doc", "paragraph", "heading", "blockquote", "code_block", "horizontal_rule", "image", "bullet_list",
  "ordered_list", "todo_list", "list_item", "todo_item", "text", "hard_break", "unknown_block", "unknown_text",
  "unknown_inline", "link", "bold", "italic", "underline", "strike", "code",
];

// Elements whose content is never published: dropped with everything inside.
const DROPPED_TAGS = new Set([
  "script", "style", "iframe", "frame", "frameset", "object", "embed", "applet", "link", "meta", "base",
  "template", "noscript", "svg", "math", "form", "textarea", "select", "button", "option", "title", "head",
  "html", "body",
]);

// Elements a document may contain. Anything else is rendered as a `span`.
const ALLOWED_TAGS = new Set([
  "p", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "aside", "div", "span", "a", "strong", "b", "em", "i",
  "u", "s", "del", "ins", "code", "pre", "kbd", "mark", "small", "sub", "sup", "cite", "q", "abbr", "time", "br",
  "hr", "img", "figure", "figcaption", "ul", "ol", "li", "dl", "dt", "dd", "table", "thead", "tbody", "tfoot",
  "tr", "th", "td", "caption", "section", "header", "footer", "article", "details", "summary", "input", "label",
]);

const ALLOWED_ATTRS = new Set([
  "class", "id", "title", "alt", "href", "src", "width", "height", "colspan", "rowspan", "start", "rel",
  "target", "lang", "dir", "datetime", "cite", "loading", "role", "type", "checked", "disabled", "contenteditable",
]);
const URL_ATTRS = new Set(["href", "src", "cite"]);

/** Only links a browser can be trusted to follow. Anything else becomes `#`. */
export function safeHref(href) {
  const value = String(href || "").trim();
  if (/^(https?:|mailto:|tel:|\/|#|\.)/i.test(value)) return value;
  return "#";
}

/**
 * The attributes an element may publish, by the allowlist. `className` is
 * read as `class`; `contenteditable` survives only in the editor (`edit`).
 */
export function sanitizeAttrs(tag, attrs, edit) {
  const out = {};
  for (const [raw, value] of Object.entries(attrs || {})) {
    if (value === undefined || value === null || value === false) continue;
    const name = raw === "className" ? "class" : raw;
    if (!/^[a-z][a-z0-9-]*$/.test(name)) continue;
    const allowed = ALLOWED_ATTRS.has(name) || name.startsWith("data-") || name.startsWith("aria-");
    if (!allowed) continue;
    if (name === "contenteditable" && !(edit && value === "false")) continue;
    if (name === "type" && tag !== "input") continue;
    if (name === "target" && value !== "_blank") continue;
    out[name] = URL_ATTRS.has(name) ? safeHref(value) : value === true ? true : String(value);
  }
  if (tag === "input") { out.type = "checkbox"; if (!edit) out.disabled = true; }
  if (out.target) out.rel = "noopener noreferrer";
  return out;
}

/** A tag by the allowlist: `null` drops the element and its content. */
export function sanitizeTag(tag) {
  const name = String(tag || "").toLowerCase();
  if (DROPPED_TAGS.has(name)) return null;
  return ALLOWED_TAGS.has(name) ? name : "span";
}

function flatten(children, out = []) {
  for (const child of children) {
    if (Array.isArray(child) && typeof child[0] !== "string") flatten(child, out);
    else if (child !== null && child !== undefined && child !== false && child !== "") out.push(child);
  }
  return out;
}

/**
 * The editor's `r.h`: builds a ProseMirror DOM spec. `r.content` is the hole
 * (0) where the node's children go; it must be the only child of its element.
 */
function specH(tag, attrs, ...children) {
  const name = sanitizeTag(tag);
  if (!name) return "";
  return [name, sanitizeAttrs(name, attrs, true), ...flatten(children)];
}

function normalizeNodes(def) {
  const nodes = {};
  if (def.node) {
    nodes[def.name] = { spec: def.node, render: def.render, parse: def.parse, text: def.text, component: def.component, editComponent: def.editComponent };
  }
  for (const [type, entry] of Object.entries(def.nodes || {})) nodes[type] = entry;
  for (const [type, entry] of Object.entries(nodes)) checkComponents(def.name, type, entry);
  return nodes;
}

function normalizeMarks(def) {
  const marks = {};
  if (def.mark) {
    if (def.component || def.editComponent) throw new Error(`editor extension "${def.name}": a mark is drawn by render, not by a component`);
    marks[def.name] = { spec: def.mark, render: def.render, parse: def.parse };
  }
  for (const [type, entry] of Object.entries(def.marks || {})) marks[type] = entry;
  return marks;
}

/**
 * A node drawn by a component is an atom (its attrs are its content), and
 * its published look comes from exactly one place: `render` or `component`.
 */
function checkComponents(name, type, entry) {
  for (const key of ["component", "editComponent"]) {
    if (entry[key] !== undefined && typeof entry[key] !== "function") throw new Error(`editor extension "${name}": ${key} of "${type}" must be a component (a function)`);
  }
  if (!entry.component && !entry.editComponent) return;
  if (entry.component && entry.render) throw new Error(`editor extension "${name}": "${type}" has both render and component; the page draws it with one of them`);
  if (entry.spec && entry.spec.content) throw new Error(`editor extension "${name}": "${type}" is drawn by a component, so it is an atom: give it attrs, not content`);
}

/**
 * How a component node is written when ProseMirror itself serializes it
 * (copy and paste, `getHTML`): its attrs as JSON, read back by the same rule.
 */
function componentDom(type, inline) {
  const tag = inline ? "span" : "div";
  return {
    toDOM: (value) => [tag, { "data-zeb-node": type, "data-attrs": JSON.stringify(value.attrs) }],
    parse: [{
      tag: `${tag}[data-zeb-node="${type}"]`,
      getAttrs: (dom) => { try { return JSON.parse(dom.getAttribute("data-attrs") || "{}"); } catch { return false; } },
    }],
  };
}

/** What `r` holds while the editor draws a node: no numbering, the hole for content. */
const EDIT_CONTEXT = { h: specH, content: 0, edit: true, safeHref, collect: () => null, collected: () => [] };

/**
 * Declare an extension. Returns the object `<Editor extensions>`,
 * `<DocumentView extensions>` and `renderDocumentHtml(doc, { extensions })`
 * all take.
 *
 * def: {
 *   name,                       unique; also the node's type when `node` is given
 *   node | mark,                ProseMirror spec without toDOM/parseDOM
 *   render(node, r),            r: { h(tag, attrs, ...children), content, edit, collect, collected, safeHref }
 *   parse,                      ProseMirror parseDOM rules (optional)
 *   text(node),                 the node's plain-text form (atoms)
 *   nodes / marks,              more than one type: { type: { spec, render, parse, text } }
 *   footer(r),                  rendered once after the document (references…)
 *   insert,                     slash-menu items: [{ label, hint, keys, run(api) }]
 *   trigger, picker,            "@" and { search(query) → items, toAttrs(item), type? }
 *   fields, actions,            the panel for a node under the caret
 *   inputRules, keymap, plugins, commands   (schema, pm) → engine pieces
 *   component,                  a Zeb React component that draws the node (instead of render)
 *   editComponent,              a Zeb React component that draws it in the editor only
 *   options,                    the extension's configuration, handed to both components
 * }
 *
 * Components (`zeb/ui/editor-component` holds the machinery). A node drawn
 * by a component is an atom: no content, only attrs. Both get the same props:
 *
 *   { node: { type, attrs }, attrs, options, edit, selected, readOnly, update(patch) }
 *
 * `component` is the published look: `<DocumentView>` renders it (so the
 * page's SSR prints it and the page hydrates it), `renderDocumentHtml` prints
 * it with zeb/react's `renderToString` (the same bytes), and the editor draws
 * it too unless `editComponent` is given. Its output passes the same
 * tag/attribute/URL allowlist as `render`; function props such as `onClick`
 * are kept, since they come from code, never from a stored document. There
 * `edit` is false, `readOnly` true and `update` does nothing.
 *
 * `editComponent` is editor chrome: it runs only in the browser, as the
 * node's ProseMirror node view, is not sanitized, may use form controls, and
 * changes the node with `update({ attr: value })`. Pair it with `render` or
 * `component` for the published look.
 */
export function defineExtension(def) {
  if (!def || typeof def.name !== "string" || !/^[a-z][a-z0-9_]*$/.test(def.name)) {
    throw new Error("an editor extension needs a name: lowercase letters, digits and _");
  }
  const nodes = normalizeNodes(def);
  const marks = normalizeMarks(def);
  const toSpec = (entry, isMark, type) => {
    const spec = { ...entry.spec };
    if (entry.parse) spec.parseDOM = entry.parse;
    if (!isMark && !entry.render && (entry.component || entry.editComponent)) {
      const dom = componentDom(type, entry.spec.inline);
      if (!entry.parse) spec.parseDOM = dom.parse;
      spec.toDOM = dom.toDOM;
      return spec;
    }
    spec.toDOM = (value) => {
      const out = entry.render ? entry.render(value, EDIT_CONTEXT) : null;
      if (out) return out;
      return isMark || entry.spec.content ? [entry.spec.inline || isMark ? "span" : "div", 0] : ["span"];
    };
    return spec;
  };
  return {
    ...def,
    panel: Boolean((def.fields && def.fields.length) || (def.actions && def.actions.length)),
    renderers: { nodes, marks },
    nodes: Object.fromEntries(Object.entries(nodes).map(([type, entry]) => [type, toSpec(entry, false, type)])),
    marks: Object.fromEntries(Object.entries(marks).map(([type, entry]) => [type, toSpec(entry, true, type)])),
    insert: def.insert || [],
    options: def.options || {},
  };
}

/**
 * The extensions a document is rendered with, checked: one name each, and no
 * type defined twice or over one of the engine's own.
 */
export function composeExtensions(extensions) {
  const registry = { list: [], nodes: {}, marks: {}, byName: {} };
  for (const ext of extensions || []) {
    if (!ext || !ext.renderers) throw new Error("an editor extension must come from defineExtension");
    if (registry.byName[ext.name]) throw new Error(`editor extension "${ext.name}" is listed twice`);
    // One character opens one picker: two kinds on "@" would leave the second unreachable.
    const sharing = ext.trigger && registry.list.find((other) => other.trigger === ext.trigger);
    if (sharing) throw new Error(`editor extensions "${sharing.name}" and "${ext.name}" share the trigger "${ext.trigger}"`);
    registry.byName[ext.name] = ext;
    registry.list.push(ext);
    for (const [kind, table] of [["nodes", ext.renderers.nodes], ["marks", ext.renderers.marks]]) {
      for (const [type, entry] of Object.entries(table)) {
        if (BASE_NAMES.includes(type) || registry.nodes[type] || registry.marks[type]) {
          throw new Error(`editor extension "${ext.name}" redefines "${type}"`);
        }
        registry[kind][type] = { ...entry, ext };
      }
    }
  }
  return registry;
}

/** A dotted attr path (`snapshot.title`) read from attrs. */
export function readAttrPath(attrs, path) {
  return path.split(".").reduce((value, key) => (value && typeof value === "object" ? value[key] : undefined), attrs);
}

/** The attrs with one dotted path set, copied — never mutated in place. */
export function writeAttrPath(attrs, path, value) {
  const [head, ...rest] = path.split(".");
  if (rest.length === 0) return { ...attrs, [head]: value };
  const inner = attrs && typeof attrs[head] === "object" && attrs[head] ? attrs[head] : {};
  return { ...attrs, [head]: writeAttrPath(inner, rest.join("."), value) };
}

/**
 * A picker that asks a project route: `GET route?q=…` answers a JSON array
 * of `{ id, label, href, snapshot }`. Anything else is an error the picker
 * shows, not an empty list.
 */
export function routeSearch(route) {
  return async (query) => {
    const url = `${route}${route.includes("?") ? "&" : "?"}q=${encodeURIComponent(query)}`;
    const response = await fetch(url, { credentials: "same-origin", headers: { Accept: "application/json" } });
    if (!response.ok) throw new Error(`${route} answered ${response.status}`);
    const items = await response.json();
    if (!Array.isArray(items)) throw new Error(`${route} must answer a JSON array of { id, label, href, snapshot }`);
    return items;
  };
}

export default defineExtension;
