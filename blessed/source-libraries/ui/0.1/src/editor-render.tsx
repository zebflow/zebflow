import { h, renderToString as zebRenderToString } from "zeb/react";
import { tokenize } from "zeb/ui/code-block";
import { composeExtensions, safeHref, sanitizeAttrs, sanitizeTag } from "zeb/ui/editor-extension";
import { componentElement } from "zeb/ui/editor-component";

/**
 * The document's look, and how it renders without the editor.
 *
 * `EDITOR_CLASSES` is the one place a block's classes are written; the
 * editor hands it to the engine (`createEditor({ classes })`) so the live
 * document and the published one are styled by the same strings — strings
 * that live here, in template source, where the Tailwind scan finds them.
 *
 * `<DocumentView doc>` renders a document as elements (server or browser,
 * no raw HTML); `renderDocumentHtml(doc)` renders the same tree to a string
 * for a `body_html` column. Both walk the same table, so they cannot drift.
 *
 * Both take the extensions the document was written with —
 * `<DocumentView doc extensions={[…]} />`, `renderDocumentHtml(doc, { extensions })`
 * — and render each extension node through its own `render`, or its
 * `component` (an element in DocumentView, zeb/react's `renderToString` in
 * the string: the same bytes either way). A node no
 * extension knows is not dropped: it renders as a `data-unknown-node` element
 * holding its content. Every element passes the same allowlist
 * (`zeb/ui/editor-extension`), so scripts, handlers and `javascript:` links
 * never reach the page.
 */

export const EDITOR_CLASSES = {
  root: "outline-none min-h-[8rem] pl-7 text-base leading-7 text-foreground",
  paragraph: "my-1.5",
  heading1: "mt-8 mb-2 text-3xl font-bold tracking-tight",
  heading2: "mt-6 mb-2 text-2xl font-semibold tracking-tight",
  heading3: "mt-4 mb-1 text-xl font-semibold",
  blockquote: "my-3 border-l-2 border-border pl-4 text-muted-foreground",
  codeBlock: "my-3 overflow-x-auto rounded-lg border border-border bg-muted px-4 py-3 font-mono text-[0.85rem] leading-6",
  hr: "my-6 border-border",
  image: "my-4 max-w-full rounded-lg",
  bulletList: "my-1.5 list-disc pl-6",
  orderedList: "my-1.5 list-decimal pl-6",
  todoList: "my-1.5 list-none pl-0",
  listItem: "my-0.5",
  todoItem: "my-0.5 flex items-start gap-2",
  todoBox: "mt-2 size-4 shrink-0 accent-primary",
  todoBody: "min-w-0 flex-1",
  link: "text-info underline underline-offset-4",
  code: "rounded bg-muted px-1.5 py-0.5 font-mono text-[0.9em]",
  unknown: "my-1.5 rounded-md border border-dashed border-border px-2 text-muted-foreground",
  handle: "w-5 cursor-grab select-none text-center font-mono text-xs leading-4 text-muted-foreground/40 hover:text-muted-foreground",
  placeholder: "is-empty",
  dropCursor: "bg-primary",
};

const TOKEN_CLASSES = {
  comment: "italic text-muted-foreground",
  string: "text-success",
  number: "text-warning",
  keyword: "text-primary",
  tag: "text-info",
  attr: "text-chart-4",
  type: "text-chart-2",
  fn: "text-chart-2",
  variable: "text-warning",
  punct: "text-muted-foreground",
  plain: "",
};

function escapeHtml(text) {
  return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function flatten(children, out = []) {
  for (const child of children) {
    if (Array.isArray(child)) flatten(child, out);
    else if (child !== null && child !== undefined && child !== false && child !== "") out.push(child);
  }
  return out;
}

/**
 * The one door every element goes through: `base(tag, attrs, children[])`
 * is called only with an allowed tag and allowed attributes. `key` is kept
 * for the element renderer and never printed.
 */
function guarded(base) {
  return (tag, attrs, ...children) => {
    const name = sanitizeTag(tag);
    if (!name) return null;
    const clean = sanitizeAttrs(name, attrs, false);
    if (clean.class !== undefined) { clean.className = clean.class; delete clean.class; }
    if (attrs && attrs.key !== undefined) clean.key = attrs.key;
    return base(name, clean, flatten(children));
  };
}

/**
 * The context an extension's `render` and `footer` get: `r`. `place` puts a
 * node drawn by a component — an element, or its HTML.
 */
function renderContext(hx, place) {
  const lists = new Map();
  return {
    h: hx,
    place,
    edit: false,
    inline: false,
    safeHref,
    /** Number `id` within `key` by first appearance (1, 2, …); the same id keeps its number. */
    collect(key, id, item) {
      if (!lists.has(key)) lists.set(key, new Map());
      const list = lists.get(key);
      const k = String(id);
      if (!list.has(k)) list.set(k, { n: list.size + 1, id: k, item });
      return list.get(k).n;
    },
    collected: (key) => Array.from((lists.get(key) || new Map()).values()),
  };
}

function renderMark(mark, out, hx, c, key, reg, r) {
  switch (mark.type) {
    case "bold": return hx("strong", { key }, out);
    case "italic": return hx("em", { key }, out);
    case "underline": return hx("u", { key }, out);
    case "strike": return hx("s", { key }, out);
    case "code": return hx("code", { key, className: c.code }, out);
    case "link": return hx("a", { key, className: c.link, href: mark.attrs?.href, title: mark.attrs?.title || undefined, rel: "noopener" }, out);
    default: {
      // An unknown mark keeps its text and loses only the styling.
      const entry = reg.marks[mark.type];
      return entry && entry.render ? entry.render({ type: mark.type, attrs: mark.attrs || {} }, { ...r, content: out }) : out;
    }
  }
}

/**
 * Walk a document with a hyperscript function `hx(tag, attrs, ...children)`.
 * The same walk builds elements (DocumentView) and strings (renderDocumentHtml).
 */
function walk(node, hx, c, key, reg, r) {
  if (!node || typeof node !== "object") return null;
  const kids = (parent) => {
    const inline = parent.type === "paragraph" || parent.type === "heading" || (parent.content || []).some((child) => child.type === "text");
    const inner = inline === r.inline ? r : { ...r, inline };
    return (parent.content || []).map((child, i) => walk(child, hx, c, i, reg, inner));
  };
  switch (node.type) {
    case "doc": return kids(node);
    case "paragraph": return hx("p", { key, className: c.paragraph }, kids(node));
    case "heading": {
      const level = Math.min(3, Math.max(1, node.attrs?.level || 1));
      return hx(`h${level}`, { key, className: c[`heading${level}`] }, kids(node));
    }
    case "blockquote": return hx("blockquote", { key, className: c.blockquote }, kids(node));
    case "code_block": {
      const code = (node.content || []).map((t) => t.text || "").join("");
      const lang = node.attrs?.language || "";
      const spans = tokenize(code, lang || "tsx").map((t, i) => hx("span", { key: i, className: TOKEN_CLASSES[t.type] || "" }, t.text));
      return hx("pre", { key, className: c.codeBlock, "data-language": lang }, hx("code", { key: "c" }, spans));
    }
    case "horizontal_rule": return hx("hr", { key, className: c.hr });
    case "image":
      return hx("img", { key, className: c.image, src: node.attrs?.src, alt: node.attrs?.alt || "", width: node.attrs?.width || undefined, "data-ref": node.attrs?.ref || undefined });
    case "bullet_list": return hx("ul", { key, className: c.bulletList }, kids(node));
    case "ordered_list": return hx("ol", { key, className: c.orderedList, start: node.attrs?.order && node.attrs.order !== 1 ? node.attrs.order : undefined }, kids(node));
    case "todo_list": return hx("ul", { key, className: c.todoList, "data-todo": "" }, kids(node));
    case "list_item": return hx("li", { key, className: c.listItem }, kids(node));
    case "todo_item":
      return hx("li", { key, className: c.todoItem, "data-checked": String(!!node.attrs?.checked) },
        hx("input", { key: "x", type: "checkbox", className: c.todoBox, checked: !!node.attrs?.checked }),
        hx("div", { key: "b", className: c.todoBody }, kids(node)));
    case "hard_break": return hx("br", { key });
    case "text": return (node.marks || []).reduce((out, mark) => renderMark(mark, out, hx, c, key, reg, r), node.text || "");
    default: {
      const entry = reg.nodes[node.type];
      if (entry && entry.component) return r.place(entry, { type: node.type, attrs: node.attrs || {} }, key);
      if (entry && entry.render) {
        return entry.render({ type: node.type, attrs: node.attrs || {} }, { ...r, content: kids(node) });
      }
      // Not ours and not an extension's: shown as what it is, content kept.
      return hx(r.inline ? "span" : "div", { key, className: c.unknown, "data-unknown-node": String(node.type) }, kids(node));
    }
  }
}

/** The document's children, then each extension's footer (references…). */
function renderTree(doc, base, place, extensions) {
  if (!doc || doc.type !== "doc") return [];
  const reg = composeExtensions(extensions);
  const hx = guarded(base);
  const r = renderContext(hx, place);
  const body = walk(doc, hx, EDITOR_CLASSES, 0, reg, r);
  const footers = reg.list.filter((ext) => ext.footer).map((ext) => ext.footer(r));
  return flatten([body, footers]);
}

/** A document as elements. Server-safe: the engine is not involved. */
export function DocumentView({ doc, extensions, className }) {
  const children = renderTree(doc, (tag, attrs, kids) => h(tag, attrs, ...kids), componentElement, extensions);
  return <div data-slot="document" className={className}>{children}</div>;
}

/** The same document as an HTML string — what a `body_html` column stores. */
export function renderDocumentHtml(doc, options = {}) {
  const VOID = new Set(["img", "br", "hr", "input"]);
  const base = (tag, attrs, children) => {
    const parts = [];
    for (const [name, value] of Object.entries(attrs)) {
      if (name === "key") continue;
      const attr = name === "className" ? "class" : name;
      // Byte for byte what zeb/react's renderToString prints for the same element.
      parts.push(value === true || value === "" ? ` ${attr}=""` : ` ${attr}="${escapeHtml(value)}"`);
    }
    const open = `<${tag}${parts.join("")}>`;
    if (VOID.has(tag)) return { __html: open };
    // A string child is text and gets escaped; an object child is markup
    // this function already produced (a mark wrapping a mark) and is kept.
    const inner = children.map((child) => (typeof child === "object" ? child.__html : escapeHtml(child))).join("");
    return { __html: `${open}${inner}</${tag}>` };
  };
  const place = (entry, node) => ({ __html: zebRenderToString(componentElement(entry, node)) });
  return renderTree(doc, base, place, options.extensions)
    .map((part) => (typeof part === "object" ? part.__html : escapeHtml(part)))
    .join("");
}

/** The words in a document, for a teaser or a count. An atom speaks through its extension's `text`. */
export function documentText(doc, options = {}) {
  const reg = composeExtensions(options.extensions);
  const out = [];
  const visit = (node) => {
    const entry = reg.nodes[node.type];
    if (node.type === "text") out.push(node.text || "");
    else if (entry && entry.text) out.push(entry.text({ type: node.type, attrs: node.attrs || {} }));
    else (node.content || []).forEach(visit);
    if (node.type !== "text" && node.type !== "doc") out.push(" ");
  };
  if (doc) visit(doc);
  return out.join("").replace(/\s+/g, " ").trim();
}

export default DocumentView;
