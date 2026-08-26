/**
 * StyleEngine v2
 *
 * High-performance CSS-like cascade for PDF IR nodes.
 *
 * Selector support
 *   .class                    single class
 *   .a.b                      chained (node must have ALL listed classes)
 *   .parent .child            descendant — any depth, greedy right-to-left
 *   .a .b:nth-child(even)     combined
 *   :nth-child(even|odd)      row alternation
 *   :first-child / :last-child
 *   inline style {}           always wins (applied after cascade)
 *
 * Performance
 *   Selectors compiled once per document (parseSelector never called per node).
 *   Rules sorted by specificity once at compile time.
 *   Per-rule fast rejection via last-segment class index — O(1) Set lookup.
 *   WeakMap-based className → Set cache; input nodes are never mutated.
 */

// ─── Default stylesheet ───────────────────────────────────────────────────────

export const DEFAULT_STYLES = {
  // Document baseline
  '.doc': {
    'font-family': 'Helvetica',
    'font-size': 12,
    'color': '#000000',
    'line-height': 1.4,
  },
  '.page': {
    'background-color': '#ffffff',
  },

  // Table
  '.table': {
    'border-width': 0.5,
    'border-color': '#000000',
    'border-style': 'solid',
  },

  // Rows
  '.row': {},
  '.row:nth-child(odd)':  { 'background-color': '#ffffff' },
  '.row:nth-child(even)': { 'background-color': '#f5f5f5' },

  // Cells
  '.cell': {
    'padding': 4,
    'border-width': 0.5,
    'border-color': '#000000',
    'border-style': 'solid',
    'font-size': 11,
  },

  // Header / footer rows
  '.header': {
    'font-weight': 'bold',
    'background-color': '#222222',
    'color': '#ffffff',
  },
  '.header .cell': {
    'border-color': '#444444',
  },
  '.footer': {
    'font-weight': 'bold',
    'background-color': '#eeeeee',
    'color': '#000000',
  },

  // Utility
  '.bold':   { 'font-weight': 'bold' },
  '.italic': { 'font-style': 'italic' },
  '.small':  { 'font-size': 10 },
  '.large':  { 'font-size': 18 },
  '.highlight': { 'background-color': '#fff3cd' },

  // Primitives
  '.text':  { 'line-height': 1.4 },
  '.line':  { 'stroke': '#000000', 'stroke-width': 1, 'stroke-style': 'solid' },
  '.rect':  { 'fill': '#ffffff', 'stroke': '#000000', 'stroke-width': 1 },
  '.image': { 'display': 'block' },
};

// ─── WeakMap class-set cache ──────────────────────────────────────────────────

const _classSets = new WeakMap();

function classSet(node) {
  let s = _classSets.get(node);
  if (!s) {
    s = new Set((node.className || '').split(/\s+/).filter(Boolean));
    _classSets.set(node, s);
  }
  return s;
}

// ─── Selector parser ──────────────────────────────────────────────────────────
//
// ".invoice-table .row:nth-child(even) .cell.highlight"
//   → segments: [
//       { classes:['invoice-table'], pseudos:[] },
//       { classes:['row'],           pseudos:['nth-child(even)'] },
//       { classes:['cell','highlight'], pseudos:[] },
//     ]
//   → specificity: 10+10+1+10+10 + (3-1)*5 = 51

function parseSelector(sel) {
  const parts = sel.trim().split(/\s+/);
  const segments = [];
  let specificity = 0;

  for (const part of parts) {
    const classes = [];
    const pseudos = [];

    for (const m of part.matchAll(/\.([a-zA-Z_-][a-zA-Z0-9_-]*)/g)) {
      classes.push(m[1]);
      specificity += 10;
    }
    // Captures :pseudo and :pseudo(param) including nth-child(even)
    for (const m of part.matchAll(/:([a-zA-Z-]+(?:\([^)]*\))?)/g)) {
      pseudos.push(m[1]);
      specificity += 1;
    }
    if (classes.length || pseudos.length) {
      segments.push({ classes, pseudos });
    }
  }

  if (segments.length > 1) specificity += (segments.length - 1) * 5;

  return { segments, specificity };
}

// ─── Segment → node matching ─────────────────────────────────────────────────

function segmentMatches(seg, node, sibIdx, sibCount) {
  const cs = classSet(node);
  for (const c of seg.classes) {
    if (!cs.has(c)) return false;
  }
  for (const p of seg.pseudos) {
    if (p === 'nth-child(even)') {
      if (sibIdx === undefined || (sibIdx + 1) % 2 !== 0) return false;
    } else if (p === 'nth-child(odd)') {
      if (sibIdx === undefined || (sibIdx + 1) % 2 !== 1) return false;
    } else if (p === 'first-child') {
      if (sibIdx !== 0) return false;
    } else if (p === 'last-child') {
      if (sibCount === undefined || sibIdx !== sibCount - 1) return false;
    }
  }
  return true;
}

// ─── Full rule matching ───────────────────────────────────────────────────────
//
// Ancestor array: [{node, sibIdx, sibCount}, ...] ordered root → direct parent.
// Matching is greedy right-to-left: each preceding segment is matched against
// the nearest available ancestor (CSS descendant combinator semantics).

function ruleMatches(rule, node, ancestors, sibIdx, sibCount) {
  const segs = rule.segments;

  // Fast path: last segment must match the current node
  if (!segmentMatches(segs[segs.length - 1], node, sibIdx, sibCount)) return false;
  if (segs.length === 1) return true;

  // Walk ancestors right-to-left for remaining segments
  let ai = ancestors.length - 1;
  for (let si = segs.length - 2; si >= 0; si--) {
    let found = false;
    while (ai >= 0) {
      const a = ancestors[ai--];
      if (segmentMatches(segs[si], a.node, a.sibIdx, a.sibCount)) {
        found = true;
        break;
      }
    }
    if (!found) return false;
  }
  return true;
}

// ─── CompiledRule ─────────────────────────────────────────────────────────────

class CompiledRule {
  constructor(selector, properties) {
    const p = parseSelector(selector);
    this.segments   = p.segments;
    this.specificity = p.specificity;
    this.properties = properties;
    // Fast-rejection key: first class of the last segment
    const last = this.segments[this.segments.length - 1];
    this._key = (last && last.classes[0]) || null;
  }
}

// ─── StyleEngine ─────────────────────────────────────────────────────────────

export class StyleEngine {
  /**
   * @param {object} documentStyles  User-defined styles from the IR document.
   *                                 Merged over DEFAULT_STYLES at compile time.
   */
  constructor(documentStyles = {}) {
    this._rules = [];
    const merged = { ...DEFAULT_STYLES, ...documentStyles };
    for (const [sel, props] of Object.entries(merged)) {
      if (!props || !Object.keys(props).length) continue;
      const rule = new CompiledRule(sel, props);
      if (rule.segments.length) this._rules.push(rule);
    }
    // Ascending specificity → lower applied first, higher overwrites
    this._rules.sort((a, b) => a.specificity - b.specificity);
  }

  /**
   * Resolve the computed style for one node.
   *
   * @param {object} node       IR node (reads .className and .style)
   * @param {Array}  ancestors  [{node, sibIdx, sibCount}, ...] root → parent
   * @param {number} sibIdx     0-based index of this node among its siblings
   * @param {number} sibCount   Total sibling count at this level
   * @returns {object}          Fully expanded CSS-property map
   */
  resolve(node, ancestors = [], sibIdx = 0, sibCount = 1) {
    const result = {};
    const cs = classSet(node);

    for (const rule of this._rules) {
      // Fast reject: last segment needs a class this node doesn't have
      if (rule._key !== null && !cs.has(rule._key)) continue;
      if (ruleMatches(rule, node, ancestors, sibIdx, sibCount)) {
        Object.assign(result, rule.properties);
      }
    }

    // Inline style always wins
    if (node.style) Object.assign(result, node.style);

    return expandShorthands(result);
  }
}

// ─── Shorthand expansion ──────────────────────────────────────────────────────

export function expandShorthands(style) {
  const e = { ...style };

  if ('padding' in e) {
    const p = e.padding;
    if (typeof p === 'number') {
      e['padding-top'] = e['padding-right'] = e['padding-bottom'] = e['padding-left'] = p;
    } else if (Array.isArray(p)) {
      const [t, r, b, l] = p.length === 1 ? [p[0], p[0], p[0], p[0]]
                         : p.length === 2 ? [p[0], p[1], p[0], p[1]]
                         : p.length === 3 ? [p[0], p[1], p[2], p[1]]
                                          : [p[0], p[1], p[2], p[3]];
      e['padding-top'] = t; e['padding-right'] = r;
      e['padding-bottom'] = b; e['padding-left'] = l;
    }
    delete e.padding;
  }

  if ('margin' in e) {
    const m = e.margin;
    if (typeof m === 'number') {
      e['margin-top'] = e['margin-right'] = e['margin-bottom'] = e['margin-left'] = m;
    } else if (Array.isArray(m)) {
      const [t, r, b, l] = m.length === 1 ? [m[0], m[0], m[0], m[0]]
                         : m.length === 2 ? [m[0], m[1], m[0], m[1]]
                         : m.length === 3 ? [m[0], m[1], m[2], m[1]]
                                          : [m[0], m[1], m[2], m[3]];
      e['margin-top'] = t; e['margin-right'] = r;
      e['margin-bottom'] = b; e['margin-left'] = l;
    }
    delete e.margin;
  }

  if ('border-width' in e) {
    const v = e['border-width'];
    e['border-top-width'] = e['border-right-width'] =
    e['border-bottom-width'] = e['border-left-width'] = v;
  }
  if ('border-style' in e) {
    const v = e['border-style'];
    e['border-top-style'] = e['border-right-style'] =
    e['border-bottom-style'] = e['border-left-style'] = v;
  }
  if ('border-color' in e) {
    const v = e['border-color'];
    e['border-top-color'] = e['border-right-color'] =
    e['border-bottom-color'] = e['border-left-color'] = v;
  }

  return e;
}

// ─── Color utilities ──────────────────────────────────────────────────────────

const NAMED_COLORS = {
  black: '#000000', white: '#ffffff', red: '#ff0000', green: '#008000',
  blue: '#0000ff', yellow: '#ffff00', cyan: '#00ffff', magenta: '#ff00ff',
  gray: '#808080', grey: '#808080', silver: '#c0c0c0', maroon: '#800000',
  olive: '#808000', lime: '#00ff00', aqua: '#00ffff', teal: '#008080',
  navy: '#000080', fuchsia: '#ff00ff', purple: '#800080', orange: '#ffa500',
};

export function parseColor(color) {
  if (!color || color === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  if (color.startsWith('#')) {
    const h = color.slice(1);
    if (h.length === 3) return { r: parseInt(h[0]+h[0], 16), g: parseInt(h[1]+h[1], 16), b: parseInt(h[2]+h[2], 16), a: 1 };
    if (h.length === 6) return { r: parseInt(h.slice(0,2), 16), g: parseInt(h.slice(2,4), 16), b: parseInt(h.slice(4,6), 16), a: 1 };
  }
  if (color.startsWith('rgb')) {
    const m = color.match(/rgba?\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+))?\s*\)/);
    if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] !== undefined ? +m[4] : 1 };
  }
  const named = NAMED_COLORS[color.toLowerCase()];
  return named ? parseColor(named) : { r: 0, g: 0, b: 0, a: 1 };
}

export function colorToRgb(c) {
  const { r, g, b } = parseColor(c);
  return [r / 255, g / 255, b / 255];
}

// ─── Backward-compat shim ─────────────────────────────────────────────────────

export function computeStyle(node, userStyles = {}, context = {}) {
  const engine = new StyleEngine(userStyles);
  const sibIdx = context.rowIndex !== undefined ? context.rowIndex : 0;
  return engine.resolve(node, [], sibIdx);
}
