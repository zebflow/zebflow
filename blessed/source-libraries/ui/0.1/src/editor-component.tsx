import { h, render as zebRender } from "zeb/react";
import { sanitizeAttrs, sanitizeTag } from "zeb/ui/editor-extension";

/**
 * Editor components — the machinery behind an extension's `component` and
 * `editComponent` (`zeb/ui/editor-extension` states the contract).
 *
 * - `guardComponent(C)` is `C` with its output passed through the document
 *   allowlist. `<DocumentView>` and `renderDocumentHtml` render a node's
 *   `component` only through it, so a component cannot publish a script, a
 *   style or a `javascript:` link however its attrs were stored.
 * - `componentProps(node, ext, state)` is the one props object both
 *   components get.
 * - `withNodeViews(ext, getEditor, readOnly)` gives `<Editor>` the
 *   extension with a ProseMirror node view for each component node: the
 *   component is mounted with zeb/react's `render` into the node's element
 *   and drawn again whenever the node changes.
 *
 * Plain JavaScript (no JSX), like every editor extension file.
 */

const GUARDED = new WeakMap();
const FRAGMENT = Symbol.for("zeb.fragment");
const ERROR_BOUNDARY = Symbol.for("zeb.error-boundary");
// Kept as they are: they come from code, never from a stored document, and
// renderToString never prints them.
const CODE_PROPS = /^(on[A-Z].*|ref)$/;

function noop() {}

/** A component tree with every element passed through the allowlist. */
function sanitizeTree(value) {
  if (value === null || value === undefined || typeof value === "boolean") return value;
  if (typeof value === "string" || typeof value === "number" || typeof value === "bigint") return value;
  if (Array.isArray(value)) return value.map(sanitizeTree);
  if (typeof value !== "object" || value.type == null) return null;
  const { type, props = {}, key } = value;
  if (typeof type === "function") return { type: guardComponent(type), props, key };
  if (type === FRAGMENT || type === ERROR_BOUNDARY) return { type, props: { ...props, children: sanitizeTree(props.children) }, key };
  // A portal would draw outside the document; nothing else is an element.
  if (typeof type !== "string") return null;
  const name = sanitizeTag(type);
  if (!name) return null;
  const attrs = {};
  const code = {};
  for (const [prop, v] of Object.entries(props)) {
    if (prop === "children" || prop === "dangerouslySetInnerHTML") continue;
    if (CODE_PROPS.test(prop)) { if (typeof v === "function" || (prop === "ref" && v)) code[prop] = v; continue; }
    attrs[prop] = v;
  }
  const clean = sanitizeAttrs(name, attrs, false);
  // `class` printed last, as the document renderer prints it.
  if (clean.class !== undefined) { const cls = clean.class; delete clean.class; clean.className = cls; }
  return { type: name, props: { ...clean, ...code, children: sanitizeTree(props.children) }, key };
}

/** `component` with its output sanitized; the same wrapper for the same component. */
export function guardComponent(component) {
  if (typeof component !== "function") return component;
  if (GUARDED.has(component)) return GUARDED.get(component);
  const guarded = function GuardedComponent(props) { return sanitizeTree(component(props)); };
  for (const key of ["compare", "context", "defaultProps", "displayName"]) {
    if (component[key] !== undefined) guarded[key] = component[key];
  }
  GUARDED.set(component, guarded);
  return guarded;
}

/** The props an extension's component gets. `ext` is the defineExtension object. */
export function componentProps(node, ext, state = {}) {
  const attrs = { ...(node.attrs || {}) };
  return {
    node: { type: node.type.name || node.type, attrs },
    attrs,
    options: (ext && ext.options) || {},
    edit: Boolean(state.edit),
    selected: Boolean(state.selected),
    readOnly: state.readOnly !== false,
    update: state.update || noop,
  };
}

/**
 * The published element of a component node: what DocumentView and
 * renderDocumentHtml place. A stored node missing an attr gets the spec's
 * default, as the editor's engine gives it, so both draw the same node.
 */
export function componentElement(entry, node, key) {
  const attrs = {};
  for (const [name, spec] of Object.entries((entry.spec && entry.spec.attrs) || {})) {
    if (spec && "default" in spec) attrs[name] = spec.default;
  }
  Object.assign(attrs, node.attrs);
  return h(guardComponent(entry.component), { key, ...componentProps({ type: node.type, attrs }, entry.ext) });
}

// Events the component handles itself; ProseMirror leaves them alone.
const OWN_EVENTS = "input, textarea, select, button, label, a[href], [data-node-view-events]";

function nodeView(ext, entry, getEditor, readOnly) {
  const component = entry.editComponent || guardComponent(entry.component);
  return (node, view, getPos) => {
    const dom = document.createElement(node.isInline ? "span" : "div");
    dom.setAttribute("data-node-view", node.type.name);
    dom.setAttribute("contenteditable", "false");
    let current = node;
    let selected = false;
    const update = (patch) => {
      const pos = getPos();
      if (readOnly || typeof pos !== "number" || !patch) return;
      const editor = getEditor();
      if (editor) editor.setNodeAttrs(pos, patch);
    };
    const draw = () => zebRender(h(component, componentProps(current, ext, { edit: true, selected, readOnly, update })), dom);
    draw();
    return {
      dom,
      update(next) {
        if (next.type !== current.type) return false;
        current = next;
        draw();
        return true;
      },
      selectNode() { selected = true; dom.setAttribute("data-selected", ""); draw(); },
      deselectNode() { selected = false; dom.removeAttribute("data-selected"); draw(); },
      stopEvent: (event) => Boolean(event.target && event.target !== dom && event.target.closest && event.target.closest(OWN_EVENTS)),
      // The component (and whatever it mounts, a player's shadow root…) owns this DOM.
      ignoreMutation: () => true,
      destroy() { zebRender(null, dom); },
    };
  };
}

/**
 * The extension as `<Editor>` hands it to the engine: unchanged, or with one
 * more plugin holding a node view for each node drawn by a component.
 */
export function withNodeViews(ext, getEditor, readOnly) {
  const views = {};
  for (const [type, entry] of Object.entries(ext.renderers.nodes)) {
    if (entry.component || entry.editComponent) views[type] = nodeView(ext, entry, getEditor, readOnly);
  }
  if (Object.keys(views).length === 0) return ext;
  return {
    ...ext,
    plugins: (schema, pm) => [...(ext.plugins ? ext.plugins(schema, pm) : []), new pm.Plugin({ props: { nodeViews: views } })],
  };
}

export default guardComponent;
