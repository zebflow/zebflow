import { h } from "zeb/react";
import { sanitizeAttrs, sanitizeTag } from "zeb/ui/editor-extension";

/**
 * Stored document HTML on a page — the narrow door, not a raw-HTML escape.
 *
 *   <DocumentHtml html={row.body_html} />
 *
 * A project that stores `renderDocumentHtml(doc, { extensions })` beside the
 * document's JSON shows it with this. The string is never trusted and never
 * set as HTML: it is parsed here into a tree, and every element is rebuilt
 * through the same allowlist `renderDocumentHtml` and `<DocumentView>` use
 * (`sanitizeTag` / `sanitizeAttrs` in `zeb/ui/editor-extension`). A tampered
 * row still cannot put a script, a style, an `on…` handler, an iframe or a
 * `javascript:` link on the page.
 *
 * The result is ordinary Zeb React elements, so the server and the browser
 * build the same tree from the same string and hydration matches. HTML that
 * `renderDocumentHtml` wrote renders byte for byte as `<DocumentView>` would
 * render its document. A block whose library mounts from markup (the Potoru
 * player's placeholder) is in the server HTML, so the page loads that
 * library — and only when such a block is there. (Never write that marker
 * in this file, even in a comment: RWE reads the client code for it too.)
 * A block whose component needs its own handlers in the page needs
 * `<DocumentView doc>`: stored HTML carries markup, not code.
 *
 * Plain JavaScript (no JSX), like every editor extension file.
 */

// Elements with no end tag and no content.
const VOID_TAGS = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"]);
// Elements whose content is text up to their own end tag, never markup.
const RAW_TEXT_TAGS = new Set(["script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes", "noscript"]);
// Attributes that are on when present, whatever their value.
const BOOLEAN_ATTRS = new Set(["checked", "disabled"]);
// Deeper than this, elements are kept as their parent's content.
const MAX_DEPTH = 200;

const NAMED_ENTITIES = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };

function decodeEntities(text) {
  if (text.indexOf("&") < 0) return text;
  return text.replace(/&(#[xX][0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);/g, (whole, body) => {
    if (body[0] === "#") {
      const code = body[1] === "x" || body[1] === "X" ? parseInt(body.slice(2), 16) : parseInt(body.slice(1), 10);
      return Number.isFinite(code) && code > 0 && code <= 0x10ffff ? String.fromCodePoint(code) : "\ufffd";
    }
    const named = NAMED_ENTITIES[body.toLowerCase()];
    return named === undefined ? whole : named;
  });
}

const NAME = /[A-Za-z][A-Za-z0-9:-]*/y;
const SPACE = /[\s/]*/y;
const ATTR_NAME = /[^\s"'>/=]+/y;
const ATTR_VALUE = /\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]*))/y;

function readAt(re, text, at) {
  re.lastIndex = at;
  return re.exec(text);
}

/** A start tag at `at` (just after `<`): `{ tag, attrs, end, selfClosing }`, or null. */
function readStartTag(text, at) {
  const name = readAt(NAME, text, at);
  if (!name) return null;
  let i = at + name[0].length;
  const attrs = {};
  for (;;) {
    i += readAt(SPACE, text, i)[0].length;
    if (i >= text.length) return { tag: name[0].toLowerCase(), attrs, end: text.length, selfClosing: false };
    if (text[i] === ">") {
      return { tag: name[0].toLowerCase(), attrs, end: i + 1, selfClosing: text[i - 1] === "/" };
    }
    const attr = readAt(ATTR_NAME, text, i);
    if (!attr) { i += 1; continue; }
    i += attr[0].length;
    const value = readAt(ATTR_VALUE, text, i);
    let raw = "";
    if (value) {
      i += value[0].length;
      raw = value[1] ?? value[2] ?? value[3] ?? "";
    }
    const key = attr[0].toLowerCase();
    if (!(key in attrs)) attrs[key] = decodeEntities(raw);
  }
}

/**
 * HTML → a tree of `{ tag, attrs, children }` and strings. Tolerant: an end
 * tag closes the nearest open element of its name and is ignored when none
 * is open; comments, doctypes and processing instructions are dropped.
 */
export function parseDocumentHtml(html) {
  const text = typeof html === "string" ? html : "";
  const root = { tag: "", attrs: {}, children: [] };
  const stack = [root];
  const top = () => stack[stack.length - 1];
  const pushText = (value) => { if (value) top().children.push(decodeEntities(value)); };
  let i = 0;
  while (i < text.length) {
    const lt = text.indexOf("<", i);
    if (lt < 0) { pushText(text.slice(i)); break; }
    pushText(text.slice(i, lt));
    const next = text[lt + 1];
    if (text.startsWith("<!--", lt)) {
      const close = text.indexOf("-->", lt + 4);
      i = close < 0 ? text.length : close + 3;
    } else if (next === "!" || next === "?") {
      const close = text.indexOf(">", lt);
      i = close < 0 ? text.length : close + 1;
    } else if (next === "/") {
      const name = readAt(NAME, text, lt + 2);
      const close = text.indexOf(">", lt);
      i = close < 0 ? text.length : close + 1;
      if (!name) continue;
      const tag = name[0].toLowerCase();
      for (let s = stack.length - 1; s > 0; s -= 1) {
        if (stack[s].tag === tag) { stack.length = s; break; }
      }
    } else {
      const start = readStartTag(text, lt + 1);
      if (!start) { pushText("<"); i = lt + 1; continue; }
      i = start.end;
      const element = { tag: start.tag, attrs: start.attrs, children: [] };
      if (stack.length > MAX_DEPTH) {
        if (!VOID_TAGS.has(start.tag) && !start.selfClosing) continue;
      } else {
        top().children.push(element);
      }
      if (RAW_TEXT_TAGS.has(start.tag)) {
        const close = text.toLowerCase().indexOf(`</${start.tag}`, i);
        const end = close < 0 ? text.length : close;
        if (end > i) element.children.push(text.slice(i, end));
        const gt = close < 0 ? -1 : text.indexOf(">", close);
        i = gt < 0 ? text.length : gt + 1;
      } else if (!VOID_TAGS.has(start.tag) && !start.selfClosing && stack.length <= MAX_DEPTH) {
        stack.push(element);
      }
    }
  }
  return root.children;
}

/** One parsed node as an element, through the document allowlist. */
function toElement(node, key) {
  if (typeof node === "string") return node;
  const name = sanitizeTag(node.tag);
  if (!name) return null;
  const attrs = {};
  for (const [attr, value] of Object.entries(node.attrs)) {
    attrs[attr] = value === "" && BOOLEAN_ATTRS.has(attr) ? true : value;
  }
  const clean = sanitizeAttrs(name, attrs, false);
  // `class` printed last, as the document renderer prints it.
  if (clean.class !== undefined) { const cls = clean.class; delete clean.class; clean.className = cls; }
  if (VOID_TAGS.has(name)) return h(name, { key, ...clean });
  return h(name, { key, ...clean }, ...toChildren(node.children));
}

function toChildren(nodes) {
  const out = [];
  nodes.forEach((node, i) => {
    const element = toElement(node, i);
    if (element !== null && element !== "") out.push(element);
  });
  return out;
}

/** Stored document HTML, sanitized, as elements: the same on the server and in the browser. */
export function DocumentHtml({ html, className }) {
  return h("div", { "data-slot": "document", className }, ...toChildren(parseDocumentHtml(html)));
}

export default DocumentHtml;
