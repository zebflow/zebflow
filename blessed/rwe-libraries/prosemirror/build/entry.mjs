/**
 * zeb/prosemirror — the editing engine under `zeb/ui/editor`.
 *
 * Engine only: ProseMirror's packages, Zebflow's document schema, and
 * `createEditor`, which wires them into an editor that reports to callbacks
 * and renders no UI of its own. Toolbars, menus, colours and upload all
 * belong to whoever calls it — `zeb/ui/editor` for the opinionated one, a
 * project's own component for a custom one. Both speak the same document.
 *
 * Built by ./build.sh into ../0.1/runtime/prosemirror.bundle.mjs.
 */

import { EditorState, Plugin, PluginKey, TextSelection, NodeSelection, Selection } from "prosemirror-state";
import { EditorView, Decoration, DecorationSet } from "prosemirror-view";
import { Schema, DOMParser, DOMSerializer, Fragment, Slice, Node as PMNode } from "prosemirror-model";
import { history, undo, redo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import {
  baseKeymap, toggleMark, setBlockType, wrapIn, lift, chainCommands, exitCode, selectParentNode,
  joinUp, joinDown, newlineInCode, createParagraphNear, liftEmptyBlock, splitBlock, deleteSelection,
  joinBackward, selectNodeBackward,
} from "prosemirror-commands";
import { inputRules, wrappingInputRule, textblockTypeInputRule, InputRule, undoInputRule, smartQuotes, emDash, ellipsis } from "prosemirror-inputrules";
import { wrapInList, splitListItem, liftListItem, sinkListItem } from "prosemirror-schema-list";
import { dropCursor } from "prosemirror-dropcursor";
import { gapCursor } from "prosemirror-gapcursor";

// Everything a custom editor might reach for, under one name.
export const pm = {
  EditorState, Plugin, PluginKey, TextSelection, NodeSelection, Selection,
  EditorView, Decoration, DecorationSet,
  Schema, DOMParser, DOMSerializer, Fragment, Slice, Node: PMNode,
  history, undo, redo, keymap,
  baseKeymap, toggleMark, setBlockType, wrapIn, lift, chainCommands, exitCode, selectParentNode,
  inputRules, wrappingInputRule, textblockTypeInputRule, InputRule, undoInputRule,
  wrapInList, splitListItem, liftListItem, sinkListItem, dropCursor, gapCursor,
};

// ── Schema ───────────────────────────────────────────────────────────────────
//
// The Notion-basic block set, plus whatever the caller's extensions add.
// `classes` are supplied by the caller so the class strings live in template
// source, where the Tailwind scan sees them.
//
// An extension is a plain object (zeb/ui/editor-extension builds one with
// `defineExtension`); the engine reads only these keys:
//
//   name                      unique id
//   nodes / marks             { typeName: ProseMirror spec } — toDOM included
//   trigger                   one character that opens the caller's picker ("@")
//   panel                     true when the caller shows a panel for its nodes
//   commands(schema, pm)      { name: command } merged into instance.commands
//   inputRules(schema, pm)    [InputRule]
//   keymap(schema, pm)        { key: command }, consulted before the base keys
//   plugins(schema, pm)       [Plugin]

const NO_CLASSES = new Proxy({}, { get: () => "" });

// What a document from a richer editor keeps when this one lacks its
// extension: the node survives load and save unchanged, its text stays
// editable where it has text, and it is shown as what it is.
const UNKNOWN = {
  unknown_block: {
    attrs: { original: { default: null } }, content: "block*", group: "block", defining: true,
    toDOM: (node) => ["div", { "data-unknown-node": node.attrs.original?.type || "" }, 0],
  },
  unknown_text: {
    attrs: { original: { default: null } }, content: "inline*", group: "block", defining: true,
    toDOM: (node) => ["div", { "data-unknown-node": node.attrs.original?.type || "" }, 0],
  },
  unknown_inline: {
    attrs: { original: { default: null } }, inline: true, group: "inline", atom: true,
    toDOM: (node) => ["span", { "data-unknown-node": node.attrs.original?.type || "", contenteditable: "false" }, plainText(node.attrs.original) || "?"],
  },
};

function plainText(json) {
  if (!json) return "";
  if (json.type === "text") return json.text || "";
  return (json.content || []).map(plainText).join("");
}

export function createSchema(classes = NO_CLASSES, extensions = []) {
  const c = classes;
  const nodes = {
    doc: { content: "block+" },
    paragraph: {
      content: "inline*", group: "block",
      parseDOM: [{ tag: "p" }],
      toDOM: () => ["p", { class: c.paragraph }, 0],
    },
    heading: {
      attrs: { level: { default: 1 } }, content: "inline*", group: "block", defining: true,
      parseDOM: [1, 2, 3].map((level) => ({ tag: `h${level}`, attrs: { level } })),
      toDOM: (node) => [`h${node.attrs.level}`, { class: c[`heading${node.attrs.level}`] }, 0],
    },
    blockquote: {
      content: "block+", group: "block", defining: true,
      parseDOM: [{ tag: "blockquote" }],
      toDOM: () => ["blockquote", { class: c.blockquote }, 0],
    },
    code_block: {
      attrs: { language: { default: "" } }, content: "text*", marks: "", group: "block", code: true, defining: true,
      parseDOM: [{ tag: "pre", preserveWhitespace: "full", getAttrs: (dom) => ({ language: dom.getAttribute("data-language") || "" }) }],
      toDOM: (node) => ["pre", { class: c.codeBlock, "data-language": node.attrs.language }, ["code", 0]],
    },
    horizontal_rule: {
      group: "block",
      parseDOM: [{ tag: "hr" }],
      toDOM: () => ["hr", { class: c.hr }],
    },
    image: {
      attrs: { src: {}, alt: { default: "" }, ref: { default: null }, width: { default: null } },
      group: "block", draggable: true,
      parseDOM: [{ tag: "img[src]", getAttrs: (dom) => ({ src: dom.getAttribute("src"), alt: dom.getAttribute("alt") || "", ref: dom.getAttribute("data-ref"), width: dom.getAttribute("width") }) }],
      toDOM: (node) => ["img", { class: c.image, src: node.attrs.src, alt: node.attrs.alt, "data-ref": node.attrs.ref || undefined, width: node.attrs.width || undefined }],
    },
    bullet_list: {
      content: "list_item+", group: "block",
      parseDOM: [{ tag: "ul" }],
      toDOM: () => ["ul", { class: c.bulletList }, 0],
    },
    ordered_list: {
      attrs: { order: { default: 1 } }, content: "list_item+", group: "block",
      parseDOM: [{ tag: "ol", getAttrs: (dom) => ({ order: dom.hasAttribute("start") ? +dom.getAttribute("start") : 1 }) }],
      toDOM: (node) => ["ol", { class: c.orderedList, start: node.attrs.order === 1 ? undefined : node.attrs.order }, 0],
    },
    todo_list: {
      content: "todo_item+", group: "block",
      parseDOM: [{ tag: "ul[data-todo]" }],
      toDOM: () => ["ul", { class: c.todoList, "data-todo": "" }, 0],
    },
    list_item: {
      content: "paragraph block*", defining: true,
      parseDOM: [{ tag: "li" }],
      toDOM: () => ["li", { class: c.listItem }, 0],
    },
    todo_item: {
      attrs: { checked: { default: false } }, content: "paragraph block*", defining: true,
      parseDOM: [{ tag: "li[data-checked]", getAttrs: (dom) => ({ checked: dom.getAttribute("data-checked") === "true" }) }],
      toDOM: (node) => ["li", { class: c.todoItem, "data-checked": String(node.attrs.checked) }, ["input", { type: "checkbox", class: c.todoBox, checked: node.attrs.checked ? "" : undefined, contenteditable: "false" }], ["div", { class: c.todoBody }, 0]],
    },
    text: { group: "inline" },
    hard_break: {
      inline: true, group: "inline", selectable: false,
      parseDOM: [{ tag: "br" }],
      toDOM: () => ["br"],
    },
  };
  const marks = {
    link: {
      attrs: { href: {}, title: { default: null } }, inclusive: false,
      parseDOM: [{ tag: "a[href]", getAttrs: (dom) => ({ href: dom.getAttribute("href"), title: dom.getAttribute("title") }) }],
      toDOM: (mark) => ["a", { class: c.link, href: mark.attrs.href, title: mark.attrs.title, rel: "noopener" }, 0],
    },
    bold: { parseDOM: [{ tag: "strong" }, { tag: "b" }, { style: "font-weight=bold" }], toDOM: () => ["strong", 0] },
    italic: { parseDOM: [{ tag: "em" }, { tag: "i" }, { style: "font-style=italic" }], toDOM: () => ["em", 0] },
    underline: { parseDOM: [{ tag: "u" }, { style: "text-decoration=underline" }], toDOM: () => ["u", 0] },
    strike: { parseDOM: [{ tag: "s" }, { tag: "del" }, { style: "text-decoration=line-through" }], toDOM: () => ["s", 0] },
    code: { parseDOM: [{ tag: "code" }], toDOM: () => ["code", { class: c.code }, 0] },
  };
  Object.assign(nodes, UNKNOWN);
  for (const ext of extensions) {
    for (const [name, spec] of Object.entries(ext.nodes || {})) {
      if (nodes[name] || marks[name]) throw new Error(`editor extension "${ext.name}" redefines "${name}"`);
      nodes[name] = spec;
    }
    for (const [name, spec] of Object.entries(ext.marks || {})) {
      if (nodes[name] || marks[name]) throw new Error(`editor extension "${ext.name}" redefines "${name}"`);
      marks[name] = spec;
    }
  }
  return new Schema({ nodes, marks });
}

/**
 * A stored document as this schema can hold it: a node or mark the schema
 * does not know becomes an `unknown_*` node carrying the original, so loading
 * and saving a document never loses what another editor wrote.
 */
export function adoptDocument(schema, json) {
  const known = (type) => Object.prototype.hasOwnProperty.call(schema.nodes, type) && !type.startsWith("unknown_");
  const inlineType = (type) => known(type) && schema.nodes[type].isInline;
  const adopt = (node, inline) => {
    if (!node || typeof node !== "object") return null;
    const marksKnown = (node.marks || []).every((m) => Object.prototype.hasOwnProperty.call(schema.marks, m.type));
    if (known(node.type) && marksKnown) {
      if (!node.content) return node;
      const inner = schema.nodes[node.type].inlineContent;
      return { ...node, content: node.content.map((child) => adopt(child, inner)).filter(Boolean) };
    }
    if (inline || inlineType(node.type) || node.type === "text") return { type: "unknown_inline", attrs: { original: node } };
    const { content, ...original } = node;
    const children = content || [];
    if (children.some((child) => child.type === "text" || inlineType(child.type))) {
      return { type: "unknown_text", attrs: { original }, content: children.map((child) => adopt(child, true)).filter(Boolean) };
    }
    return { type: "unknown_block", attrs: { original }, content: children.map((child) => adopt(child, false)).filter(Boolean) };
  };
  return adopt(json, false);
}

/** The inverse of `adoptDocument`: every `unknown_*` node back to what it was. */
export function restoreDocument(json) {
  if (!json || typeof json !== "object") return json;
  if (json.type === "unknown_inline") return json.attrs.original;
  const content = json.content ? json.content.map(restoreDocument) : undefined;
  if (json.type === "unknown_text" || json.type === "unknown_block") {
    const original = { ...json.attrs.original };
    if (content && content.length) original.content = content;
    return original;
  }
  return content ? { ...json, content } : json;
}

/** The document a fresh editor starts with. */
export const EMPTY_DOC = { type: "doc", content: [{ type: "paragraph" }] };

// ── Commands ─────────────────────────────────────────────────────────────────

function buildCommands(schema) {
  const n = schema.nodes;
  const m = schema.marks;
  const listOf = { bullet: n.bullet_list, ordered: n.ordered_list, todo: n.todo_list };
  return {
    toggleBold: toggleMark(m.bold),
    toggleItalic: toggleMark(m.italic),
    toggleUnderline: toggleMark(m.underline),
    toggleStrike: toggleMark(m.strike),
    toggleCode: toggleMark(m.code),
    paragraph: setBlockType(n.paragraph),
    heading: (level) => setBlockType(n.heading, { level }),
    codeBlock: setBlockType(n.code_block),
    blockquote: wrapIn(n.blockquote),
    list: (kind) => wrapInList(listOf[kind]),
    lift,
    undo,
    redo,
    horizontalRule: (state, dispatch) => {
      if (dispatch) dispatch(state.tr.replaceSelectionWith(n.horizontal_rule.create()).scrollIntoView());
      return true;
    },
    image: (attrs) => (state, dispatch) => {
      if (dispatch) dispatch(state.tr.replaceSelectionWith(n.image.create(attrs)).scrollIntoView());
      return true;
    },
    setLink: (href) => (state, dispatch) => {
      const { from, to, empty } = state.selection;
      if (empty) return false;
      if (dispatch) {
        const tr = state.tr.removeMark(from, to, m.link);
        if (href) tr.addMark(from, to, m.link.create({ href }));
        dispatch(tr);
      }
      return true;
    },
  };
}

// ── Plugins ──────────────────────────────────────────────────────────────────

const escapeRegExp = (text) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Tracks a trigger character and the query typed after it, from the caret
 * back to the character (which must start the block or follow a space).
 * `rules` is `[{ name, pattern }]`; the pattern's group 1 is the query.
 */
function triggerPlugin(name, rules, report) {
  const key = new PluginKey(name);
  return new Plugin({
    key,
    state: {
      init: () => null,
      apply(tr, prev, _old, state) {
        const { $from, empty } = state.selection;
        if (!empty || !$from.parent.isTextblock || $from.parent.type.spec.code) return null;
        const text = $from.parent.textBetween(0, $from.parentOffset, undefined, "￼");
        for (const rule of rules) {
          const match = rule.pattern.exec(text);
          if (match) return { name: rule.name, query: match[1], from: $from.pos - match[1].length - 1, to: $from.pos };
        }
        return null;
      },
    },
    view: () => ({
      update(view, prevState) {
        const cur = key.getState(view.state);
        const prev = key.getState(prevState);
        if (cur === prev) return;
        if (!cur) { report(null); return; }
        const coords = view.coordsAtPos(cur.from);
        report({ ...cur, left: coords.left, top: coords.top, bottom: coords.bottom });
      },
    }),
  });
}

/** Tracks a `/query` typed at the start of an empty-ish text block. */
function slashPlugin(onSlash) {
  return triggerPlugin("zebSlash", [{ name: "slash", pattern: /(?:^|\s)\/([\w-]*)$/ }], (state) => onSlash(state && { query: state.query, from: state.from, to: state.to, left: state.left, top: state.top, bottom: state.bottom }));
}

/** `@ada` and the like: an extension's trigger character, then a short query. */
function extensionTriggerPlugin(extensions, onTrigger) {
  const rules = extensions.filter((ext) => ext.trigger).map((ext) => {
    const c = escapeRegExp(ext.trigger);
    return { name: ext.name, pattern: new RegExp(`(?:^|\\s)${c}((?:[^\\s${c}][^${c}\\n]{0,39})?)$`) };
  });
  return rules.length ? triggerPlugin("zebTrigger", rules, onTrigger) : null;
}

/**
 * Reports the innermost node around the selection that belongs to an
 * extension with a panel (`ext.panel`), so the caller can edit its attrs.
 */
function nodePlugin(extensions, onNode) {
  const owner = new Map();
  for (const ext of extensions) {
    if (!ext.panel) continue;
    for (const type of Object.keys(ext.nodes || {})) owner.set(type, ext.name);
  }
  if (owner.size === 0) return null;
  const find = (state) => {
    const sel = state.selection;
    if (sel instanceof NodeSelection && owner.has(sel.node.type.name)) return { node: sel.node, pos: sel.from };
    const $from = sel.$from;
    for (let depth = $from.depth; depth > 0; depth--) {
      const node = $from.node(depth);
      if (owner.has(node.type.name)) return { node, pos: $from.before(depth) };
    }
    return null;
  };
  let last = "";
  return new Plugin({
    view: () => ({
      update(view) {
        const found = find(view.state);
        const sig = found ? `${found.pos}:${found.node.type.name}:${JSON.stringify(found.node.attrs)}` : "";
        if (sig === last) return;
        last = sig;
        if (!found) { onNode(null); return; }
        const dom = view.nodeDOM(found.pos);
        const rect = dom && dom.getBoundingClientRect ? dom.getBoundingClientRect() : view.coordsAtPos(found.pos);
        onNode({ extension: owner.get(found.node.type.name), type: found.node.type.name, pos: found.pos, attrs: found.node.attrs, left: rect.left, top: rect.top, bottom: rect.bottom });
      },
    }),
  });
}

/** Reports the selection to the caller: where it is, what marks it has. */
function selectionPlugin(onSelection) {
  return new Plugin({
    view: () => ({
      update(view, prevState) {
        if (view.state.selection.eq(prevState.selection) && view.state.doc.eq(prevState.doc)) return;
        const { from, to, empty, $from } = view.state.selection;
        if (empty || !view.hasFocus()) { onSelection(null); return; }
        const start = view.coordsAtPos(from);
        const end = view.coordsAtPos(to);
        const marks = {};
        for (const name of Object.keys(view.state.schema.marks)) {
          const type = view.state.schema.marks[name];
          marks[name] = view.state.doc.rangeHasMark(from, to, type);
        }
        const block = $from.parent;
        onSelection({
          from, to,
          left: (start.left + end.left) / 2, top: start.top, bottom: end.bottom,
          marks,
          block: block.type.name,
          attrs: block.attrs,
          text: view.state.doc.textBetween(from, to, " "),
        });
      },
    }),
  });
}

/** Placeholder on the only empty paragraph of an otherwise empty document. */
function placeholderPlugin(text, className) {
  return new Plugin({
    props: {
      decorations(state) {
        const { doc } = state;
        const empty = doc.childCount === 1 && doc.firstChild.type.name === "paragraph" && doc.firstChild.content.size === 0;
        if (!empty || !text) return null;
        return DecorationSet.create(doc, [Decoration.node(0, doc.firstChild.nodeSize, { class: className, "data-placeholder": text })]);
      },
    },
  });
}

/** A grab handle in front of every top-level block; dragging it moves the block. */
/**
 * One drag handle, outside the editable, that follows the block under the
 * mouse. Outside on purpose: a widget inside the contenteditable is text to
 * the browser — Home/Shift+Home run past it, copy includes it, and floats
 * make Chrome's caret wander. Dropping uses ProseMirror's own drop handling
 * through `view.dragging`, so the drop cursor and the move are native.
 */
function handlePlugin(className) {
  return new Plugin({
    view(view) {
      const doc = view.dom.ownerDocument;
      const host = view.dom.parentNode;
      if (!host) return {};
      const handle = doc.createElement("div");
      handle.className = className;
      handle.setAttribute("draggable", "true");
      handle.setAttribute("data-drag-handle", "");
      handle.setAttribute("aria-hidden", "true");
      handle.title = "Drag to move";
      handle.textContent = "⋮⋮";
      handle.style.position = "absolute";
      handle.style.display = "none";
      if (doc.defaultView.getComputedStyle(host).position === "static") host.style.position = "relative";
      host.appendChild(handle);
      let pos = null;
      let frame = 0;

      const hide = () => { pos = null; handle.style.display = "none"; };
      const blockAt = (y) => {
        let found = null;
        view.state.doc.forEach((node, offset) => {
          if (found !== null) return;
          const dom = view.nodeDOM(offset);
          if (!dom || !dom.getBoundingClientRect) return;
          const r = dom.getBoundingClientRect();
          if (y >= r.top && y <= r.bottom) found = offset;
        });
        return found;
      };
      const place = (p) => {
        const dom = view.nodeDOM(p);
        if (!dom || !dom.getBoundingClientRect) return hide();
        pos = p;
        const r = dom.getBoundingClientRect();
        const h = host.getBoundingClientRect();
        const root = view.dom.getBoundingClientRect();
        handle.style.display = "";
        handle.style.top = `${r.top - h.top + host.scrollTop + 4}px`;
        handle.style.left = `${root.left - h.left + 4}px`;
      };
      const onMove = (event) => {
        if (frame) return;
        frame = requestAnimationFrame(() => {
          frame = 0;
          if (!view.editable) return hide();
          const found = blockAt(event.clientY);
          if (found !== null && found !== pos) place(found);
        });
      };
      const onLeave = (event) => {
        if (event.relatedTarget && host.contains(event.relatedTarget)) return;
        hide();
      };
      const onMouseDown = () => {
        if (pos === null || pos >= view.state.doc.content.size) return;
        view.dispatch(view.state.tr.setSelection(NodeSelection.create(view.state.doc, pos)));
      };
      const onDragStart = (event) => {
        if (pos === null || pos >= view.state.doc.content.size) { event.preventDefault(); return; }
        const selection = NodeSelection.create(view.state.doc, pos);
        view.dispatch(view.state.tr.setSelection(selection));
        const dom = view.nodeDOM(pos);
        event.dataTransfer.effectAllowed = "copyMove";
        event.dataTransfer.setData("text/plain", selection.node.textContent);
        if (dom && dom.nodeType === 1) event.dataTransfer.setDragImage(dom, 0, 0);
        view.dragging = { slice: selection.content(), move: true };
      };
      host.addEventListener("mousemove", onMove);
      host.addEventListener("mouseleave", onLeave);
      handle.addEventListener("mousedown", onMouseDown);
      handle.addEventListener("dragstart", onDragStart);
      handle.addEventListener("dragend", hide);
      return {
        update(v, prev) { if (pos !== null && !v.state.doc.eq(prev.doc)) hide(); },
        destroy() {
          if (frame) cancelAnimationFrame(frame);
          host.removeEventListener("mousemove", onMove);
          host.removeEventListener("mouseleave", onLeave);
          handle.remove();
        },
      };
    },
  });
}

/** Clicking a to-do's box toggles it — the box is contenteditable=false. */
function todoPlugin(schema) {
  return new Plugin({
    props: {
      handleDOMEvents: {
        mousedown(view, event) {
          const target = event.target;
          if (!(target instanceof HTMLInputElement) || target.type !== "checkbox") return false;
          const li = target.closest("li[data-checked]");
          if (!li) return false;
          const $pos = view.state.doc.resolve(view.posAtDOM(li, 0));
          const node = $pos.parent;
          if (node.type !== schema.nodes.todo_item) return false;
          view.dispatch(view.state.tr.setNodeMarkup($pos.before(), undefined, { ...node.attrs, checked: !node.attrs.checked }));
          event.preventDefault();
          return true;
        },
      },
    },
  });
}

/** Dropped or pasted image files go through the caller's uploader. */
function imagePlugin(schema, upload) {
  const insert = (view, files, pos) => {
    if (!upload) return false;
    const images = Array.from(files).filter((f) => f.type.startsWith("image/"));
    if (images.length === 0) return false;
    images.forEach((file) => {
      Promise.resolve(upload(file)).then((result) => {
        if (!result) return;
        const attrs = typeof result === "string" ? { src: result } : { src: result.src || result.url, ref: result.ref || null, alt: result.alt || file.name };
        const node = schema.nodes.image.create(attrs);
        const at = pos == null ? view.state.selection.from : pos;
        view.dispatch(view.state.tr.insert(at, node));
      });
    });
    return true;
  };
  return new Plugin({
    props: {
      handleDrop(view, event) {
        const files = event.dataTransfer && event.dataTransfer.files;
        if (!files || files.length === 0) return false;
        const coords = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (insert(view, files, coords ? coords.pos : null)) { event.preventDefault(); return true; }
        return false;
      },
      handlePaste(view, event) {
        const files = event.clipboardData && event.clipboardData.files;
        if (!files || files.length === 0) return false;
        if (insert(view, files, null)) { event.preventDefault(); return true; }
        return false;
      },
    },
  });
}

function buildInputRules(schema) {
  const n = schema.nodes;
  return inputRules({
    rules: [
      ...smartQuotes, ellipsis, emDash,
      textblockTypeInputRule(/^(#{1,3})\s$/, n.heading, (match) => ({ level: match[1].length })),
      wrappingInputRule(/^\s*>\s$/, n.blockquote),
      wrappingInputRule(/^\s*([-+*])\s$/, n.bullet_list),
      wrappingInputRule(/^(\d+)\.\s$/, n.ordered_list, (match) => ({ order: +match[1] }), (match, node) => node.childCount + node.attrs.order === +match[1]),
      wrappingInputRule(/^\s*\[( |x)?\]\s$/, n.todo_list),
      textblockTypeInputRule(/^```([a-z]*)\s$/, n.code_block, (match) => ({ language: match[1] })),
      new InputRule(/^---$/, (state, _match, start, end) => state.tr.replaceWith(start - 1, end, n.horizontal_rule.create())),
    ],
  });
}

function buildKeymap(schema, cmd) {
  const n = schema.nodes;
  const keys = {
    "Mod-z": undo, "Shift-Mod-z": redo, "Mod-y": redo,
    "Mod-b": cmd.toggleBold, "Mod-i": cmd.toggleItalic, "Mod-u": cmd.toggleUnderline,
    "Mod-Shift-x": cmd.toggleStrike, "Mod-e": cmd.toggleCode,
    "Mod-Alt-0": cmd.paragraph, "Mod-Alt-1": cmd.heading(1), "Mod-Alt-2": cmd.heading(2), "Mod-Alt-3": cmd.heading(3),
    "Mod-Shift-7": cmd.list("ordered"), "Mod-Shift-8": cmd.list("bullet"), "Mod-Shift-9": cmd.list("todo"),
    "Mod-Shift-b": cmd.blockquote,
    "Backspace": undoInputRule,
    "Enter": chainCommands(splitListItem(n.list_item), splitListItem(n.todo_item), newlineInCode, createParagraphNear, liftEmptyBlock, splitBlock),
    "Tab": chainCommands(sinkListItem(n.list_item), sinkListItem(n.todo_item)),
    "Shift-Tab": chainCommands(liftListItem(n.list_item), liftListItem(n.todo_item)),
    "Mod-Enter": exitCode, "Shift-Enter": (state, dispatch) => { if (dispatch) dispatch(state.tr.replaceSelectionWith(n.hard_break.create()).scrollIntoView()); return true; },
    "Escape": selectParentNode,
  };
  return keymap(keys);
}

// ── createEditor ─────────────────────────────────────────────────────────────

/** The document to load: adopted into the schema, or empty when it cannot be. */
function loadDocument(schema, json, onError) {
  try {
    return PMNode.fromJSON(schema, adoptDocument(schema, json || EMPTY_DOC));
  } catch (err) {
    onError(err);
    return PMNode.fromJSON(schema, EMPTY_DOC);
  }
}

/**
 * Mount an editor into `mount`. Returns the instance; the caller renders every
 * piece of UI from the callbacks and drives edits through `instance.exec`.
 *
 * options: { doc, classes, extensions, placeholder, editable, onChange(json),
 *            onSlash(state|null), onTrigger(state|null), onNode(state|null),
 *            onSelection(state|null), onFocus(bool), onError(err),
 *            uploadImage(file) → src | { src, ref, alt } }
 */
export function createEditor(mount, options = {}) {
  const classes = options.classes || NO_CLASSES;
  const extensions = options.extensions || [];
  const schema = createSchema(classes, extensions);
  const cmd = buildCommands(schema);
  for (const ext of extensions) {
    for (const [name, command] of Object.entries(ext.commands ? ext.commands(schema, pm) : {})) {
      if (cmd[name]) throw new Error(`editor extension "${ext.name}" redefines command "${name}"`);
      cmd[name] = command;
    }
  }
  const noop = () => {};
  const onChange = options.onChange || noop;
  const onSlash = options.onSlash || noop;
  const onSelection = options.onSelection || noop;
  const onFocus = options.onFocus || noop;
  const onError = options.onError || noop;
  const toJSON = (doc) => restoreDocument(doc.toJSON());

  const doc = loadDocument(schema, options.doc, onError);

  const extensionRules = extensions.flatMap((ext) => (ext.inputRules ? ext.inputRules(schema, pm) : []));
  const plugins = [
    ...(extensionRules.length ? [inputRules({ rules: extensionRules })] : []),
    buildInputRules(schema),
    ...extensions.filter((ext) => ext.keymap).map((ext) => keymap(ext.keymap(schema, pm))),
    buildKeymap(schema, cmd),
    keymap(baseKeymap),
    history(),
    dropCursor({ class: classes.dropCursor || undefined }),
    gapCursor(),
    slashPlugin(onSlash),
    extensionTriggerPlugin(extensions, options.onTrigger || noop),
    nodePlugin(extensions, options.onNode || noop),
    selectionPlugin(onSelection),
    placeholderPlugin(options.placeholder || "", classes.placeholder),
    handlePlugin(classes.handle),
    todoPlugin(schema),
    imagePlugin(schema, options.uploadImage),
    ...extensions.flatMap((ext) => (ext.plugins ? ext.plugins(schema, pm) : [])),
  ].filter(Boolean);

  const state = EditorState.create({ doc, plugins });
  let view;
  view = new EditorView(mount, {
    state,
    editable: () => options.editable !== false,
    attributes: { class: classes.root || "", role: "textbox", "aria-multiline": "true" },
    dispatchTransaction(tr) {
      const next = view.state.apply(tr);
      view.updateState(next);
      if (tr.docChanged) onChange(toJSON(next.doc));
    },
    handleDOMEvents: {
      focus: () => { onFocus(true); return false; },
      blur: () => { onFocus(false); onSelection(null); return false; },
    },
  });

  function exec(command, ...args) {
    const fn = typeof command === "string" ? cmd[command] : command;
    if (!fn) return false;
    const run = args.length ? fn(...args) : fn;
    const ok = run(view.state, view.dispatch, view);
    view.focus();
    return ok;
  }

  return {
    view,
    schema,
    pm,
    commands: cmd,
    exec,
    focus: () => view.focus(),
    getJSON: () => toJSON(view.state.doc),
    setJSON(json) {
      view.updateState(EditorState.create({ doc: loadDocument(schema, json, onError), plugins }));
    },
    /** Remove the `/query` (or trigger) text a menu was opened with. */
    deleteSlash(slash) {
      view.dispatch(view.state.tr.delete(slash.from, slash.to));
      view.focus();
    },
    /**
     * Insert a node of `type` in place of `range` (default: the selection).
     * An inline node is followed by a space; a node with inline content takes
     * the caret inside it, so a figure's caption is typed straight away.
     */
    insertNode(type, attrs, range) {
      const nodeType = schema.nodes[type];
      if (!nodeType) throw new Error(`the editor has no node "${type}"`);
      const node = nodeType.createAndFill(attrs || null);
      if (!node) throw new Error(`"${type}" cannot be created empty`);
      const from = range ? range.from : view.state.selection.from;
      const to = range ? range.to : view.state.selection.to;
      const tr = view.state.tr.replaceRangeWith(from, to, node);
      if (node.isInline) tr.insertText(" ");
      else if (node.inlineContent) {
        const at = tr.mapping.map(from, -1);
        let inside = null;
        tr.doc.nodesBetween(at, Math.min(tr.doc.content.size, at + node.nodeSize + 2), (child, pos) => {
          if (inside === null && child.type === nodeType) inside = pos + 1;
          return inside === null;
        });
        if (inside !== null) tr.setSelection(TextSelection.create(tr.doc, inside));
      }
      view.dispatch(tr.scrollIntoView());
      view.focus();
      return true;
    },
    /** Change some attrs of the node at `pos`, keeping the rest. Does not take focus. */
    setNodeAttrs(pos, attrs) {
      const node = view.state.doc.nodeAt(pos);
      if (!node) return false;
      view.dispatch(view.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, ...attrs }));
      return true;
    },
    /** Serialize to HTML in the browser (the schema's toDOM). */
    getHTML() {
      const div = document.createElement("div");
      div.appendChild(DOMSerializer.fromSchema(schema).serializeFragment(view.state.doc.content));
      return div.innerHTML;
    },
    /** Load HTML (a legacy body) through the schema's parseDOM. */
    setHTML(html) {
      const div = document.createElement("div");
      div.innerHTML = html;
      const parsed = DOMParser.fromSchema(schema).parse(div);
      view.updateState(EditorState.create({ doc: parsed, plugins }));
      onChange(toJSON(parsed));
    },
    destroy: () => view.destroy(),
  };
}

/** Parse an HTML string into a document JSON (browser only; the schema's parseDOM). */
export function htmlToDocument(html, classes, extensions = []) {
  const schema = createSchema(classes, extensions);
  const div = document.createElement("div");
  div.innerHTML = html;
  return DOMParser.fromSchema(schema).parse(div).toJSON();
}
