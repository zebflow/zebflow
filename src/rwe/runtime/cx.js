// cx(...classes): the class names joined, with a later utility replacing an
// earlier one that sets the same CSS property under the same variants — the
// rule shadcn's cn() gets from tailwind-merge, so a component's className
// passed last wins over the component's own (`<Button className="px-0">`).
//
// One file for both sides: the server's runtime and the page's client module
// include this text, so the markup the server renders and the markup that
// hydrates name the same classes. A class this table does not recognise is
// kept as written, as before — nothing is ever dropped on a guess.
globalThis.cx = (function () {
  var ALL = ["", "x", "y", "t", "r", "b", "l", "s", "e"];
  // Which narrower groups a wider one replaces: `p` covers `px`, `pt`, …
  function sides(prefix, side) {
    var covered = side === "" ? ALL : side === "x" ? ["x", "l", "r", "s", "e"] : side === "y" ? ["y", "t", "b"] : [side];
    return covered.map(function (s) { return prefix + s; });
  }
  var CORNERS = {
    "": ["", "t", "r", "b", "l", "s", "e", "tl", "tr", "br", "bl", "ss", "se", "es", "ee"],
    t: ["t", "tl", "tr"], r: ["r", "tr", "br"], b: ["b", "br", "bl"], l: ["l", "tl", "bl"],
    s: ["s", "ss", "es"], e: ["e", "se", "ee"],
  };
  var SIDE = /^(x|y|t|r|b|l|s|e)$/;
  var LENGTH = /^\[(?:length:)?[-\d.]+(px|rem|em|%|vh|vw|ch|ex|pt)?\]$/;
  var COLORISH = /^\[(?:color:)?(#|rgb|hsl|oklch|oklab|lab|lch|color\(|var\(--color)/;
  var TEXT_SIZE = /^(xs|sm|base|lg|xl|[2-9]xl)(\/.*)?$/;
  var WORDS = {
    display: "block inline-block inline flex inline-flex grid inline-grid hidden contents table inline-table table-row table-cell table-caption table-column table-row-group table-header-group table-footer-group table-column-group flow-root list-item",
    position: "static fixed absolute relative sticky",
    "text-align": "text-left text-center text-right text-justify text-start text-end",
    "text-wrap": "text-wrap text-nowrap text-balance text-pretty",
    "text-overflow": "text-ellipsis text-clip truncate",
    "font-weight": "font-thin font-extralight font-light font-normal font-medium font-semibold font-bold font-extrabold font-black",
    "font-family": "font-sans font-serif font-mono",
    "border-style": "border-solid border-dashed border-dotted border-double border-hidden border-none",
    "flex-direction": "flex-row flex-row-reverse flex-col flex-col-reverse",
    "flex-wrap": "flex-wrap flex-nowrap flex-wrap-reverse",
    flex: "flex-1 flex-auto flex-initial flex-none",
    "bg-attachment": "bg-fixed bg-local bg-scroll",
    "bg-size": "bg-auto bg-cover bg-contain",
    "bg-repeat": "bg-repeat bg-no-repeat bg-repeat-x bg-repeat-y bg-repeat-round bg-repeat-space",
    "bg-position": "bg-bottom bg-center bg-left bg-left-bottom bg-left-top bg-right bg-right-bottom bg-right-top bg-top",
    "shadow-size": "shadow shadow-2xs shadow-xs shadow-sm shadow-md shadow-lg shadow-xl shadow-2xl shadow-none shadow-inner",
  };
  var WORD = {};
  Object.keys(WORDS).forEach(function (group) {
    WORDS[group].split(" ").forEach(function (word) { WORD[word] = group; });
  });

  // [group, groups it replaces] for one utility without its variants, or
  // null when this table does not know it.
  function groupOf(u) {
    var neg = u.charAt(0) === "-" ? u.slice(1) : u;
    if (WORD[u]) return [WORD[u], [WORD[u]]];
    var m = /^(p|m)(x|y|t|r|b|l|s|e)?-(.+)$/.exec(neg);
    if (m) return [m[1] + (m[2] || ""), sides(m[1], m[2] || "")];
    m = /^(w|h|size|min-w|min-h|max-w|max-h)-(.+)$/.exec(u);
    if (m) return [m[1], m[1] === "size" ? ["size", "w", "h"] : [m[1]]];
    m = /^gap(-x|-y)?-(.+)$/.exec(u);
    if (m) return ["gap" + (m[1] || ""), m[1] ? ["gap" + m[1]] : ["gap", "gap-x", "gap-y"]];
    m = /^(inset|inset-x|inset-y|top|right|bottom|left|start|end)-(.+)$/.exec(neg);
    if (m) {
      var cover = { inset: ["inset", "inset-x", "inset-y", "top", "right", "bottom", "left", "start", "end"],
        "inset-x": ["inset-x", "left", "right", "start", "end"], "inset-y": ["inset-y", "top", "bottom"] }[m[1]];
      return [m[1], cover || [m[1]]];
    }
    m = /^rounded(?:-(.+))?$/.exec(u);
    if (m) {
      var parts = (m[1] || "").split("-");
      var corner = CORNERS[parts[0]] || /^(tl|tr|br|bl|ss|se|es|ee)$/.test(parts[0]) ? parts[0] : "";
      return ["rounded-" + corner, (CORNERS[corner] || [corner]).map(function (c) { return "rounded-" + c; })];
    }
    m = /^border(?:-(.+))?$/.exec(u);
    if (m) {
      var rest = m[1] || "";
      var bits = rest.split("-");
      var side = SIDE.test(bits[0]) ? bits.shift() : "";
      var value = bits.join("-");
      if (value === "" || /^\d+$/.test(value) || LENGTH.test(value)) return ["bw" + side, sides("bw", side)];
      if (/^(collapse|separate|spacing)/.test(rest)) return null;
      if (/^\[/.test(value) && !COLORISH.test(value)) return null;
      return ["bc" + side, sides("bc", side)];
    }
    m = /^text-(.+)$/.exec(u);
    if (m) {
      if (TEXT_SIZE.test(m[1]) || LENGTH.test(m[1])) return ["text-size", ["text-size"]];
      if (/^\[/.test(m[1]) && !COLORISH.test(m[1])) return null;
      return ["text-color", ["text-color"]];
    }
    m = /^bg-(.+)$/.exec(u);
    if (m) {
      if (/^(none|gradient-|linear-|radial|conic)/.test(m[1]) || /^\[url/.test(m[1])) return ["bg-image", ["bg-image"]];
      if (/^(clip|origin)-/.test(m[1])) return ["bg-" + m[1].split("-")[0], ["bg-" + m[1].split("-")[0]]];
      if (/^\[/.test(m[1]) && !COLORISH.test(m[1])) return null;
      return ["bg-color", ["bg-color"]];
    }
    m = /^shadow-(.+)$/.exec(u);
    if (m) return /^\[/.test(m[1]) ? null : ["shadow-color", ["shadow-color"]];
    m = /^ring(?:-(.+))?$/.exec(u);
    if (m) {
      // A bracket value is a width when it is a length (`ring-[3px]`) and a
      // colour when it reads as one, as for `border-`; anything else stays.
      if (!m[1] || /^\d+$/.test(m[1]) || LENGTH.test(m[1])) return ["ring-width", ["ring-width"]];
      if (/^(offset|inset)/.test(m[1])) return null;
      if (/^\[/.test(m[1]) && !COLORISH.test(m[1])) return null;
      return ["ring-color", ["ring-color"]];
    }
    m = /^(leading|tracking|opacity|z|items|self|cursor|whitespace|select|overflow-x|overflow-y|overflow)-(.+)$/.exec(u);
    if (m) {
      if (m[1] === "overflow") return ["overflow", ["overflow", "overflow-x", "overflow-y"]];
      return [m[1], [m[1]]];
    }
    m = /^justify-(start|end|center|between|around|evenly|stretch|normal)$/.exec(u);
    if (m) return ["justify-content", ["justify-content"]];
    return null;
  }

  // `md:hover:!px-0` -> variants "md:hover:", important, utility "px-0".
  function split(token) {
    var depth = 0, last = -1;
    for (var i = 0; i < token.length; i++) {
      var c = token.charAt(i);
      if (c === "[") depth++;
      else if (c === "]") depth--;
      else if (c === ":" && depth === 0) last = i;
    }
    var variants = token.slice(0, last + 1);
    var utility = token.slice(last + 1);
    var important = "";
    if (utility.charAt(0) === "!") { important = "!"; utility = utility.slice(1); }
    else if (utility.charAt(utility.length - 1) === "!") { important = "!"; utility = utility.slice(0, -1); }
    return [variants + important, utility];
  }

  return function cx() {
    var tokens = [];
    for (var i = 0; i < arguments.length; i++) {
      var part = arguments[i];
      if (!part) continue;
      String(part).split(/\s+/).forEach(function (t) { if (t) tokens.push(t); });
    }
    var claimed = {};
    var kept = [];
    for (var j = tokens.length - 1; j >= 0; j--) {
      var parsed = split(tokens[j]);
      var group = groupOf(parsed[1]);
      if (!group) { kept.push(tokens[j]); continue; }
      var key = parsed[0] + group[0];
      if (claimed[key]) continue;
      group[1].forEach(function (g) { claimed[parsed[0] + g] = true; });
      kept.push(tokens[j]);
    }
    return kept.reverse().join(" ");
  };
})();
