/**
 * zeb/prosemirror 0.1 — ProseMirror rich text editor for RWE templates.
 *
 * ── FEATURES ────────────────────────────────────────────────────────────────
 *  • SPA / API-first: MutationObserver auto-mounts editors whenever they appear
 *    in the DOM (post-fetch renders, conditional rendering, loops).
 *  • Reactive two-way state bridge: stateKey ↔ usePageState via
 *    window.__rweSetPageState / rwe:state:change event.
 *  • statsKey: writes { words, chars, isDirty } to a separate page state key.
 *  • Extensible toolbar: built-in commands + registered custom plugins.
 *  • Built-in plugins: doodle (canvas drawing), photo (file/camera), link.
 *  • Instance registry: window.__zebProse.get(id) for imperative access.
 *  • toolbarMode: "inline" (always visible) | "bubble" (shows on selection).
 *  • Auto-cleanup: MutationObserver destroys instances when divs leave the DOM.
 *
 * ── OFFLINE BUNDLE ───────────────────────────────────────────────────────────
 *  All ProseMirror packages are bundled inline — no CDN fetches at runtime.
 *  This file is produced by:
 *
 *    cd /tmp/zeb-pm-build
 *    npm install
 *    node_modules/.bin/esbuild entry.mjs \
 *      --bundle --format=esm \
 *      --outfile=prosemirror.bundle.mjs
 *
 *  Then move prosemirror.bundle.mjs to:
 *    libraries/zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs
 *
 * ── QUICK REFERENCE ─────────────────────────────────────────────────────────
 *  TSX import:    import ProseEditor from "zeb/prosemirror";
 *  Register plugin: window.__zebProse.registerPlugin({ id, label, icon, onActivate })
 *  Imperative:    window.__zebProse.get("editor-id").getHTML() / setHTML(html)
 *  Events:        container.addEventListener("zeb:prose:change", e => e.detail.html)
 *  State event:   window.addEventListener("rwe:state:change", e => e.detail.myKey)
 */

/* ── Static imports — esbuild bundles these inline (no CDN at runtime) ── */
import * as _pmState      from "prosemirror-state";
import * as _pmView       from "prosemirror-view";
import * as _pmModel      from "prosemirror-model";
import * as _pmHistory    from "prosemirror-history";
import * as _pmKeymap     from "prosemirror-keymap";
import * as _pmCommands   from "prosemirror-commands";
import * as _pmSchemaBasic from "prosemirror-schema-basic";
import * as _pmSchemaList  from "prosemirror-schema-list";
import * as _pmGapcursor   from "prosemirror-gapcursor";

/* ── Module cache ─────────────────────────────────────────────────────────── */

/** Resolved synchronously — packages are already bundled in. */
let _pm = null;

async function loadPm() {
  if (_pm) return _pm;
  _pm = {
    state:       _pmState,
    view:        _pmView,
    model:       _pmModel,
    history:     _pmHistory,
    keymap:      _pmKeymap,
    commands:    _pmCommands,
    schemaBasic: _pmSchemaBasic,
    schemaList:  _pmSchemaList,
    gapcursor:   _pmGapcursor,
  };
  return _pm;
}

/* ── CSS injection ────────────────────────────────────────────────────────── */

/**
 * Inject editor styles once into <head>.  Uses CSS custom properties from the
 * zebflow / shadcn-ui design token set so the editor matches the platform theme
 * without requiring Tailwind JIT to scan dynamically-created DOM nodes.
 */
let _cssInjected = false;
function injectCss() {
  if (_cssInjected) return;
  _cssInjected = true;
  const style = document.createElement("style");
  style.setAttribute("data-zeb-prose", "");
  style.textContent = `
    /* ── Toolbar ── */
    .zp-toolbar {
      display: flex; flex-wrap: wrap; gap: 2px; padding: 6px 8px;
      border-bottom: 1px solid var(--border, #334155);
      background: hsl(var(--muted, 217 33% 17%) / 0.8);
      border-radius: 6px 6px 0 0;
    }
    .zp-btn {
      display: inline-flex; align-items: center; justify-content: center;
      width: 28px; height: 28px; padding: 0; border: none; border-radius: 4px;
      background: transparent; cursor: pointer; font-size: 12px; font-weight: 600;
      color: var(--muted-foreground, #94a3b8); transition: background 0.1s, color 0.1s;
    }
    .zp-btn:hover { background: hsl(var(--accent, 217 33% 27%)); color: var(--foreground, #f1f5f9); }
    .zp-btn.active { background: hsl(var(--accent, 217 33% 27%)); color: hsl(var(--accent-foreground, 210 40% 98%)); }
    .zp-btn:disabled { opacity: 0.35; cursor: default; }
    .zp-btn.loading { pointer-events: none; opacity: 0.6; }
    .zp-sep { width: 1px; height: 18px; background: var(--border, #334155); margin: 0 2px; align-self: center; flex-shrink: 0; }

    /* ── Editor area ── */
    .zp-editor { outline: none; padding: 12px 14px; min-height: inherit; cursor: text; color: var(--foreground, #f1f5f9); }
    .zp-editor .ProseMirror { outline: none; min-height: inherit; }
    .zp-editor .ProseMirror > * + * { margin-top: 0.6em; }
    .zp-editor .ProseMirror h1 { font-size: 1.6em; font-weight: 700; }
    .zp-editor .ProseMirror h2 { font-size: 1.3em; font-weight: 700; }
    .zp-editor .ProseMirror h3 { font-size: 1.1em; font-weight: 600; }
    .zp-editor .ProseMirror ul { list-style: disc; padding-left: 1.4em; }
    .zp-editor .ProseMirror ol { list-style: decimal; padding-left: 1.4em; }
    .zp-editor .ProseMirror blockquote { border-left: 3px solid var(--border,#334155); padding-left: 1em; color: var(--muted-foreground,#94a3b8); }
    .zp-editor .ProseMirror code { background: hsl(var(--muted,217 33% 17%)); padding: 0.1em 0.3em; border-radius: 3px; font-family: monospace; font-size: 0.88em; }
    .zp-editor .ProseMirror pre { background: hsl(var(--muted,217 33% 17%)); padding: 0.8em 1em; border-radius: 6px; overflow-x: auto; }
    .zp-editor .ProseMirror pre code { background: none; padding: 0; }
    .zp-editor .ProseMirror hr { border: none; border-top: 1px solid var(--border,#334155); margin: 1em 0; }
    .zp-editor .ProseMirror img { max-width: 100%; height: auto; border-radius: 4px; }
    /* placeholder */
    .zp-editor .ProseMirror p.is-empty:first-child::before {
      content: attr(data-placeholder); color: var(--muted-foreground,#94a3b8);
      pointer-events: none; float: left; height: 0;
    }
    /* gapcursor */
    .zp-editor .ProseMirror .ProseMirror-gapcursor { display: none; pointer-events: none; position: absolute; }
    .zp-editor .ProseMirror.ProseMirror-focused .ProseMirror-gapcursor { display: block; }

    /* ── Bubble toolbar ── */
    .zp-bubble {
      position: fixed; z-index: 9999; display: flex; gap: 2px; padding: 4px 6px;
      background: var(--background,#1e293b); border: 1px solid var(--border,#334155);
      border-radius: 6px; box-shadow: 0 4px 12px rgba(0,0,0,0.4);
      pointer-events: auto; transition: opacity 0.1s;
    }
    .zp-bubble.hidden { opacity: 0; pointer-events: none; }

    /* ── Doodle overlay ── */
    .zp-doodle-overlay {
      position: fixed; inset: 0; z-index: 9998; background: rgba(0,0,0,0.55);
      display: flex; align-items: center; justify-content: center;
    }
    .zp-doodle-box {
      background: #fff; border-radius: 10px; overflow: hidden;
      display: flex; flex-direction: column; box-shadow: 0 8px 32px rgba(0,0,0,0.2);
      max-width: 90vw; max-height: 90vh;
    }
    .zp-doodle-bar {
      display: flex; gap: 8px; padding: 8px 12px; border-bottom: 1px solid #e2e8f0;
      align-items: center;
    }
    .zp-doodle-canvas { display: block; touch-action: none; cursor: crosshair; }
    .zp-doodle-btn {
      padding: 4px 12px; border-radius: 4px; border: 1px solid #e2e8f0;
      background: #fff; cursor: pointer; font-size: 13px; font-weight: 500;
    }
    .zp-doodle-btn.primary { background: #1d4ed8; color: #fff; border-color: #1d4ed8; }
    .zp-doodle-btn:hover { opacity: 0.85; }
  `;
  document.head.appendChild(style);
}

/* ── Schema ───────────────────────────────────────────────────────────────── */

/**
 * Build the ProseMirror schema: basic schema + list nodes + strikethrough
 * and underline marks.  The schema is shared across all editor instances on
 * the page (created once after the first mount).
 */
let _schema = null;
function getSchema(pm) {
  if (_schema) return _schema;
  const { Schema } = pm.model;
  const { addListNodes } = pm.schemaList;
  const { schema: basicSchema } = pm.schemaBasic;

  const nodes = addListNodes(basicSchema.spec.nodes, "paragraph block*", "block");
  const marks = basicSchema.spec.marks.append({
    strikethrough: {
      parseDOM: [{ tag: "s" }, { tag: "strike" }, { style: "text-decoration=line-through" }],
      toDOM() { return ["s"]; },
    },
    underline: {
      parseDOM: [{ tag: "u" }, { style: "text-decoration=underline" }],
      toDOM() { return ["u"]; },
    },
  });

  _schema = new Schema({ nodes, marks });
  return _schema;
}

/* ── Toolbar presets ──────────────────────────────────────────────────────── */

const TOOLBAR_PRESETS = {
  /** Minimal: just bold, italic, strike — for comments, short notes. */
  minimal: ["bold", "italic", "strike"],
  /** Basic: headings + lists + blockquote — for answers, descriptions. */
  basic:   ["bold", "italic", "strike", "|", "h1", "h2", "|", "bulletList", "orderedList", "|", "blockquote"],
  /** Full: everything — for blog posts, rich documents. */
  full:    ["bold", "italic", "strike", "underline", "code", "|",
            "h1", "h2", "h3", "|",
            "bulletList", "orderedList", "|",
            "blockquote", "codeBlock", "horizontalRule", "|",
            "link", "doodle", "photo", "|",
            "undo", "redo"],
};

/* ── Toolbar SVG icons ────────────────────────────────────────────────────── */

const ICONS = {
  bold:            `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 4h8a4 4 0 0 1 0 8H6z"/><path d="M6 12h9a4 4 0 0 1 0 8H6z"/></svg>`,
  italic:          `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="19" y1="4" x2="10" y2="4"/><line x1="14" y1="20" x2="5" y2="20"/><line x1="15" y1="4" x2="9" y2="20"/></svg>`,
  strike:          `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M16 4H9a3 3 0 0 0 0 6h6a3 3 0 0 1 0 6H6"/><line x1="4" y1="12" x2="20" y2="12"/></svg>`,
  underline:       `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M6 4v6a6 6 0 0 0 12 0V4"/><line x1="4" y1="20" x2="20" y2="20"/></svg>`,
  code:            `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>`,
  h1:              `<span style="font-size:11px;font-weight:700;line-height:1">H1</span>`,
  h2:              `<span style="font-size:11px;font-weight:700;line-height:1">H2</span>`,
  h3:              `<span style="font-size:11px;font-weight:700;line-height:1">H3</span>`,
  bulletList:      `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="9" y1="6" x2="20" y2="6"/><line x1="9" y1="12" x2="20" y2="12"/><line x1="9" y1="18" x2="20" y2="18"/><circle cx="4" cy="6" r="1.5" fill="currentColor" stroke="none"/><circle cx="4" cy="12" r="1.5" fill="currentColor" stroke="none"/><circle cx="4" cy="18" r="1.5" fill="currentColor" stroke="none"/></svg>`,
  orderedList:     `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="10" y1="6" x2="21" y2="6"/><line x1="10" y1="12" x2="21" y2="12"/><line x1="10" y1="18" x2="21" y2="18"/><text x="2" y="9" font-size="7" fill="currentColor" stroke="none" font-weight="700">1.</text></svg>`,
  blockquote:      `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V20c0 1 0 1 1 1z"/><path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3c0 1 0 1 1 1z"/></svg>`,
  codeBlock:       `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="2" y="3" width="20" height="18" rx="2"/><polyline points="8 9 12 13 8 17"/><line x1="16" y1="17" x2="12" y2="17"/></svg>`,
  horizontalRule:  `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="3" y1="12" x2="21" y2="12"/></svg>`,
  link:            `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>`,
  doodle:          `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>`,
  photo:           `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>`,
  undo:            `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 .49-3.5"/></svg>`,
  redo:            `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-.49-3.5"/></svg>`,
};

/* ── Mark/node active-state helpers ──────────────────────────────────────── */

/** Returns true when the given mark type is active at the current selection. */
function isMarkActive(state, markType) {
  if (!markType) return false;
  const { from, $from, to, empty } = state.selection;
  if (empty) return !!markType.isInSet(state.storedMarks || $from.marks());
  return state.doc.rangeHasMark(from, to, markType);
}

/**
 * Returns true when the selection is inside a node of the given type.
 * For headings, pass attrs = { level: N }.
 */
function isNodeActive(state, nodeType, attrs) {
  const { $from, to } = state.selection;
  let found = false;
  state.doc.nodesBetween($from.pos, to, (node) => {
    if (node.type === nodeType) {
      if (!attrs) { found = true; return false; }
      let match = true;
      for (const k in attrs) if (node.attrs[k] !== attrs[k]) { match = false; }
      if (match) found = true;
    }
  });
  return found;
}

/* ── Built-in command actions ─────────────────────────────────────────────── */

/**
 * Maps toolbar item names to { run(state, dispatch, view), isActive(state) }.
 * Built lazily once the schema is available.
 */
function buildCommandMap(pm, schema) {
  const { toggleMark, setBlockType, wrapIn, lift, chainCommands, exitCode,
          joinBackward, selectNodeBackward, joinForward, selectNodeForward,
          deleteSelection, newlineInCode, createParagraphNear, liftEmptyBlock,
          splitBlock, baseKeymap } = pm.commands;
  const { undo, redo, history } = pm.history;
  const { wrapInList, splitListItem, liftListItem, sinkListItem } = pm.schemaList;
  const m = schema.marks;
  const n = schema.nodes;

  return {
    bold:           { run: toggleMark(m.strong),                 isActive: s => isMarkActive(s, m.strong) },
    italic:         { run: toggleMark(m.em),                     isActive: s => isMarkActive(s, m.em) },
    strike:         { run: toggleMark(m.strikethrough),          isActive: s => isMarkActive(s, m.strikethrough) },
    underline:      { run: toggleMark(m.underline),              isActive: s => isMarkActive(s, m.underline) },
    code:           { run: toggleMark(m.code),                   isActive: s => isMarkActive(s, m.code) },
    h1:             { run: setBlockType(n.heading, { level: 1 }), isActive: s => isNodeActive(s, n.heading, { level: 1 }) },
    h2:             { run: setBlockType(n.heading, { level: 2 }), isActive: s => isNodeActive(s, n.heading, { level: 2 }) },
    h3:             { run: setBlockType(n.heading, { level: 3 }), isActive: s => isNodeActive(s, n.heading, { level: 3 }) },
    bulletList:     { run: wrapInList(n.bullet_list),            isActive: s => isNodeActive(s, n.bullet_list) },
    orderedList:    { run: wrapInList(n.ordered_list),           isActive: s => isNodeActive(s, n.ordered_list) },
    blockquote:     { run: wrapIn(n.blockquote),                 isActive: s => isNodeActive(s, n.blockquote) },
    codeBlock:      { run: setBlockType(n.code_block),           isActive: s => isNodeActive(s, n.code_block) },
    horizontalRule: {
      run: (state, dispatch) => {
        if (dispatch) dispatch(state.tr.replaceSelectionWith(n.horizontal_rule.create()));
        return true;
      },
      isActive: () => false,
    },
    undo: { run: undo, isActive: () => false },
    redo: { run: redo, isActive: () => false },
  };
}

/* ── Built-in plugins ─────────────────────────────────────────────────────── */

/**
 * "link" plugin — prompts for a URL and wraps the selection in a link mark.
 * TODO: replace the prompt() with a proper inline popover UI.
 */
const PLUGIN_LINK = {
  id: "link",
  label: "Link",
  icon: ICONS.link,
  onActivate(ctx) {
    const existing = ctx._getLinkAtSelection();
    const url = window.prompt("URL", existing || "https://");
    if (url === null) return;           // cancelled
    if (url === "") {
      ctx._removeLink();
    } else {
      ctx._setLink(url);
    }
  },
};

/**
 * "doodle" plugin — canvas drawing overlay.
 * Opens a full-screen canvas, lets the user draw with mouse or touch,
 * then inserts the drawing as a base64 PNG <img> into the editor.
 */
const PLUGIN_DOODLE = {
  id: "doodle",
  label: "Draw",
  icon: ICONS.doodle,
  onActivate(ctx) {
    /* ── Overlay structure ── */
    const overlay = document.createElement("div");
    overlay.className = "zp-doodle-overlay";

    const box = document.createElement("div");
    box.className = "zp-doodle-box";

    /* Toolbar bar */
    const bar = document.createElement("div");
    bar.className = "zp-doodle-bar";
    bar.innerHTML = `
      <span style="font-size:13px;font-weight:600;flex:1">Draw</span>
      <label style="font-size:12px">
        Colour: <input type="color" id="zp-doodle-color" value="#000000" style="width:32px;height:24px;border:none;cursor:pointer">
      </label>
      <label style="font-size:12px">
        Size: <input type="range" id="zp-doodle-size" min="1" max="20" value="3" style="width:64px">
      </label>
      <button class="zp-doodle-btn" id="zp-doodle-clear">Clear</button>
      <button class="zp-doodle-btn" id="zp-doodle-cancel">Cancel</button>
      <button class="zp-doodle-btn primary" id="zp-doodle-insert">Insert</button>
    `;

    /* Canvas */
    const canvas = document.createElement("canvas");
    canvas.className = "zp-doodle-canvas";
    canvas.width  = Math.min(window.innerWidth  * 0.85, 800);
    canvas.height = Math.min(window.innerHeight * 0.72, 520);
    canvas.style.background = "#fff";

    box.appendChild(bar);
    box.appendChild(canvas);
    overlay.appendChild(box);
    document.body.appendChild(overlay);

    /* ── Drawing logic ── */
    const ctx2d = canvas.getContext("2d");
    ctx2d.lineCap = "round";
    ctx2d.lineJoin = "round";
    let drawing = false;

    function getXY(e) {
      const r = canvas.getBoundingClientRect();
      const src = e.touches ? e.touches[0] : e;
      return [src.clientX - r.left, src.clientY - r.top];
    }
    function startDraw(e) {
      drawing = true;
      const [x, y] = getXY(e);
      ctx2d.beginPath();
      ctx2d.moveTo(x, y);
      e.preventDefault();
    }
    function draw(e) {
      if (!drawing) return;
      const [x, y] = getXY(e);
      const colorPicker = bar.querySelector("#zp-doodle-color");
      const sizePicker  = bar.querySelector("#zp-doodle-size");
      ctx2d.strokeStyle = colorPicker ? colorPicker.value : "#000";
      ctx2d.lineWidth   = sizePicker  ? Number(sizePicker.value) : 3;
      ctx2d.lineTo(x, y);
      ctx2d.stroke();
      e.preventDefault();
    }
    function endDraw() { drawing = false; }

    canvas.addEventListener("mousedown",  startDraw);
    canvas.addEventListener("mousemove",  draw);
    canvas.addEventListener("mouseleave", endDraw);
    canvas.addEventListener("mouseup",    endDraw);
    canvas.addEventListener("touchstart", startDraw, { passive: false });
    canvas.addEventListener("touchmove",  draw,      { passive: false });
    canvas.addEventListener("touchend",   endDraw);

    /* ── Button handlers ── */
    bar.querySelector("#zp-doodle-clear").onclick = () => {
      ctx2d.clearRect(0, 0, canvas.width, canvas.height);
    };
    bar.querySelector("#zp-doodle-cancel").onclick = () => {
      document.body.removeChild(overlay);
    };
    bar.querySelector("#zp-doodle-insert").onclick = () => {
      const dataUrl = canvas.toDataURL("image/png");
      ctx.insertImage(dataUrl, "Drawing");
      document.body.removeChild(overlay);
    };
  },
};

/**
 * "photo" plugin — file/camera picker.
 * On mobile this opens the camera; on desktop it opens the file browser.
 * The selected image is read as a base64 data URL and inserted as <img>.
 * No server upload needed — image is embedded inline.
 */
const PLUGIN_PHOTO = {
  id: "photo",
  label: "Photo",
  icon: ICONS.photo,
  onActivate(ctx) {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.capture = "environment";   // mobile: prefer back camera
    input.style.display = "none";
    document.body.appendChild(input);

    input.onchange = () => {
      const file = input.files?.[0];
      if (!file) { document.body.removeChild(input); return; }

      const reader = new FileReader();
      reader.onload = (e) => {
        ctx.insertImage(e.target.result, file.name);
        document.body.removeChild(input);
      };
      reader.readAsDataURL(file);
    };

    input.click();
  },
};

/* ── Plugin registry ──────────────────────────────────────────────────────── */

/**
 * Module-level plugin map.  Seeded with built-in plugins.
 * Users call window.__zebProse.registerPlugin({ id, label, icon, onActivate })
 * from behavior files to add custom toolbar items.
 */
const _plugins = new Map([
  [PLUGIN_LINK.id,   PLUGIN_LINK],
  [PLUGIN_DOODLE.id, PLUGIN_DOODLE],
  [PLUGIN_PHOTO.id,  PLUGIN_PHOTO],
]);

/* ── Word/char counting ───────────────────────────────────────────────────── */

function countWords(doc) {
  const text = doc.textContent.trim();
  return text ? text.split(/\s+/).length : 0;
}

/* ── Toolbar builder ──────────────────────────────────────────────────────── */

/**
 * Creates and returns the toolbar DOM element for the given item list.
 *
 * @param {string[]} items     - Array of command names, plugin ids, or "|" separators.
 * @param {object}   cmdMap    - Built-in command map from buildCommandMap().
 * @param {function} onCommand - Called with (cmdName, view) when a button is clicked.
 *
 * Button elements are given data-cmd attributes so updateToolbarActiveState()
 * can toggle the "active" class without rebuilding the whole toolbar.
 */
function buildToolbar(items, cmdMap, onCommand) {
  const bar = document.createElement("div");
  bar.className = "zp-toolbar";

  for (const item of items) {
    if (item === "|") {
      const sep = document.createElement("div");
      sep.className = "zp-sep";
      bar.appendChild(sep);
      continue;
    }

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "zp-btn";
    btn.dataset.cmd = item;
    btn.title = item;

    /* Icon: prefer built-in SVG, fallback to label text for custom plugins. */
    const icon = ICONS[item] || _plugins.get(item)?.icon || item;
    btn.innerHTML = icon;

    btn.addEventListener("click", (e) => {
      e.preventDefault();
      onCommand(item, btn);
    });

    bar.appendChild(btn);
  }

  return bar;
}

/**
 * Update toolbar button active/disabled states after each editor transaction.
 * Called from dispatchTransaction so the toolbar always reflects current selection.
 */
function updateToolbarActiveState(toolbarEl, editorState, cmdMap) {
  if (!toolbarEl) return;
  toolbarEl.querySelectorAll(".zp-btn[data-cmd]").forEach((btn) => {
    const cmd = btn.dataset.cmd;
    const def = cmdMap[cmd];
    if (def && def.isActive) {
      btn.classList.toggle("active", def.isActive(editorState));
    }
  });
}

/* ── HTML helpers ─────────────────────────────────────────────────────────── */

function getHTML(view, schema, pm) {
  const { DOMSerializer } = pm.model;
  const serializer = DOMSerializer.fromSchema(schema);
  const div = document.createElement("div");
  div.appendChild(serializer.serializeFragment(view.state.doc.content));
  return div.innerHTML;
}

function parseHTML(html, schema, pm) {
  const { DOMParser: PMParser } = pm.model;
  const div = document.createElement("div");
  div.innerHTML = html;
  return PMParser.fromSchema(schema).parse(div);
}

/* ── Bubble toolbar (toolbarMode="bubble") ────────────────────────────────── */

/**
 * Creates a floating toolbar that appears above the selected text.
 * Hidden when selection is empty.  Positioned using getBoundingClientRect
 * on the selection range so it follows the cursor on scroll.
 */
function setupBubbleToolbar(bubbleEl, view) {
  function updateBubble() {
    const { from, to, empty } = view.state.selection;
    if (empty) {
      bubbleEl.classList.add("hidden");
      return;
    }
    /* Use the native selection range to get screen coordinates. */
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) { bubbleEl.classList.add("hidden"); return; }
    const range = sel.getRangeAt(0);
    const rect  = range.getBoundingClientRect();
    const bRect = bubbleEl.getBoundingClientRect();

    bubbleEl.style.top  = `${rect.top  - bRect.height - 8 + window.scrollY}px`;
    bubbleEl.style.left = `${Math.max(4, rect.left + rect.width / 2 - bRect.width / 2)}px`;
    bubbleEl.classList.remove("hidden");
  }

  /* Update position after Preact/PM re-renders and on window scroll. */
  view.dom.addEventListener("mouseup", () => requestAnimationFrame(updateBubble));
  view.dom.addEventListener("keyup",   () => requestAnimationFrame(updateBubble));
  document.addEventListener("selectionchange", () => requestAnimationFrame(updateBubble));
  window.addEventListener("scroll", updateBubble, { passive: true });
}

/* ── Core mount ───────────────────────────────────────────────────────────── */

/**
 * Mount a ProseMirror editor into the given container element.
 *
 * Reads configuration from data-config JSON.  Called by the MutationObserver
 * whenever a [data-zeb-lib="prosemirror"] element is added to the DOM.
 *
 * @param {HTMLElement} container - The div[data-zeb-lib="prosemirror"] element.
 * @returns {Promise<void>}
 */
async function mountEditor(container) {
  /* Prevent double-mount. */
  if (container._zpMounted) return;
  container._zpMounted = true;

  const pm     = await loadPm();
  const schema = getSchema(pm);
  injectCss();

  /* ── Parse config ── */
  let config = {};
  try { config = JSON.parse(container.dataset.config || "{}"); } catch {}

  const {
    content     = "",
    stateKey    = null,
    statsKey    = null,
    editable    = true,
    autofocus   = false,
    placeholder = "",
    toolbar     = "basic",
    toolbarMode = "inline",
  } = config;

  /* ── Auto-generate id if missing ── */
  if (!container.id) {
    container.id = `zp-${Math.random().toString(36).slice(2, 8)}`;
  }
  const instanceId = container.id;

  /* ── Determine initial content ──────────────────────────────────────────
   * Priority:
   *   1. window.__rwePageState[stateKey]  — page state already has data
   *      (API fetch completed before editor mounted in the DOM)
   *   2. content prop                     — baked at Preact render time
   *   3. empty                            — user will type / API arrives later
   * ─────────────────────────────────────────────────────────────────────── */
  let initialHtml = content || "";
  if (stateKey && window.__rwePageState?.[stateKey]) {
    initialHtml = window.__rwePageState[stateKey];
  }

  /* ── Build initial PM document ── */
  const initialDoc = initialHtml
    ? parseHTML(initialHtml, schema, pm)
    : schema.node("doc", null, [schema.node("paragraph")]);

  /* ── Plugins ── */
  const { history } = pm.history;
  const { keymap }  = pm.keymap;
  const { baseKeymap, undo, redo } = pm.commands;
  const { GapCursor, gapCursor }   = pm.gapcursor;

  const pmPlugins = [
    history(),
    keymap({ "Mod-z": undo, "Mod-y": redo, "Mod-Shift-z": redo }),
    keymap(baseKeymap),
    gapCursor(),
  ];

  const editorState = pm.state.EditorState.create({
    schema,
    doc: initialDoc,
    plugins: pmPlugins,
  });

  /* ── DOM structure ──────────────────────────────────────────────────────
   * container (the data-zeb-lib div)
   *   ├── toolbarEl  (or bubbleEl fixed)
   *   └── editorWrap (the div the PM view mounts into)
   * ─────────────────────────────────────────────────────────────────────── */
  container.style.display = "flex";
  container.style.flexDirection = "column";
  /* Default min-height so the editor is visible even without Tailwind classes.
   * Callers can override via the container's own style or className after mount. */
  if (!container.style.minHeight) container.style.minHeight = "200px";

  const editorWrap = document.createElement("div");
  editorWrap.className = "zp-editor";
  editorWrap.style.flex = "1";

  /* Resolve toolbar item list from preset or custom array. */
  let toolbarItems = [];
  if (toolbar && toolbar !== false) {
    toolbarItems = Array.isArray(toolbar) ? toolbar : (TOOLBAR_PRESETS[toolbar] || TOOLBAR_PRESETS.basic);
  }

  /* Command map is built once per page (schema is shared). */
  const cmdMap = buildCommandMap(pm, schema);

  /* ── Last-HTML tracker — used to break stateKey → PM → stateKey loops ── */
  let _lastHtml = initialHtml;
  let _isDirty  = false;

  /* Forward declaration: view is used inside dispatchTransaction. */
  let view;

  /* ── Toolbar element ── */
  let toolbarEl = null;
  let bubbleEl  = null;

  if (toolbarItems.length > 0 && editable) {
    if (toolbarMode === "bubble") {
      /* Bubble: a floating div appended to body, positioned after each selection. */
      bubbleEl = buildToolbar(toolbarItems, cmdMap, handleToolbarCommand);
      bubbleEl.className = "zp-bubble hidden";
      document.body.appendChild(bubbleEl);
    } else {
      /* Inline: sits above the editor inside the container. */
      toolbarEl = buildToolbar(toolbarItems, cmdMap, handleToolbarCommand);
      container.appendChild(toolbarEl);
    }
  }

  container.appendChild(editorWrap);

  /* ── PM view ── */
  view = new pm.view.EditorView(editorWrap, {
    state: editorState,
    editable: () => editable,

    dispatchTransaction(tr) {
      const newState = view.state.apply(tr);
      view.updateState(newState);
      updateToolbarActiveState(toolbarEl || bubbleEl, newState, cmdMap);

      if (!tr.docChanged) return;

      const html  = getHTML(view, schema, pm);
      const words = countWords(view.state.doc);
      const chars = view.state.doc.textContent.length;
      _isDirty  = true;
      _lastHtml = html;

      /* ── Push to page state (stateKey two-way bridge) ── */
      if (stateKey) {
        window.__rweSetPageState?.({ [stateKey]: html });
      }

      /* ── Push stats to page state (statsKey one-way) ── */
      if (statsKey) {
        window.__rweSetPageState?.({ [statsKey]: { words, chars, isDirty: _isDirty } });
      }

      /* ── Fire custom event on the container ── */
      container.dispatchEvent(new CustomEvent("zeb:prose:change", {
        bubbles: true,
        detail: { html, json: view.state.doc.toJSON(), words, chars, isDirty: _isDirty },
      }));
    },

    /* Placeholder: add data attribute on empty paragraphs so CSS ::before works. */
    nodeViews: placeholder ? {
      paragraph(node, nodeView, getPos) {
        const dom = document.createElement("p");
        function update(node) {
          const isEmpty = node.content.size === 0;
          if (isEmpty) {
            dom.setAttribute("data-placeholder", placeholder);
            dom.classList.add("is-empty");
          } else {
            dom.removeAttribute("data-placeholder");
            dom.classList.remove("is-empty");
          }
          return true;
        }
        update(node);
        return { dom, contentDOM: dom, update };
      }
    } : {},
  });

  if (autofocus) setTimeout(() => view.focus(), 0);
  if (toolbarMode === "bubble" && bubbleEl) setupBubbleToolbar(bubbleEl, view);

  /* ── stateKey reactive listener (Preact → PM direction) ─────────────────
   * Fires when the page calls setMyKey(newHtml) — e.g. examiner navigating
   * through submissions, or API populating an empty editor after fetch.
   *
   * Anti-loop: skip if the incoming value equals _lastHtml (meaning we just
   * dispatched it ourselves in dispatchTransaction).
   * ─────────────────────────────────────────────────────────────────────── */
  let _stateListener = null;
  if (stateKey) {
    _stateListener = (e) => {
      const incoming = e.detail?.[stateKey];
      if (incoming === undefined || incoming === _lastHtml) return;
      _lastHtml = incoming;
      const newDoc = incoming
        ? parseHTML(incoming, schema, pm)
        : schema.node("doc", null, [schema.node("paragraph")]);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
    };
    window.addEventListener("rwe:state:change", _stateListener);
  }

  /* ── Plugin context factory ── */
  function makePluginCtx() {
    const { schema: s } = view.state;
    const linkMark = s.marks.link;
    return {
      view,
      container,

      getHTML()         { return getHTML(view, schema, pm); },
      getJSON()         { return view.state.doc.toJSON(); },
      getSelectedText() {
        const { from, to } = view.state.selection;
        return view.state.doc.textBetween(from, to, " ");
      },

      /**
       * Insert raw HTML at the current cursor position (or replace selection).
       * Parses the HTML string into PM nodes via a temp div.
       */
      insertHTML(html) {
        const div = document.createElement("div");
        div.innerHTML = html;
        const { DOMParser: PMParser } = pm.model;
        const slice = PMParser.fromSchema(schema).parseSlice(div);
        view.dispatch(view.state.tr.replaceSelection(slice));
        view.focus();
      },

      /** Insert an <img src="{src}" alt="{alt}"> at the cursor. */
      insertImage(src, alt = "") {
        const node = schema.nodes.image.create({ src, alt });
        view.dispatch(view.state.tr.replaceSelectionWith(node));
        view.focus();
      },

      /** Replace the current selection with raw HTML. */
      replaceSelection(html) {
        const div = document.createElement("div");
        div.innerHTML = html;
        const { DOMParser: PMParser } = pm.model;
        const slice = PMParser.fromSchema(schema).parseSlice(div);
        view.dispatch(view.state.tr.replaceSelection(slice));
        view.focus();
      },

      /**
       * Open an arbitrary HTML element as an overlay over the editor.
       * Returns a close() function to remove the overlay.
       */
      openOverlay(el) {
        document.body.appendChild(el);
        return () => el.parentNode && document.body.removeChild(el);
      },

      /** Show/hide a loading spinner on the triggering toolbar button. */
      setLoading(loading, btnEl) {
        if (btnEl) btnEl.classList.toggle("loading", loading);
      },

      /* Internal helpers for the link plugin. */
      _getLinkAtSelection() {
        if (!linkMark) return null;
        const { from, $from } = view.state.selection;
        const marks = $from.marks();
        const lm = marks.find(m => m.type === linkMark);
        return lm?.attrs.href || null;
      },
      _setLink(href) {
        if (!linkMark) return;
        const { from, to } = view.state.selection;
        view.dispatch(view.state.tr.addMark(from, to, linkMark.create({ href })));
        view.focus();
      },
      _removeLink() {
        if (!linkMark) return;
        const { from, to } = view.state.selection;
        view.dispatch(view.state.tr.removeMark(from, to, linkMark));
        view.focus();
      },
    };
  }

  /* ── Toolbar command handler ── */
  function handleToolbarCommand(item, btnEl) {
    if (cmdMap[item]) {
      /* Built-in PM command */
      cmdMap[item].run(view.state, view.dispatch, view);
      view.focus();
    } else if (_plugins.has(item)) {
      /* Plugin */
      const plugin = _plugins.get(item);
      const ctx = makePluginCtx();
      ctx.setLoading(true, btnEl);
      Promise.resolve(plugin.onActivate(ctx)).finally(() => ctx.setLoading(false, btnEl));
    }
  }

  /* ── Public instance ── */
  const instance = {
    /** Get current editor content as HTML string. */
    getHTML()       { return getHTML(view, schema, pm); },
    /** Get current content as a ProseMirror JSON document. */
    getJSON()       { return view.state.doc.toJSON(); },
    /** Replace editor content with an HTML string. */
    setHTML(html)   {
      const newDoc = html
        ? parseHTML(html, schema, pm)
        : schema.node("doc", null, [schema.node("paragraph")]);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      _lastHtml = html;
      view.dispatch(tr);
    },
    /** Replace editor content with a ProseMirror JSON document object. */
    setJSON(json)   {
      const newDoc = schema.nodeFromJSON(json);
      const tr = view.state.tr.replaceWith(0, view.state.doc.content.size, newDoc.content);
      view.dispatch(tr);
    },
    focus()  { view.focus(); },
    blur()   { view.dom.blur(); },

    /** Destroy the editor, clean up listeners, remove from instance registry. */
    destroy() {
      if (_stateListener) window.removeEventListener("rwe:state:change", _stateListener);
      if (bubbleEl && bubbleEl.parentNode) document.body.removeChild(bubbleEl);
      view.destroy();
      _instances.delete(instanceId);
      container._zpMounted = false;
    },
  };

  _instances.set(instanceId, instance);

  /* Fire ready event so behavior files can access the instance immediately. */
  container.dispatchEvent(new CustomEvent("zeb:prose:ready", {
    bubbles: true,
    detail: { instance, id: instanceId },
  }));
}

/* ── Instance registry ────────────────────────────────────────────────────── */

/** Map of instanceId (container id) → ProseInstance. */
const _instances = new Map();

/* ── MutationObserver — auto-mount / auto-destroy ─────────────────────────── */

/**
 * Auto-mount any [data-zeb-lib="prosemirror"] element added to the DOM at any
 * time — including elements added by Preact re-renders after API fetches.
 * Also auto-destroys instances when their containers are removed from the DOM.
 *
 * This is the core mechanism that makes the SPA / API-first pattern work:
 *   1. Page loads, renders empty shell.
 *   2. useEffect fetches data, calls setSubmissions(data).
 *   3. Preact re-renders, adds ProseEditor divs to DOM.
 *   4. MutationObserver fires → mountEditor() is called for each new div.
 *   5. Each editor reads data-config, resolves initial content, and mounts.
 */
function destroyEditor(node) {
  if (node.nodeType !== 1) return;
  if (node._zpMounted && node.id) {
    _instances.get(node.id)?.destroy();
  }
  node.querySelectorAll?.("[data-zeb-lib='prosemirror']").forEach((el) => {
    if (el._zpMounted && el.id) _instances.get(el.id)?.destroy();
  });
}

const _observer = new MutationObserver((mutations) => {
  for (const mut of mutations) {
    for (const node of mut.addedNodes) {
      if (node.nodeType !== 1) continue;
      if (node.matches?.("[data-zeb-lib='prosemirror']"))    mountEditor(node);
      node.querySelectorAll?.("[data-zeb-lib='prosemirror']").forEach(mountEditor);
    }
    for (const node of mut.removedNodes) {
      destroyEditor(node);
    }
  }
});

/* Start observing immediately.  childList + subtree catches all DOM changes. */
if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => {
      _observer.observe(document.body, { childList: true, subtree: true });
      /* Also scan elements already in the DOM at script-load time. */
      document.querySelectorAll("[data-zeb-lib='prosemirror']").forEach(mountEditor);
    });
  } else {
    _observer.observe(document.body, { childList: true, subtree: true });
    document.querySelectorAll("[data-zeb-lib='prosemirror']").forEach(mountEditor);
  }
}

/* ── Public surface ───────────────────────────────────────────────────────── */

/**
 * window.__zebProse — global registry exposed for behavior files and pages.
 *
 * @example
 *   // Imperative access from a behavior file:
 *   const inst = window.__zebProse.get("answer-q1");
 *   const html = inst.getHTML();
 *
 *   // Custom plugin:
 *   window.__zebProse.registerPlugin({
 *     id: "ai-rewrite",
 *     label: "AI",
 *     icon: "<svg>…</svg>",
 *     async onActivate(ctx) {
 *       ctx.setLoading(true);
 *       const improved = await fetch("/api/ai", {
 *         method: "POST",
 *         body: JSON.stringify({ text: ctx.getSelectedText() })
 *       }).then(r => r.json()).then(d => d.result);
 *       ctx.replaceSelection(improved);
 *       ctx.setLoading(false);
 *     }
 *   });
 */
if (typeof window !== "undefined") {
  window.__zebProse = {
    /**
     * Get a ProseInstance by its container element id.
     * Returns undefined if the editor hasn't mounted yet.
     */
    get(id) { return _instances.get(id); },

    /**
     * Register a custom plugin.  It becomes available as a toolbar item
     * immediately — any editor already on the page will pick it up on the
     * next toolbar rebuild (page reload or editor re-mount).
     *
     * @param {{ id: string, label: string, icon: string, onActivate: function }} plugin
     */
    registerPlugin(plugin) {
      if (!plugin?.id) throw new Error("zeb/prosemirror: plugin must have an id");
      _plugins.set(plugin.id, plugin);
    },

    /** Remove a previously registered plugin by id. */
    unregisterPlugin(id) { _plugins.delete(id); },
  };
}

/* ── Legacy / direct-call export ─────────────────────────────────────────── */

/**
 * mountProseEditor — direct imperative mount.
 * Useful when you manage the lifecycle yourself (no MutationObserver).
 *
 * @param {HTMLElement} container - DOM element to mount into.
 * @param {object}      config    - Same keys as data-config JSON.
 * @returns {Promise<ProseInstance>}
 *
 * @example
 *   const inst = await mountProseEditor(document.getElementById("editor"), {
 *     stateKey: "body", toolbar: "full", editable: true,
 *   });
 */
export async function mountProseEditor(container, config = {}) {
  /* Merge config into data-config attribute so mountEditor() picks it up. */
  container.dataset.config = JSON.stringify(config);
  container.dataset.zebLib = "prosemirror";
  await mountEditor(container);
  return _instances.get(container.id);
}

export const prosemirror = { mountProseEditor };

/**
 * ProseEditor — Preact component exported from the bundle.
 *
 * Used in TSX page templates via:
 *   import ProseEditor from "zeb/prosemirror";
 *
 * The `import` line is stripped by the RWE compiler at build time.
 * `Object.assign(globalThis, bundle)` in the client preamble assigns this
 * function to globalThis.ProseEditor, so the bare reference in the compiled
 * page code resolves correctly at runtime.
 *
 * Uses useRef + useEffect to avoid Preact hydration conflicts:
 *   - The component renders a display:contents wrapper (layout-invisible).
 *   - useEffect fires AFTER Preact hydration → appends the real PM sentinel
 *     div imperatively. The MutationObserver catches it → mountEditor().
 *   - Preact's VNode for the wrapper has no children, so diffChildren never
 *     touches the PM-managed inner div on subsequent re-renders.
 *
 * Props mirror the data-config schema — see mountEditor() above.
 *
 * @param {{ id?, content?, stateKey?, statsKey?, editable?, autofocus?,
 *           placeholder?, toolbar?, toolbarMode?, className? }} props
 */
export function ProseEditor(props) {
  const _h         = globalThis.h;
  const _useRef    = globalThis.useRef;
  const _useEffect = globalThis.useEffect;

  if (!_h) return null;

  const config = {
    content:     props.content,
    stateKey:    props.stateKey,
    statsKey:    props.statsKey,
    editable:    props.editable !== false,
    autofocus:   props.autofocus ?? false,
    placeholder: props.placeholder,
    toolbar:     props.toolbar !== undefined ? props.toolbar : "basic",
    toolbarMode: props.toolbarMode ?? "inline",
  };

  /* ── Client path: hooks available → useRef + useEffect avoids Preact conflict ──
   *
   * Problem: ProseEditor returns a div with no VNode children. Preact's hydration
   * walk treats any DOM children not matching the VNode as "excess" and removes
   * them. PM adds children (toolbar + editorWrap) to the div after hydration via
   * MutationObserver. A state change then triggers a Preact re-render which calls
   * diffChildren, and Preact removes the PM children.
   *
   * Fix: render a display:contents wrapper (Preact-managed, no VNode children).
   * useEffect fires AFTER hydration completes → creates the inner sentinel div
   * and appends it imperatively. Preact never sees the inner div in its vdom and
   * therefore never removes it. On unmount, inner.remove() triggers the
   * MutationObserver → destroyEditor() cleans up the PM instance.
   */
  if (_useRef && _useEffect) {
    const wrapRef = _useRef(null);

    _useEffect(() => {
      const wrap = wrapRef.current;
      if (!wrap) return;

      const inner = document.createElement("div");
      inner.setAttribute("data-zeb-lib", "prosemirror");
      inner.setAttribute("data-config", JSON.stringify(config));
      if (props.id) inner.id = props.id;
      inner.className = props.className || "w-full min-h-[200px]";
      wrap.appendChild(inner);
      // MutationObserver fires here → mountEditor(inner)

      return () => { inner.remove(); }; // MutationObserver → destroyEditor on removal
    }, []); // mount / unmount once

    /* display:contents makes the wrapper invisible to the layout box model —
     * the inner PM div behaves as a direct child of the parent element. */
    return _h("div", {
      ref:                wrapRef,
      "data-zeb-wrapper": "ProseEditor",
      style:              { display: "contents" },
    });
  }

  /* ── SSR fallback (hooks not available): return sentinel div directly ── */
  return _h("div", {
    "data-zeb-lib":     "prosemirror",
    "data-zeb-wrapper": "ProseEditor",
    "data-config":      JSON.stringify(config),
    id:                 props.id,
    class:              props.className || "w-full min-h-[200px]",
  });
}
