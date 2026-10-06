import { defineExtension } from "zeb/ui/editor-extension";

/**
 * Figure extension — an image with a caption and a credit.
 *
 *   <Editor extensions={[figureExtension()]} uploadImage={upload} />   then `/figure`
 *
 * The caption is typed in place; the image address, alt text and credit
 * are set in the panel the caret inside a figure opens. With `uploadImage`
 * the figure starts from an uploaded picture; without it, from an address.
 */

export function figureExtension() {
  return defineExtension({
    name: "figure",
    node: {
      attrs: { src: { default: "" }, alt: { default: "" }, ref: { default: null }, credit: { default: "" } },
      content: "inline*", group: "block", defining: true, draggable: true,
    },
    render: (node, r) => {
      const a = node.attrs;
      const image = a.src
        ? r.h("img", { class: "w-full rounded-lg", src: a.src, alt: a.alt, "data-ref": a.ref || undefined, contenteditable: "false" })
        : r.edit ? r.h("div", { class: "rounded-lg border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground", contenteditable: "false" }, "No image yet — set its address in the panel") : null;
      const caption = r.edit || r.content.length ? r.h("figcaption", { class: "mt-2 text-sm text-muted-foreground" }, r.content) : null;
      const credit = a.credit ? r.h("p", { class: "mt-1 text-xs text-muted-foreground", "data-credit": "", contenteditable: "false" }, a.credit) : null;
      return r.h("figure", { class: "my-5", "data-figure": "" }, image, caption, credit);
    },
    parse: [{
      tag: "figure[data-figure]",
      contentElement: "figcaption",
      getAttrs: (dom) => {
        const img = dom.querySelector("img");
        const credit = dom.querySelector("[data-credit]");
        return { src: img ? img.getAttribute("src") || "" : "", alt: img ? img.getAttribute("alt") || "" : "", ref: img ? img.getAttribute("data-ref") : null, credit: credit ? credit.textContent : "" };
      },
    }],
    // Enter ends the caption: a new paragraph after the figure, not a second figure.
    keymap: (schema, pm) => ({
      Enter: (state, dispatch) => {
        const { $from } = state.selection;
        if ($from.parent.type !== schema.nodes.figure) return false;
        if (dispatch) {
          const after = $from.after();
          const tr = state.tr.insert(after, schema.nodes.paragraph.create());
          dispatch(tr.setSelection(pm.TextSelection.create(tr.doc, after + 1)).scrollIntoView());
        }
        return true;
      },
    }),
    insert: [{
      id: "figure", label: "Figure", hint: "Image, caption and credit", keys: "figure image photo caption credit",
      run: async (api) => {
        const image = api.canUpload ? await api.upload() : null;
        api.insert("figure", image ? { src: image.src, alt: image.alt || "", ref: image.ref || null } : null);
      },
    }],
    fields: [
      { name: "src", label: "Image address", type: "url", placeholder: "https://…" },
      { name: "alt", label: "Alt text" },
      { name: "credit", label: "Credit", placeholder: "Photo: …" },
    ],
  });
}

export default figureExtension;
