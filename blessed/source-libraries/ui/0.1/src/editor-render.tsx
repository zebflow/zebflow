import { h } from "zeb/react";
import { tokenize } from "zeb/ui/code-block";

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
 */

export const EDITOR_CLASSES = {
  root: "outline-none min-h-[8rem] pl-7 text-base leading-7 text-foreground",
  paragraph: "my-1.5",
  heading1: "mt-8 mb-2 text-3xl font-bold tracking-tight",
  heading2: "mt-6 mb-2 text-2xl font-semibold tracking-tight",
  heading3: "mt-4 mb-1 text-xl font-semibold",
  blockquote: "my-3 border-l-2 border-border pl-4 text-muted-foreground",
  callout: "my-3 flex gap-3 rounded-lg border border-border bg-muted/60 px-4 py-3",
  calloutIcon: "select-none text-lg leading-7",
  calloutBody: "min-w-0 flex-1",
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

/** Only links a browser can be trusted to follow. Anything else becomes `#`. */
export function safeHref(href) {
  const value = String(href || "").trim();
  if (/^(https?:|mailto:|tel:|\/|#|\.)/i.test(value)) return value;
  return "#";
}

function escapeHtml(text) {
  return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/**
 * Walk a document with a hyperscript function `hx(tag, attrs, children[])`.
 * The same walk builds elements (DocumentView) and strings (renderDocumentHtml).
 */
function walk(node, hx, c, key) {
  const kids = (parent) => (parent.content || []).map((child, i) => walk(child, hx, c, i));
  switch (node.type) {
    case "doc": return kids(node);
    case "paragraph": return hx("p", { key, className: c.paragraph }, kids(node));
    case "heading": {
      const level = Math.min(3, Math.max(1, node.attrs?.level || 1));
      return hx(`h${level}`, { key, className: c[`heading${level}`] }, kids(node));
    }
    case "blockquote": return hx("blockquote", { key, className: c.blockquote }, kids(node));
    case "callout":
      return hx("aside", { key, className: c.callout, "data-callout": "" }, [
        hx("span", { key: "i", className: c.calloutIcon }, [node.attrs?.icon || "💡"]),
        hx("div", { key: "b", className: c.calloutBody }, kids(node)),
      ]);
    case "code_block": {
      const code = (node.content || []).map((t) => t.text || "").join("");
      const lang = node.attrs?.language || "";
      const spans = tokenize(code, lang || "tsx").map((t, i) => hx("span", { key: i, className: TOKEN_CLASSES[t.type] || "" }, [t.text]));
      return hx("pre", { key, className: c.codeBlock, "data-language": lang }, [hx("code", { key: "c" }, spans)]);
    }
    case "horizontal_rule": return hx("hr", { key, className: c.hr }, []);
    case "image":
      return hx("img", { key, className: c.image, src: safeHref(node.attrs?.src), alt: node.attrs?.alt || "", width: node.attrs?.width || undefined, "data-ref": node.attrs?.ref || undefined }, []);
    case "bullet_list": return hx("ul", { key, className: c.bulletList }, kids(node));
    case "ordered_list": return hx("ol", { key, className: c.orderedList, start: node.attrs?.order && node.attrs.order !== 1 ? node.attrs.order : undefined }, kids(node));
    case "todo_list": return hx("ul", { key, className: c.todoList, "data-todo": "" }, kids(node));
    case "list_item": return hx("li", { key, className: c.listItem }, kids(node));
    case "todo_item":
      return hx("li", { key, className: c.todoItem, "data-checked": String(!!node.attrs?.checked) }, [
        hx("input", { key: "x", type: "checkbox", className: c.todoBox, checked: !!node.attrs?.checked, disabled: true }, []),
        hx("div", { key: "b", className: c.todoBody }, kids(node)),
      ]);
    case "hard_break": return hx("br", { key }, []);
    case "text": {
      let out = node.text || "";
      for (const mark of node.marks || []) {
        switch (mark.type) {
          case "bold": out = hx("strong", { key }, [out]); break;
          case "italic": out = hx("em", { key }, [out]); break;
          case "underline": out = hx("u", { key }, [out]); break;
          case "strike": out = hx("s", { key }, [out]); break;
          case "code": out = hx("code", { key, className: c.code }, [out]); break;
          case "link": out = hx("a", { key, className: c.link, href: safeHref(mark.attrs?.href), title: mark.attrs?.title || undefined, rel: "noopener" }, [out]); break;
          default: break;
        }
      }
      return out;
    }
    default: return kids(node);
  }
}

/** A document as elements. Server-safe: the engine is not involved. */
export function DocumentView({ doc, className }) {
  const children = doc && doc.type === "doc" ? walk(doc, h, EDITOR_CLASSES) : [];
  return <div data-slot="document" className={className}>{children}</div>;
}

/** The same document as an HTML string — what a `body_html` column stores. */
export function renderDocumentHtml(doc) {
  const VOID = new Set(["img", "br", "hr", "input"]);
  const hx = (tag, attrs, children) => {
    const parts = [];
    for (const [name, value] of Object.entries(attrs || {})) {
      if (name === "key" || value === undefined || value === null || value === false) continue;
      const attr = name === "className" ? "class" : name;
      parts.push(value === true ? ` ${attr}` : ` ${attr}="${escapeHtml(value)}"`);
    }
    const open = `<${tag}${parts.join("")}>`;
    if (VOID.has(tag)) return { __html: open };
    // A string child is text and gets escaped; an object child is markup
    // this function already produced (a mark wrapping a mark) and is kept.
    const inner = (children || []).map((child) => (typeof child === "string" ? escapeHtml(child) : (child && child.__html) || "")).join("");
    return { __html: `${open}${inner}</${tag}>` };
  };
  if (!doc || doc.type !== "doc") return "";
  return walk(doc, hx, EDITOR_CLASSES).map((part) => (typeof part === "string" ? escapeHtml(part) : part.__html)).join("");
}

/** The words in a document, for a teaser or a count. */
export function documentText(doc) {
  const out = [];
  const visit = (node) => {
    if (node.type === "text") out.push(node.text || "");
    (node.content || []).forEach(visit);
    if (node.type !== "text" && node.type !== "doc") out.push(" ");
  };
  if (doc) visit(doc);
  return out.join("").replace(/\s+/g, " ").trim();
}

export default DocumentView;
