import { defineExtension } from "zeb/ui/editor-extension";

/**
 * Callout extension — a box with an icon and a tone around any blocks.
 *
 *   <Editor extensions={[calloutExtension()]} />   then `/callout`
 *
 * The caret inside a callout opens a panel for its icon and tone.
 */

const TONES = {
  note: "my-3 flex gap-3 rounded-lg border border-border bg-muted/60 px-4 py-3",
  info: "my-3 flex gap-3 rounded-lg border border-info/40 bg-info/10 px-4 py-3",
  success: "my-3 flex gap-3 rounded-lg border border-success/40 bg-success/10 px-4 py-3",
  warning: "my-3 flex gap-3 rounded-lg border border-warning/40 bg-warning/10 px-4 py-3",
  danger: "my-3 flex gap-3 rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3",
};

export function calloutExtension() {
  return defineExtension({
    name: "callout",
    node: { attrs: { icon: { default: "💡" }, tone: { default: "note" } }, content: "block+", group: "block", defining: true },
    render: (node, r) => r.h(
      "aside",
      { class: TONES[node.attrs.tone] || TONES.note, "data-callout": node.attrs.tone, "data-icon": node.attrs.icon },
      r.h("span", { class: "select-none text-lg leading-7", contenteditable: "false" }, node.attrs.icon),
      r.h("div", { class: "min-w-0 flex-1" }, r.content),
    ),
    parse: [{ tag: "aside[data-callout]", getAttrs: (dom) => ({ icon: dom.getAttribute("data-icon") || "💡", tone: dom.getAttribute("data-callout") || "note" }) }],
    insert: [{ id: "callout", label: "Callout", hint: "A box with an icon", keys: "callout note tip warning", run: (api) => api.wrap("callout") }],
    fields: [
      { name: "icon", label: "Icon" },
      { name: "tone", label: "Tone", type: "select", options: Object.keys(TONES) },
    ],
  });
}

export default calloutExtension;
