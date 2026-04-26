import {
  CompletionContext,
  EditorView,
  autocompletion,
  basicSetup,
  css,
  cssLanguage,
  javascript,
  javascriptLanguage,
  lintGutter,
  linter,
  oneDark,
  setDiagnostics,
  snippetCompletion,
} from "./codemirror.bundle.mjs";

let vimModule = null;
const vimState = new WeakMap();

const JS_LIKE_KINDS = new Set([
  "template",
  "tsx",
  "typescript",
  "ts",
  "javascript",
  "js",
  "jsx",
]);

const DEFAULT_IMPORT_SOURCES = [
  "zeb",
  "zeb/use",
  "zeb/deckgl",
  "zeb/markdown",
  "zeb/pdf",
  "zeb/d3",
  "zeb/icons",
  "zeb/codemirror",
  "zeb/graphui",
  "zeb/preact",
  "zeb/prosemirror",
  "zeb/threejs",
  "zeb/threejs-vrm",
];

const TOOL_NAMESPACE_OPTIONS = [
  {
    label: "time",
    type: "namespace",
    detail: "Tool.time",
    info: "Date/time helpers: format, diff, add, relativeTime, tz, Hijri conversion.",
  },
  {
    label: "arr",
    type: "namespace",
    detail: "Tool.arr",
    info: "Array/data shaping helpers: sortBy, filterBy, paginate, groupBy, sumBy, uniqueBy.",
  },
  {
    label: "stat",
    type: "namespace",
    detail: "Tool.stat",
    info: "Statistics helpers: mean, median, variance, percentile, correlation, linreg, histogram.",
  },
  {
    label: "geo",
    type: "namespace",
    detail: "Tool.geo",
    info: "Geospatial helpers: distance, bbox, center, pointInPolygon, centroid, nearestPoint.",
  },
];

const TOOL_MEMBER_OPTIONS = {
  time: [
    fnOption("format", "Tool.time.format(date, pattern, locale?)", "Format a date using Zebflow tokens."),
    fnOption("diff", "Tool.time.diff(a, b, unit?)", "Return difference between two dates."),
    fnOption("add", "Tool.time.add(date, amount, unit)", "Add time units to a date."),
    fnOption("subtract", "Tool.time.subtract(date, amount, unit)", "Subtract time units from a date."),
    fnOption("startOf", "Tool.time.startOf(date, unit)", "Snap to the start of day/week/month/year."),
    fnOption("endOf", "Tool.time.endOf(date, unit)", "Snap to the end of day/month/year."),
    fnOption("isBefore", "Tool.time.isBefore(a, b)", "True when a is before b."),
    fnOption("isAfter", "Tool.time.isAfter(a, b)", "True when a is after b."),
    fnOption("isSame", "Tool.time.isSame(a, b, unit?)", "Compare dates, optionally at a given unit."),
    fnOption("relativeTime", "Tool.time.relativeTime(date, locale?)", "Human-readable relative time string."),
    fnOption("tz", "Tool.time.tz(date, timezone)", "Convert a date to an IANA timezone."),
    fnOption("toHijri", "Tool.time.toHijri(date)", "Convert Gregorian date to Hijri parts."),
    fnOption("fromHijri", "Tool.time.fromHijri(day, month, year)", "Convert Hijri date to Date."),
    fnOption("locale", "Tool.time.locale(code)", "Set the default locale for Tool.time."),
  ],
  arr: [
    fnOption("sortBy", "Tool.arr.sortBy(data, key, dir?)", "Sort array items by property or selector."),
    fnOption("filterBy", "Tool.arr.filterBy(data, filters)", "Filter an array by object, text, or predicate."),
    fnOption("paginate", "Tool.arr.paginate(data, page, size)", "Return paginated items and totals."),
    fnOption("groupBy", "Tool.arr.groupBy(data, key)", "Group items by property or selector."),
    fnOption("flatGroupBy", "Tool.arr.flatGroupBy(data, key)", "Return grouped items as flat objects."),
    fnOption("sumBy", "Tool.arr.sumBy(data, key)", "Sum a numeric field across items."),
    fnOption("countBy", "Tool.arr.countBy(data, key)", "Count items by group."),
    fnOption("uniqueBy", "Tool.arr.uniqueBy(data, key)", "Keep only the first item for each key."),
  ],
  stat: [
    fnOption("mean", "Tool.stat.mean(values)", "Average of numeric values."),
    fnOption("median", "Tool.stat.median(values)", "Median of numeric values."),
    fnOption("variance", "Tool.stat.variance(values)", "Population variance."),
    fnOption("stddev", "Tool.stat.stddev(values)", "Standard deviation."),
    fnOption("percentile", "Tool.stat.percentile(values, p)", "Percentile at p (0-100)."),
    fnOption("zscore", "Tool.stat.zscore(values)", "Z-scores for each value."),
    fnOption("rateAbove", "Tool.stat.rateAbove(values, threshold)", "Percent of values above threshold."),
    fnOption("correlation", "Tool.stat.correlation(xs, ys)", "Pearson correlation."),
    fnOption("linreg", "Tool.stat.linreg(xs, ys)", "Linear regression slope/intercept/r2."),
    fnOption("histogram", "Tool.stat.histogram(values, bins)", "Histogram bins with counts."),
  ],
  geo: [
    fnOption("distance", "Tool.geo.distance(from, to)", "Distance in km between [lon, lat] points."),
    fnOption("bbox", "Tool.geo.bbox(features)", "Bounding box [minLon, minLat, maxLon, maxLat]."),
    fnOption("center", "Tool.geo.center(pointsOrFeatures)", "Center point of a bbox."),
    fnOption("pointInPolygon", "Tool.geo.pointInPolygon(point, polygon)", "True when point is inside Polygon or MultiPolygon."),
    fnOption("centroid", "Tool.geo.centroid(geometry)", "Centroid of Polygon or MultiPolygon."),
    fnOption("nearestPoint", "Tool.geo.nearestPoint(origin, points)", "Nearest point index and distance in km."),
  ],
};

function normalizeLanguageKind(kind) {
  return String(kind || "").trim().toLowerCase();
}

function resolveLanguageExtension(kind) {
  switch (normalizeLanguageKind(kind)) {
    case "template":
    case "tsx":
    case "typescript":
    case "ts":
      return javascript({ jsx: true, typescript: true });
    case "javascript":
    case "js":
    case "jsx":
      return javascript({ jsx: true, typescript: false });
    case "css":
    case "style":
      return css();
    default:
      return null;
  }
}

function buildEditorTheme(options = {}) {
  const root = {};
  const scroller = {};

  if (options.height) {
    root.height = options.height;
    scroller.height = "100%";
  }
  if (options.minHeight) {
    root.minHeight = options.minHeight;
  }
  if (options.maxHeight) {
    root.maxHeight = options.maxHeight;
    scroller.maxHeight = options.maxHeight;
  }
  if (options.scrollerOverflow) {
    scroller.overflow = options.scrollerOverflow;
  } else if (options.height || options.maxHeight) {
    scroller.overflow = "auto";
  }

  if (!Object.keys(root).length && !Object.keys(scroller).length) {
    return null;
  }

  const spec = {};
  if (Object.keys(root).length) {
    spec["&"] = root;
  }
  if (Object.keys(scroller).length) {
    spec[".cm-scroller"] = scroller;
  }
  return EditorView.theme(spec);
}

function createLightweightVimTheme() {
  return EditorView.theme({
    "&": {
      position: "relative",
    },
    '&[data-zf-vim-mode="normal"] .cm-cursor, &[data-zf-vim-mode="visual"] .cm-cursor, &[data-zf-vim-mode="visual-line"] .cm-cursor': {
      borderLeftWidth: "0 !important",
      width: "0.62ch",
      backgroundColor: "rgba(251, 146, 60, 0.45)",
    },
    '&[data-zf-vim-mode="normal"] .cm-content, &[data-zf-vim-mode="visual"] .cm-content, &[data-zf-vim-mode="visual-line"] .cm-content': {
      caretColor: "transparent",
    },
    "&[data-zf-vim-status]::after": {
      content: "attr(data-zf-vim-status)",
      position: "absolute",
      right: "10px",
      bottom: "8px",
      padding: "2px 8px",
      borderRadius: "999px",
      backgroundColor: "rgba(15, 23, 42, 0.92)",
      border: "1px solid rgba(251, 146, 60, 0.38)",
      color: "#fdba74",
      fontSize: "11px",
      fontWeight: "700",
      letterSpacing: "0.01em",
      pointerEvents: "none",
      zIndex: "30",
    },
  });
}

function fnOption(label, detail, info) {
  return {
    label,
    type: "function",
    detail,
    info,
  };
}

function resolveVimPreference(options = {}) {
  if (typeof options.vim === "boolean") {
    return options.vim;
  }
  if (typeof window !== "undefined") {
    if (window.__zf_editor_preferences) {
      return !!window.__zf_editor_preferences?.vim;
    }
    try {
      const raw = window.localStorage?.getItem?.("zf-editor-preferences");
      if (raw) {
        const parsed = JSON.parse(raw);
        window.__zf_editor_preferences = {
          vim: !!parsed?.vim,
        };
        return !!parsed?.vim;
      }
    } catch (_) {}
  }
  return false;
}

async function enableVimSupport() {
  return vimModule;
}

function getStudioClipboard() {
  if (typeof window === "undefined") {
    return null;
  }
  return window.__zf_clipboard || null;
}

async function writeStudioClipboardText(text, meta = {}) {
  const clipboard = getStudioClipboard();
  if (clipboard && typeof clipboard.writeText === "function") {
    return clipboard.writeText(text, meta);
  }
  if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (_) {}
  }
  return null;
}

async function readStudioClipboardText() {
  const clipboard = getStudioClipboard();
  if (clipboard && typeof clipboard.readText === "function") {
    return clipboard.readText();
  }
  if (typeof navigator !== "undefined" && navigator.clipboard?.readText) {
    try {
      return await navigator.clipboard.readText();
    } catch (_) {}
  }
  return "";
}

function createEmptySelection(pos) {
  return { anchor: pos, head: pos };
}

function getDocText(view) {
  return view.state.doc.toString();
}

function cloneSelection(view) {
  const main = view.state.selection.main;
  return { anchor: main.anchor, head: main.head };
}

function setSelection(view, anchor, head = anchor) {
  view.dispatch({ selection: { anchor, head } });
}

function ensureVimState(view) {
  let state = vimState.get(view);
  if (state) return state;
  state = {
    mode: "normal",
    pending: "",
    count: "",
    visualAnchor: null,
    visualLine: false,
    register: { text: "", linewise: false },
    search: null,
    command: null,
    undo: [],
    redo: [],
    internalUndo: false,
    preferredColumn: null,
  };
  vimState.set(view, state);
  view.dom.setAttribute("data-zf-vim-mode", "normal");
  return state;
}

function setVimMode(view, state, mode) {
  state.mode = mode;
  state.pending = "";
  state.count = "";
  if (mode === "normal") {
    state.visualAnchor = null;
    state.visualLine = false;
    state.preferredColumn = null;
  }
  view.dom.setAttribute("data-zf-vim-mode", mode);
}

function setVimStatus(view, text = "") {
  if (!text) {
    view.dom.removeAttribute("data-zf-vim-status");
    return;
  }
  view.dom.setAttribute("data-zf-vim-status", text);
}

function getLineInfo(state, pos) {
  const line = state.doc.lineAt(pos);
  const text = line.text || "";
  const firstNonSpaceOffset = text.search(/\S/);
  return {
    line,
    lineStart: line.from,
    lineEnd: line.to,
    firstNonSpace: line.from + (firstNonSpaceOffset >= 0 ? firstNonSpaceOffset : 0),
  };
}

function clampPos(view, pos) {
  return Math.max(0, Math.min(view.state.doc.length, pos));
}

function moveToLine(view, pos, delta) {
  const state = view.state;
  const current = state.doc.lineAt(pos);
  const currentColumn = pos - current.from;
  const nextNumber = Math.max(1, Math.min(state.doc.lines, current.number + delta));
  const next = state.doc.line(nextNumber);
  return next.from + Math.min(currentColumn, next.length);
}

function isWordChar(ch) {
  return /[A-Za-z0-9_]/.test(ch || "");
}

function moveWordForward(view, pos) {
  const text = getDocText(view);
  let i = clampPos(view, pos);
  if (i >= text.length) return text.length;
  const startChar = text[i];
  if (isWordChar(startChar)) {
    while (i < text.length && isWordChar(text[i])) i++;
  }
  while (i < text.length && !isWordChar(text[i])) i++;
  return i;
}

function moveWordBackward(view, pos) {
  const text = getDocText(view);
  let i = clampPos(view, pos);
  if (i <= 0) return 0;
  i--;
  while (i > 0 && !isWordChar(text[i])) i--;
  while (i > 0 && isWordChar(text[i - 1])) i--;
  return i;
}

function moveWordEnd(view, pos) {
  const text = getDocText(view);
  let i = clampPos(view, pos);
  if (i >= text.length) return text.length;
  if (!isWordChar(text[i])) {
    while (i < text.length && !isWordChar(text[i])) i++;
  }
  while (i < text.length && isWordChar(text[i])) i++;
  return Math.max(0, i - 1);
}

function lineStartForNumber(view, lineNumber) {
  const safe = Math.max(1, Math.min(view.state.doc.lines, lineNumber));
  return view.state.doc.line(safe).from;
}

function consumeCount(state) {
  const value = Number.parseInt(state.count || "", 10);
  state.count = "";
  return Number.isFinite(value) && value > 0 ? value : 1;
}

function applyMotionCount(view, key, count) {
  let pos = view.state.selection.main.head;
  for (let index = 0; index < count; index += 1) {
    const next = resolveMotion(view, key);
    if (next == null || next === pos) {
      return pos;
    }
    pos = next;
    setSelection(view, pos);
  }
  return pos;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function findCurrentWordBounds(view) {
  const text = getDocText(view);
  const pos = clampPos(view, view.state.selection.main.head);
  if (!text.length) {
    return null;
  }
  let start = pos;
  let end = pos;
  if (!isWordChar(text[start])) {
    if (start > 0 && isWordChar(text[start - 1])) {
      start -= 1;
      end = start;
    } else {
      return null;
    }
  }
  while (start > 0 && isWordChar(text[start - 1])) start -= 1;
  while (end < text.length && isWordChar(text[end])) end += 1;
  if (start === end) {
    return null;
  }
  return { from: start, to: end, word: text.slice(start, end) };
}

function findSearchMatch(text, query, start, backward = false, wholeWord = false) {
  if (!query) {
    return null;
  }
  if (!wholeWord) {
    if (backward) {
      const before = text.lastIndexOf(query, Math.max(0, start));
      if (before >= 0) {
        return { from: before, to: before + query.length };
      }
      const wrapped = text.lastIndexOf(query);
      return wrapped >= 0 ? { from: wrapped, to: wrapped + query.length } : null;
    }
    const forward = text.indexOf(query, Math.max(0, start));
    if (forward >= 0) {
      return { from: forward, to: forward + query.length };
    }
    const wrapped = text.indexOf(query);
    return wrapped >= 0 ? { from: wrapped, to: wrapped + query.length } : null;
  }

  const pattern = new RegExp(`\\b${escapeRegExp(query)}\\b`, "g");
  const matches = [];
  let match;
  while ((match = pattern.exec(text))) {
    matches.push({ from: match.index, to: match.index + match[0].length });
    if (match.index === pattern.lastIndex) {
      pattern.lastIndex += 1;
    }
  }
  if (!matches.length) {
    return null;
  }
  if (backward) {
    for (let index = matches.length - 1; index >= 0; index -= 1) {
      if (matches[index].from <= start) {
        return matches[index];
      }
    }
    return matches[matches.length - 1];
  }
  for (const found of matches) {
    if (found.from >= start) {
      return found;
    }
  }
  return matches[0];
}

function applySearchMatch(view, found) {
  if (!found) {
    return false;
  }
  view.dispatch({
    selection: { anchor: found.from, head: found.to },
    scrollIntoView: true,
  });
  return true;
}

function runSearch(view, state, search, reverseOverride = null) {
  if (!search?.query) {
    return false;
  }
  const text = getDocText(view);
  const selection = view.state.selection.main;
  const backward = reverseOverride == null ? !!search.backward : !!reverseOverride;
  const start = backward ? Math.max(0, selection.from - 1) : Math.min(text.length, selection.to + 1);
  const found = findSearchMatch(text, search.query, start, backward, !!search.wholeWord);
  if (!found) {
    return false;
  }
  state.search = { ...search, backward };
  return applySearchMatch(view, found);
}

function beginVimCommand(view, state, prefix, kind) {
  state.command = { prefix, kind, value: "" };
  state.pending = "";
  state.count = "";
  setVimStatus(view, prefix);
}

function finishVimCommand(view, state) {
  state.command = null;
  setVimStatus(view, "");
}

function executeVimCommand(view, state, options = {}) {
  const command = state.command;
  finishVimCommand(view, state);
  if (!command) {
    return false;
  }
  const value = command.value.trim();
  if (command.kind === "command") {
    if ((value === "w" || value === "write") && typeof options.onSave === "function") {
      options.onSave();
      return true;
    }
    return true;
  }
  if (!value) {
    return true;
  }
  const search = {
    query: value,
    backward: command.prefix === "?",
    wholeWord: false,
  };
  return runSearch(view, state, search, search.backward);
}

function handleVimCommandKey(view, state, event, options = {}) {
  const command = state.command;
  if (!command) {
    return false;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    finishVimCommand(view, state);
    return true;
  }
  if (event.key === "Backspace") {
    event.preventDefault();
    command.value = command.value.slice(0, -1);
    setVimStatus(view, `${command.prefix}${command.value}`);
    return true;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    return executeVimCommand(view, state, options);
  }
  if (typeof event.key === "string" && event.key.length === 1 && !event.metaKey && !event.ctrlKey && !event.altKey) {
    event.preventDefault();
    command.value += event.key;
    setVimStatus(view, `${command.prefix}${command.value}`);
    return true;
  }
  return false;
}

function resolveMotion(view, key) {
  const selection = view.state.selection.main;
  const pos = selection.head;
  switch (key) {
    case "h":
      return Math.max(0, pos - 1);
    case "l":
      return Math.min(view.state.doc.length, pos + 1);
    case "j":
      return moveToLine(view, pos, 1);
    case "k":
      return moveToLine(view, pos, -1);
    case "w":
      return moveWordForward(view, pos);
    case "b":
      return moveWordBackward(view, pos);
    case "e":
      return moveWordEnd(view, pos);
    case "0":
      return getLineInfo(view.state, pos).lineStart;
    case "^":
      return getLineInfo(view.state, pos).firstNonSpace;
    case "$":
      return getLineInfo(view.state, pos).lineEnd;
    case "g":
      return 0;
    case "G":
      return view.state.doc.length;
    default:
      return null;
  }
}

function applyVisualSelection(view, state, head) {
  if (state.visualAnchor == null) {
    state.visualAnchor = view.state.selection.main.anchor;
  }
  if (state.visualLine) {
    const anchorLine = view.state.doc.lineAt(state.visualAnchor);
    const headLine = view.state.doc.lineAt(head);
    const from = Math.min(anchorLine.from, headLine.from);
    const toLine = anchorLine.number >= headLine.number ? anchorLine : headLine;
    const to = toLine.to;
    setSelection(view, from, to);
    return;
  }
  setSelection(view, state.visualAnchor, head);
}

function enterNormalMode(view, state) {
  const selection = view.state.selection.main;
  let head = selection.head;
  if (selection.empty && head > 0) {
    head -= 1;
  }
  setSelection(view, head);
  setVimMode(view, state, "normal");
  setVimStatus(view, "");
}

function recordUndoSnapshot(view, state, previousText, previousSelection) {
  if (state.internalUndo) return;
  state.undo.push({
    text: previousText,
    selection: previousSelection,
  });
  if (state.undo.length > 200) {
    state.undo.shift();
  }
  state.redo = [];
}

function restoreSnapshot(view, state, snapshot, targetStack) {
  if (!snapshot) return false;
  const current = {
    text: getDocText(view),
    selection: cloneSelection(view),
  };
  targetStack.push(current);
  state.internalUndo = true;
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: snapshot.text },
    selection: snapshot.selection,
  });
  state.internalUndo = false;
  return true;
}

function sliceRange(view, from, to) {
  return view.state.sliceDoc(from, to);
}

function linewiseTextForLine(view, line) {
  const next = line.number < view.state.doc.lines ? view.state.doc.line(line.number + 1).from : line.to;
  return view.state.sliceDoc(line.from, next);
}

function setRegister(view, state, text, linewise = false) {
  state.register = { text, linewise };
  void writeStudioClipboardText(text, {
    kind: "text/plain",
    source: "vim-yank",
    data: { linewise },
  });
}

async function pasteRegister(view, state, before = false) {
  let text = state.register?.text || "";
  let linewise = !!state.register?.linewise;
  if (!text) {
    text = await readStudioClipboardText();
    linewise = false;
  }
  if (!text) return;

  const selection = view.state.selection.main;
  if (linewise) {
    const line = view.state.doc.lineAt(selection.head);
    const insertPos = before ? line.from : (line.number < view.state.doc.lines ? view.state.doc.line(line.number + 1).from : line.to);
    view.dispatch({
      changes: { from: insertPos, to: insertPos, insert: text },
      selection: createEmptySelection(insertPos),
      userEvent: "input.paste",
    });
    return;
  }

  const insertPos = before ? selection.from : selection.to;
  view.dispatch({
    changes: { from: insertPos, to: insertPos, insert: text },
    selection: createEmptySelection(insertPos + text.length),
    userEvent: "input.paste",
  });
}

function applyRangeOperation(view, state, op, from, to, motionKey = "") {
  const start = Math.max(0, Math.min(from, to));
  const end = Math.max(start, Math.max(from, to));
  if (start === end && op !== "c") return true;

  if (op === "y") {
    setRegister(view, state, sliceRange(view, start, end), false);
    setSelection(view, start);
    setVimMode(view, state, "normal");
    return true;
  }

  const removed = sliceRange(view, start, end);
  setRegister(view, state, removed, false);
  view.dispatch({
    changes: { from: start, to: end, insert: "" },
    selection: createEmptySelection(start),
    userEvent: op === "c" ? "input.change" : "delete",
  });

  if (op === "c") {
    state.visualAnchor = null;
    state.visualLine = false;
    setVimMode(view, state, "insert");
  } else {
    setVimMode(view, state, "normal");
  }
  return true;
}

function runLinewiseOperation(view, state, op) {
  const line = view.state.doc.lineAt(view.state.selection.main.head);
  const text = linewiseTextForLine(view, line);
  const from = line.from;
  const to = from + text.length;

  if (op === "y") {
    setRegister(view, state, text, true);
    setSelection(view, from);
    setVimMode(view, state, "normal");
    return true;
  }

  setRegister(view, state, text, true);
  view.dispatch({
    changes: { from, to, insert: "" },
    selection: createEmptySelection(from),
    userEvent: op === "c" ? "input.change" : "delete",
  });
  if (op === "c") {
    setVimMode(view, state, "insert");
  } else {
    setVimMode(view, state, "normal");
  }
  return true;
}

function isPlainPrintableKey(event) {
  if (event.metaKey || event.ctrlKey || event.altKey) {
    return false;
  }
  return typeof event.key === "string" && event.key.length === 1;
}

function handleNormalKey(view, state, event, options = {}) {
  const key = event.key;
  if (state.command) {
    return handleVimCommandKey(view, state, event, options);
  }
  if (event.metaKey || event.altKey) {
    return false;
  }
  if (event.ctrlKey && key.toLowerCase() !== "r") {
    return false;
  }

  if (key === "Control" || key === "Shift" || key === "Alt" || key === "Meta") {
    return false;
  }

  if (event.ctrlKey && key.toLowerCase() === "r") {
    event.preventDefault();
    return restoreSnapshot(view, state, state.redo.pop(), state.undo);
  }

  if (state.pending === "g") {
    state.pending = "";
    if (key === "g") {
      event.preventDefault();
      const count = consumeCount(state);
      setSelection(view, count > 1 ? lineStartForNumber(view, count) : 0);
      return true;
    }
    if (key === "d") {
      event.preventDefault();
      document.execCommand?.("copy");
      return true;
    }
    if (isPlainPrintableKey(event)) {
      event.preventDefault();
      return true;
    }
    return false;
  }

  if (state.pending === "d" || state.pending === "c" || state.pending === "y") {
    const op = state.pending;
    state.pending = "";
    if (key === op) {
      event.preventDefault();
      return runLinewiseOperation(view, state, op);
    }
    const target = resolveMotion(view, key);
    if (target == null) {
      if (isPlainPrintableKey(event)) {
        event.preventDefault();
        return true;
      }
      return false;
    }
    event.preventDefault();
    const head = view.state.selection.main.head;
    const endExclusive = key === "$" || key === "e" ? target + 1 : target;
    return applyRangeOperation(view, state, op, head, endExclusive, key);
  }

  switch (key) {
    case ":":
      event.preventDefault();
      beginVimCommand(view, state, ":", "command");
      return true;
    case "/":
      event.preventDefault();
      beginVimCommand(view, state, "/", "search");
      return true;
    case "?":
      event.preventDefault();
      beginVimCommand(view, state, "?", "search");
      return true;
    case "#": {
      event.preventDefault();
      const found = findCurrentWordBounds(view);
      if (!found?.word) {
        return true;
      }
      state.search = {
        query: found.word,
        backward: true,
        wholeWord: true,
      };
      return runSearch(view, state, state.search, true);
    }
    case "n":
      event.preventDefault();
      return runSearch(view, state, state.search, state.search?.backward);
    case "N":
      event.preventDefault();
      return runSearch(view, state, state.search, !state.search?.backward);
    case "i":
    case "a":
    case "o":
    case "O": {
      event.preventDefault();
      const line = view.state.doc.lineAt(view.state.selection.main.head);
      if (key === "a") {
        setSelection(view, Math.min(view.state.doc.length, view.state.selection.main.head + 1));
      } else if (key === "o") {
        const insertPos = line.number < view.state.doc.lines ? view.state.doc.line(line.number + 1).from : line.to;
        view.dispatch({
          changes: { from: insertPos, to: insertPos, insert: "\n" },
          selection: createEmptySelection(insertPos + 1),
          userEvent: "input",
        });
      } else if (key === "O") {
        const insertPos = line.from;
        view.dispatch({
          changes: { from: insertPos, to: insertPos, insert: "\n" },
          selection: createEmptySelection(insertPos),
          userEvent: "input",
        });
      }
      setVimMode(view, state, "insert");
      return true;
    }
    case "v":
      event.preventDefault();
      state.visualAnchor = view.state.selection.main.head;
      state.visualLine = false;
      setVimMode(view, state, "visual");
      return true;
    case "V":
      event.preventDefault();
      state.visualAnchor = view.state.selection.main.head;
      state.visualLine = true;
      setVimMode(view, state, "visual-line");
      applyVisualSelection(view, state, view.state.selection.main.head);
      return true;
    case "d":
    case "c":
    case "y":
      event.preventDefault();
      state.pending = key;
      return true;
    case "p":
      event.preventDefault();
      void pasteRegister(view, state, false);
      return true;
    case "P":
      event.preventDefault();
      void pasteRegister(view, state, true);
      return true;
    case "x": {
      event.preventDefault();
      const pos = view.state.selection.main.head;
      if (pos >= view.state.doc.length) return true;
      return applyRangeOperation(view, state, "d", pos, pos + 1);
    }
    case "u":
      event.preventDefault();
      return restoreSnapshot(view, state, state.undo.pop(), state.redo);
    case "g":
      event.preventDefault();
      state.pending = "g";
      return true;
    case "G":
      event.preventDefault();
      {
        const count = consumeCount(state);
        setSelection(view, count > 1 ? lineStartForNumber(view, count) : view.state.doc.length);
      }
      return true;
    default: {
      if (!event.metaKey && !event.ctrlKey && !event.altKey && /^[1-9]$/.test(key)) {
        event.preventDefault();
        state.count += key;
        setVimStatus(view, state.count);
        return true;
      }
      if (!event.metaKey && !event.ctrlKey && !event.altKey && key === "0" && state.count) {
        event.preventDefault();
        state.count += key;
        setVimStatus(view, state.count);
        return true;
      }
      const count = consumeCount(state);
      if (!state.command) {
        setVimStatus(view, "");
      }
      const target = resolveMotion(view, key);
      if (target == null) {
        if (isPlainPrintableKey(event)) {
          event.preventDefault();
          return true;
        }
        return false;
      }
      event.preventDefault();
      let finalTarget = target;
      if (count > 1) {
        finalTarget = applyMotionCount(view, key, count);
      }
      setSelection(view, finalTarget);
      return true;
    }
  }
}

function handleVisualKey(view, state, event) {
  const key = event.key;
  if (key === "Escape") {
    event.preventDefault();
    setSelection(view, view.state.selection.main.head);
    setVimMode(view, state, "normal");
    return true;
  }
  if (key === "y") {
    event.preventDefault();
    const selection = view.state.selection.main;
    if (state.visualLine) {
      const fromLine = view.state.doc.lineAt(Math.min(selection.from, selection.to));
      const toLine = view.state.doc.lineAt(Math.max(selection.from, selection.to));
      const text = view.state.sliceDoc(fromLine.from, toLine.to);
      setRegister(view, state, text, true);
      setSelection(view, fromLine.from);
    } else {
      setRegister(view, state, sliceRange(view, selection.from, selection.to), false);
      setSelection(view, selection.from);
    }
    setVimMode(view, state, "normal");
    return true;
  }
  if (key === "d" || key === "c") {
    event.preventDefault();
    const selection = view.state.selection.main;
    const result = applyRangeOperation(view, state, key, selection.from, selection.to);
    return result;
  }
  const target = resolveMotion(view, key);
  if (target == null) {
    if (isPlainPrintableKey(event)) {
      event.preventDefault();
      return true;
    }
    return false;
  }
  event.preventDefault();
  applyVisualSelection(view, state, target);
  return true;
}

function createLightweightVimExtensions(options = {}) {
  return [
    createLightweightVimTheme(),
    EditorView.updateListener.of((update) => {
      const state = ensureVimState(update.view);
      if (update.docChanged) {
        const previousText = update.startState.doc.toString();
        const previousSelection = {
          anchor: update.startState.selection.main.anchor,
          head: update.startState.selection.main.head,
        };
        recordUndoSnapshot(update.view, state, previousText, previousSelection);
      }
    }),
    EditorView.domEventHandlers({
      keydown(event, view) {
        const state = ensureVimState(view);
        if (state.mode === "insert") {
          if (event.key === "Escape") {
            event.preventDefault();
            enterNormalMode(view, state);
            return true;
          }
          return false;
        }
        if (state.mode === "visual" || state.mode === "visual-line") {
          return handleVisualKey(view, state, event);
        }
        return handleNormalKey(view, state, event, options);
      },
    }),
    EditorView.inputHandler.of((view, _from, _to, _text, _insert) => {
      const state = ensureVimState(view);
      if (state.mode === "insert") {
        return false;
      }
      return true;
    }),
  ];
}

function replaceSelectionText(view, text) {
  const selection = view.state.selection.main;
  view.dispatch({
    changes: { from: selection.from, to: selection.to, insert: text },
    selection: { anchor: selection.from + text.length },
    userEvent: "input.paste",
  });
}

function createClipboardExtensions(options = {}) {
  return [
    EditorView.domEventHandlers({
      copy(event, view) {
        const selection = view.state.selection.main;
        if (selection.empty) {
          return false;
        }
        const text = view.state.sliceDoc(selection.from, selection.to);
        if (event.clipboardData) {
          event.preventDefault();
          event.clipboardData.setData("text/plain", text);
        }
        void writeStudioClipboardText(text, {
          kind: "text/plain",
          source: options.clipboardSource || "codemirror",
        });
        return !!event.clipboardData;
      },
      cut(event, view) {
        if (options.readonly) {
          return false;
        }
        const selection = view.state.selection.main;
        if (selection.empty) {
          return false;
        }
        const text = view.state.sliceDoc(selection.from, selection.to);
        if (event.clipboardData) {
          event.preventDefault();
          event.clipboardData.setData("text/plain", text);
        }
        void writeStudioClipboardText(text, {
          kind: "text/plain",
          source: options.clipboardSource || "codemirror",
        });
        view.dispatch({
          changes: { from: selection.from, to: selection.to, insert: "" },
          selection: { anchor: selection.from },
          userEvent: "delete.cut",
        });
        return true;
      },
      paste(event, view) {
        const text = event.clipboardData?.getData("text/plain");
        if (typeof text !== "string" || text.length === 0) {
          return false;
        }
        event.preventDefault();
        replaceSelectionText(view, text);
        void writeStudioClipboardText(text, {
          kind: "text/plain",
          source: "system-paste",
        });
        return true;
      },
    }),
  ];
}

function isJavaScriptLikeKind(kind) {
  return JS_LIKE_KINDS.has(normalizeLanguageKind(kind));
}

function normalizeProjectImportSpecifier(relPath) {
  if (typeof relPath !== "string") {
    return null;
  }

  let normalized = relPath.trim().replace(/^\/+/, "");
  if (!normalized || normalized.startsWith("docs/")) {
    return null;
  }
  if (normalized.endsWith(".zf.json")) {
    return null;
  }

  if (/\.(tsx|ts|jsx|js|mjs)$/i.test(normalized)) {
    normalized = normalized.replace(/\.(tsx|ts|jsx|js|mjs)$/i, "");
  } else if (!/\.(css|json|svg|png|jpg|jpeg|gif|webp|avif)$/i.test(normalized)) {
    return null;
  }
  return `@/${normalized}`;
}

function uniqueSorted(values) {
  return Array.from(new Set(values.filter(Boolean))).sort((a, b) => a.localeCompare(b));
}

function collectImportSpecifiers(options = {}) {
  const specifiers = [...DEFAULT_IMPORT_SOURCES];
  const projectFiles = Array.isArray(options.projectFiles) ? options.projectFiles : [];
  const importSources = Array.isArray(options.importSources) ? options.importSources : [];

  for (const relPath of projectFiles) {
    specifiers.push(normalizeProjectImportSpecifier(relPath));
  }
  for (const source of importSources) {
    if (typeof source === "string" && source.trim()) {
      specifiers.push(source.trim());
    }
  }

  return uniqueSorted(specifiers);
}

function createProjectImportIndex(projectFiles) {
  const exact = new Map();
  const normalized = uniqueSorted(Array.isArray(projectFiles) ? projectFiles : [])
    .map((value) => String(value || "").replace(/^\/+/, ""))
    .filter(Boolean);

  for (const relPath of normalized) {
    exact.set(relPath, relPath);
  }

  return { exact, files: normalized };
}

function resolveProjectImportPath(specifier, projectImportIndex) {
  if (!specifier || !specifier.startsWith("@/") || !projectImportIndex) {
    return null;
  }

  const base = specifier.slice(2).replace(/^\/+/, "");
  if (!base) {
    return null;
  }

  const candidates = [
    base,
    `${base}.tsx`,
    `${base}.ts`,
    `${base}.jsx`,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.css`,
    `${base}.json`,
    `${base}.svg`,
    `${base}.png`,
    `${base}.jpg`,
    `${base}.jpeg`,
    `${base}.gif`,
    `${base}.webp`,
    `${base}.avif`,
    `${base}/index.tsx`,
    `${base}/index.ts`,
    `${base}/index.jsx`,
    `${base}/index.js`,
    `${base}/index.mjs`,
  ];

  for (const candidate of candidates) {
    const found = projectImportIndex.exact.get(candidate);
    if (found) {
      return found;
    }
  }

  return null;
}

function resolveImportTarget(specifier, options = {}, projectImportIndex) {
  const projectPath = resolveProjectImportPath(specifier, projectImportIndex);
  if (projectPath) {
    return {
      kind: "project",
      specifier,
      relPath: projectPath,
    };
  }

  if (specifier === "zeb" || specifier.startsWith("zeb/")) {
    return {
      kind: "library",
      specifier,
    };
  }

  return null;
}

function createOutlineLoader(options = {}) {
  const cache = new Map();
  const endpoint =
    typeof options.templateOutlineUrl === "string" && options.templateOutlineUrl.trim()
      ? new URL(options.templateOutlineUrl, document.baseURI).href
      : null;

  if (!endpoint) {
    return null;
  }

  return async (relPath) => {
    const normalized = String(relPath || "").replace(/^\/+/, "");
    if (!normalized) {
      return null;
    }
    if (cache.has(normalized)) {
      return cache.get(normalized);
    }

    const promise = (async () => {
      try {
        const response = await fetch(`${endpoint}?path=${encodeURIComponent(normalized)}`, {
          headers: { Accept: "application/json" },
        });
        const payload = await response.json().catch(() => null);
        if (!response.ok) {
          return null;
        }
        return payload?.outline || null;
      } catch (_error) {
        return null;
      }
    })();

    cache.set(normalized, promise);
    return promise;
  };
}

function parseImportStatement(lineText) {
  if (typeof lineText !== "string" || !lineText.includes("from")) {
    return null;
  }

  const match = lineText.match(/^\s*(?:import|export)\s+(.+?)\s+from\s+(['"])([^'"]+)\2/);
  if (!match) {
    return null;
  }

  const clause = match[1];
  const specifier = match[3];
  const full = match[0];
  const start = match.index || 0;
  const clauseStart = start + full.indexOf(clause);
  const specStart = lineText.indexOf(specifier, clauseStart + clause.length);
  const specEnd = specStart + specifier.length;
  const refs = [];

  const braceOpen = clause.indexOf("{");
  const braceClose = braceOpen >= 0 ? clause.indexOf("}", braceOpen + 1) : -1;

  const namedFrom = braceOpen >= 0 && braceClose > braceOpen ? clauseStart + braceOpen + 1 : null;
  const namedTo = braceOpen >= 0 && braceClose > braceOpen ? clauseStart + braceClose : null;

  const prefix = braceOpen >= 0 ? clause.slice(0, braceOpen).trim().replace(/,$/, "").trim() : clause.trim();

  if (prefix) {
    const namespaceMatch = prefix.match(/\*\s+as\s+([A-Za-z_$][\w$]*)/);
    if (namespaceMatch) {
      const localName = namespaceMatch[1];
      const localIndex = clause.indexOf(localName);
      refs.push({
        mode: "namespace",
        localName,
        exportName: "*",
        from: clauseStart + localIndex,
        to: clauseStart + localIndex + localName.length,
      });
    } else {
      const defaultName = prefix.split(",")[0].trim();
      if (defaultName && /^[A-Za-z_$][\w$]*$/.test(defaultName)) {
        const defaultIndex = clause.indexOf(defaultName);
        refs.push({
          mode: "default",
          localName: defaultName,
          exportName: "default",
          from: clauseStart + defaultIndex,
          to: clauseStart + defaultIndex + defaultName.length,
        });
      }
    }
  }

  if (namedFrom != null && namedTo != null) {
    const namedClause = clause.slice(braceOpen + 1, braceClose);
    const matcher = /([A-Za-z_$][\w$]*)(\s+as\s+([A-Za-z_$][\w$]*))?/g;
    let item;
    while ((item = matcher.exec(namedClause))) {
      const exportName = item[1];
      const localName = item[3] || exportName;
      const localOffset = item[3] ? item[0].lastIndexOf(localName) : item.index;
      refs.push({
        mode: "named",
        localName,
        exportName,
        from: namedFrom + localOffset,
        to: namedFrom + localOffset + localName.length,
      });
    }
  }

  return {
    clause,
    clauseStart,
    clauseEnd: clauseStart + clause.length,
    specifier,
    specStart,
    specEnd,
    refs,
    namedFrom,
    namedTo,
  };
}

function extractImportSpecifierPrefix(before) {
  const patterns = [
    /\bfrom\s+["']([^"']*)$/,
    /\bimport\s*\(\s*["']([^"']*)$/,
    /\bimport\s+["']([^"']*)$/,
    /\bexport\s+[^"']*from\s+["']([^"']*)$/,
  ];

  for (const pattern of patterns) {
    const match = before.match(pattern);
    if (match) {
      return match[1] || "";
    }
  }

  return null;
}

function buildImportCompletionOptions(prefix, specifiers) {
  if (prefix == null) {
    return [];
  }

  if (prefix === "") {
    return [
      { label: "@/", type: "namespace", detail: "Project templates" },
      ...specifiers
        .filter((value) => value === "zeb" || value.startsWith("zeb/"))
        .map((value) => ({
          label: value,
          type: value.includes("/") ? "module" : "namespace",
          detail: "Zeb library",
        })),
    ];
  }

  const options = new Map();
  for (const specifier of specifiers) {
    if (!specifier.startsWith(prefix)) {
      continue;
    }
    if (specifier === prefix) {
      options.set(specifier, {
        label: specifier,
        type: "module",
        detail: specifier.startsWith("@/") ? "Project import" : "Library import",
      });
      continue;
    }

    const rest = specifier.slice(prefix.length);
    const boundary = rest.startsWith("/") ? "/" : "";
    const remainder = boundary ? rest.slice(1) : rest;
    if (!remainder) {
      continue;
    }

    const slashIndex = remainder.indexOf("/");
    if (slashIndex >= 0) {
      const folderLabel = prefix + boundary + remainder.slice(0, slashIndex + 1);
      if (!options.has(folderLabel)) {
        options.set(folderLabel, {
          label: folderLabel,
          type: "namespace",
          detail: "Import folder",
        });
      }
      continue;
    }

    options.set(specifier, {
      label: specifier,
      type: "module",
      detail: specifier.startsWith("@/") ? "Project import" : "Library import",
    });
  }

  return Array.from(options.values()).sort((a, b) => a.label.localeCompare(b.label));
}

function findImportReferenceAt(state, pos) {
  const line = state.doc.lineAt(pos);
  const parsed = parseImportStatement(line.text);
  if (!parsed) {
    return null;
  }

  const offset = pos - line.from;
  if (offset >= parsed.specStart && offset <= parsed.specEnd) {
    return {
      kind: "source",
      specifier: parsed.specifier,
      from: line.from + parsed.specStart,
      to: line.from + parsed.specEnd,
    };
  }

  for (const ref of parsed.refs) {
    if (offset >= ref.from && offset <= ref.to) {
      return {
        kind: "symbol",
        specifier: parsed.specifier,
        symbolName: ref.exportName,
        localName: ref.localName,
        isDefault: ref.mode === "default",
        from: line.from + ref.from,
        to: line.from + ref.to,
      };
    }
  }

  return null;
}

function findNamedImportContext(state, pos) {
  const line = state.doc.lineAt(pos);
  const parsed = parseImportStatement(line.text);
  if (!parsed || parsed.namedFrom == null || parsed.namedTo == null) {
    return null;
  }

  const offset = pos - line.from;
  if (offset < parsed.namedFrom || offset > parsed.namedTo) {
    return null;
  }

  const typedPrefix = line.text.slice(parsed.namedFrom, offset);
  const partialMatch = typedPrefix.match(/([A-Za-z_$][\w$]*)$/);
  const partial = partialMatch ? partialMatch[1] : "";
  const importedNames = parsed.refs
    .filter((ref) => ref.mode === "named")
    .map((ref) => ref.exportName);

  return {
    specifier: parsed.specifier,
    partial,
    from: pos - partial.length,
    importedNames,
  };
}

function createImportCompletionSource(options = {}) {
  const specifiers = collectImportSpecifiers(options);
  if (!specifiers.length) {
    return null;
  }

  return (context) => {
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);
    const prefix = extractImportSpecifierPrefix(before);
    if (prefix == null) {
      return null;
    }

    const options = buildImportCompletionOptions(prefix, specifiers);
    if (!options.length) {
      return null;
    }

    return {
      from: context.pos - prefix.length,
      options,
      validFor: /[@/\w.-]*/,
    };
  };
}

function completionTypeForSymbolKind(kind) {
  switch (kind) {
    case "Function":
    case "function":
    case "fn":
      return "function";
    case "Class":
    case "class":
      return "class";
    case "Type":
    case "Interface":
    case "type":
    case "interface":
      return "type";
    default:
      return "variable";
  }
}

function createImportSymbolCompletionSource(options = {}) {
  const projectImportIndex = createProjectImportIndex(options.projectFiles);
  const outlineLoader = createOutlineLoader(options);
  if (!outlineLoader) {
    return null;
  }

  return async (context) => {
    const importContext = findNamedImportContext(context.state, context.pos);
    if (!importContext) {
      return null;
    }

    const target = resolveImportTarget(importContext.specifier, options, projectImportIndex);
    if (!target || target.kind !== "project") {
      return null;
    }

    const outline = await outlineLoader(target.relPath);
    const symbols = Array.isArray(outline?.symbols) ? outline.symbols : [];
    const optionsList = symbols
      .filter((symbol) => symbol?.is_exported && !symbol?.is_default && symbol?.kind !== "Import")
      .filter((symbol) => !importContext.importedNames.includes(symbol.name) || symbol.name === importContext.partial)
      .filter((symbol) => !importContext.partial || String(symbol.name || "").startsWith(importContext.partial))
      .map((symbol) => ({
        label: symbol.name,
        type: completionTypeForSymbolKind(symbol.kind),
        detail: `${symbol.kind.toLowerCase()} · ${target.relPath}`,
        info: `line ${symbol.line} · ${target.relPath}`,
      }))
      .sort((a, b) => a.label.localeCompare(b.label));

    if (!optionsList.length) {
      return null;
    }

    return {
      from: importContext.from,
      options: optionsList,
      validFor: /[A-Za-z_$][\w$]*/,
    };
  };
}

function createToolCompletionSource() {
  return (context) => {
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);

    if (/(?:^|[^\w$])Tool\.$/.test(before)) {
      return {
        from: context.pos,
        options: TOOL_NAMESPACE_OPTIONS,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const namespaceMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)$/);
    if (namespaceMatch) {
      const partial = namespaceMatch[1];
      const options = TOOL_NAMESPACE_OPTIONS.filter((option) => option.label.startsWith(partial));
      if (!options.length) {
        return null;
      }
      return {
        from: context.pos - partial.length,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const memberDotMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)\.$/);
    if (memberDotMatch) {
      const namespace = memberDotMatch[1];
      const options = TOOL_MEMBER_OPTIONS[namespace];
      if (!options || !options.length) {
        return null;
      }
      return {
        from: context.pos,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const memberMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)\.([A-Za-z_$][\w$]*)$/);
    if (memberMatch) {
      const namespace = memberMatch[1];
      const partial = memberMatch[2];
      const options = (TOOL_MEMBER_OPTIONS[namespace] || []).filter((option) =>
        option.label.startsWith(partial)
      );
      if (!options.length) {
        return null;
      }
      return {
        from: context.pos - partial.length,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const word = context.matchBefore(/[A-Za-z_$][\w$]*/);
    if (!word || (word.from === word.to && !context.explicit)) {
      return null;
    }

    if (!"Tool".startsWith(word.text) && word.text !== "Tool") {
      return null;
    }

    return {
      from: word.from,
      options: [
        {
          label: "Tool",
          type: "namespace",
          detail: "Built-in Zebflow helper namespaces",
          info: "Available namespaces: Tool.time, Tool.arr, Tool.stat, Tool.geo",
        },
      ],
      validFor: /[A-Za-z_$][\w$]*/,
    };
  };
}

function createZebflowCompletionSource(options = {}) {
  const importSource = createImportCompletionSource(options);
  const importSymbolSource = createImportSymbolCompletionSource(options);
  const toolSource = createToolCompletionSource();

  return async (context) => {
    const importResult = importSource ? importSource(context) : null;
    if (importResult) {
      return importResult;
    }
    const importSymbolResult = importSymbolSource ? await importSymbolSource(context) : null;
    if (importSymbolResult) {
      return importSymbolResult;
    }
    return toolSource(context);
  };
}

function createImportNavigationExtensions(options = {}) {
  if (typeof options.onOpenImport !== "function") {
    return [];
  }

  const projectImportIndex = createProjectImportIndex(options.projectFiles);
  const outlineLoader = createOutlineLoader(options);

  function canOpenTarget(target) {
    if (!target) {
      return false;
    }
    if (target.kind === "project") {
      return true;
    }
    return target.kind === "library" && typeof options.onOpenLibraryImport === "function";
  }

  async function resolveOpenTarget(reference) {
    const target = resolveImportTarget(reference.specifier, options, projectImportIndex);
    if (!target) {
      return null;
    }
    if (reference.kind !== "symbol" || target.kind !== "project" || !outlineLoader) {
      return target;
    }

    const outline = await outlineLoader(target.relPath);
    const symbols = Array.isArray(outline?.symbols) ? outline.symbols : [];
    const matched = reference.isDefault
      ? symbols.find((symbol) => symbol?.is_exported && symbol?.is_default)
      : symbols.find((symbol) => symbol?.is_exported && symbol?.name === reference.symbolName);

    if (!matched) {
      return target;
    }

    return {
      ...target,
      line: matched.line,
      symbol: matched.name,
    };
  }

  function openImport(view, pos) {
    const reference = findImportReferenceAt(view.state, pos);
    if (!reference) {
      return false;
    }
    const target = resolveImportTarget(reference.specifier, options, projectImportIndex);
    if (!canOpenTarget(target)) {
      return false;
    }
    void resolveOpenTarget(reference).then((resolvedTarget) => {
      if (!resolvedTarget) {
        return;
      }
      if (resolvedTarget.kind === "library" && typeof options.onOpenLibraryImport === "function") {
        options.onOpenLibraryImport(resolvedTarget);
        return;
      }
      options.onOpenImport(resolvedTarget);
    });
    return true;
  }

  function isPointerImport(view, event) {
    const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (typeof pos !== "number") {
      return false;
    }
    const reference = findImportReferenceAt(view.state, pos);
    if (!reference) {
      return false;
    }
    return canOpenTarget(resolveImportTarget(reference.specifier, options, projectImportIndex));
  }

  return [
    EditorView.domEventHandlers({
      mousedown(event, view) {
        if (!(event.metaKey || event.ctrlKey)) {
          return false;
        }
        const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
        if (typeof pos !== "number") {
          return false;
        }
        if (!openImport(view, pos)) {
          return false;
        }
        event.preventDefault();
        return true;
      },
      mousemove(event, view) {
        view.dom.style.cursor = (event.metaKey || event.ctrlKey) && isPointerImport(view, event)
          ? "pointer"
          : "";
        return false;
      },
      mouseleave(_event, view) {
        view.dom.style.cursor = "";
        return false;
      },
      keydown(event, view) {
        if (event.key !== "F12") {
          return false;
        }
        if (!openImport(view, view.state.selection.main.head)) {
          return false;
        }
        event.preventDefault();
        return true;
      },
    }),
  ];
}

function createEditorShortcutExtensions(options = {}) {
  if (typeof options.onSave !== "function") {
    return [];
  }

  return [
    EditorView.domEventHandlers({
      keydown(event) {
        const isSaveKey =
          (event.metaKey || event.ctrlKey) &&
          !event.shiftKey &&
          !event.altKey &&
          event.key.toLowerCase() === "s";
        if (!isSaveKey) {
          return false;
        }
        event.preventDefault();
        options.onSave();
        return true;
      },
    }),
  ];
}

function createZebflowEditorExtensions(options = {}) {
  const extensions = [oneDark];

  if (resolveVimPreference(options)) {
    extensions.push(...createLightweightVimExtensions(options));
  }

  extensions.push(basicSetup);

  const theme = buildEditorTheme(options);
  if (theme) {
    extensions.push(theme);
  }

  if (options.autocomplete) {
    extensions.push(autocompletion());
  }

  if (options.diagnostics) {
    extensions.push(linter(() => []));
    extensions.push(lintGutter());
  }

  const language = resolveLanguageExtension(options.kind);
  if (language) {
    extensions.push(language);
  }

  if (options.autocomplete && isJavaScriptLikeKind(options.kind)) {
    extensions.push(
      javascriptLanguage.data.of({
        autocomplete: createZebflowCompletionSource(options),
      })
    );
  }

  if (typeof options.onDocumentChange === "function") {
    extensions.push(
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) {
          return;
        }
        options.onDocumentChange(update);
      })
    );
  }

  if (options.readonly) {
    extensions.push(EditorView.editable.of(false));
  }

  if (isJavaScriptLikeKind(options.kind)) {
    extensions.push(...createImportNavigationExtensions(options));
  }

  extensions.push(...createClipboardExtensions(options));
  extensions.push(...createEditorShortcutExtensions(options));

  return extensions;
}

const presets = {
  zebflow(options = {}) {
    return createZebflowEditorExtensions(options);
  },
};

const codemirror = {
  CompletionContext,
  EditorView,
  autocompletion,
  basicSetup,
  css,
  cssLanguage,
  javascript,
  javascriptLanguage,
  lintGutter,
  linter,
  oneDark,
  presets,
  setDiagnostics,
  snippetCompletion,
  createZebflowEditorExtensions,
  enableVimSupport,
};

export * from "./codemirror.bundle.mjs";
export { codemirror, createZebflowEditorExtensions, presets, enableVimSupport };
