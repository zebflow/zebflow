/**
 * Layout utilities — font metrics and text measurement
 *
 * AFM advance widths in 1000-unit em (divide by 1000, multiply by fontSize for pt).
 * Source: Adobe Type 1 AFM data for built-in PDF fonts.
 */

// ── Helvetica (Regular) ──────────────────────────────────────────────────────
const HELVETICA_WIDTHS = {
  ' ': 278, '!': 278, '"': 355, '#': 556, '$': 556, '%': 889, '&': 667,
  "'": 222, '(': 333, ')': 333, '*': 389, '+': 584, ',': 278, '-': 333,
  '.': 278, '/': 278,
  '0': 556, '1': 556, '2': 556, '3': 556, '4': 556,
  '5': 556, '6': 556, '7': 556, '8': 556, '9': 556,
  ':': 278, ';': 278, '<': 584, '=': 584, '>': 584, '?': 556, '@': 1015,
  'A': 667, 'B': 667, 'C': 722, 'D': 722, 'E': 667, 'F': 611, 'G': 778,
  'H': 722, 'I': 278, 'J': 500, 'K': 667, 'L': 556, 'M': 833, 'N': 722,
  'O': 778, 'P': 667, 'Q': 778, 'R': 722, 'S': 667, 'T': 611, 'U': 722,
  'V': 667, 'W': 944, 'X': 667, 'Y': 667, 'Z': 611,
  '[': 278, '\\': 278, ']': 278, '^': 469, '_': 556, '`': 333,
  'a': 556, 'b': 556, 'c': 500, 'd': 556, 'e': 556, 'f': 278, 'g': 556,
  'h': 556, 'i': 222, 'j': 222, 'k': 500, 'l': 222, 'm': 833, 'n': 556,
  'o': 556, 'p': 556, 'q': 556, 'r': 333, 's': 500, 't': 278, 'u': 556,
  'v': 500, 'w': 722, 'x': 500, 'y': 500, 'z': 500,
  '{': 334, '|': 260, '}': 334, '~': 584,
};

// ── Helvetica-Bold ───────────────────────────────────────────────────────────
const HELVETICA_BOLD_WIDTHS = {
  ' ': 278, '!': 333, '"': 474, '#': 556, '$': 556, '%': 889, '&': 722,
  "'": 278, '(': 333, ')': 333, '*': 389, '+': 584, ',': 278, '-': 333,
  '.': 278, '/': 278,
  '0': 556, '1': 556, '2': 556, '3': 556, '4': 556,
  '5': 556, '6': 556, '7': 556, '8': 556, '9': 556,
  ':': 333, ';': 333, '<': 584, '=': 584, '>': 584, '?': 611, '@': 975,
  'A': 722, 'B': 722, 'C': 722, 'D': 722, 'E': 667, 'F': 611, 'G': 778,
  'H': 722, 'I': 278, 'J': 556, 'K': 722, 'L': 611, 'M': 833, 'N': 722,
  'O': 778, 'P': 667, 'Q': 778, 'R': 722, 'S': 667, 'T': 611, 'U': 722,
  'V': 667, 'W': 944, 'X': 667, 'Y': 667, 'Z': 611,
  '[': 333, '\\': 278, ']': 333, '^': 584, '_': 556, '`': 278,
  'a': 556, 'b': 611, 'c': 556, 'd': 611, 'e': 556, 'f': 333, 'g': 611,
  'h': 611, 'i': 278, 'j': 278, 'k': 556, 'l': 278, 'm': 889, 'n': 611,
  'o': 611, 'p': 611, 'q': 611, 'r': 389, 's': 556, 't': 333, 'u': 611,
  'v': 556, 'w': 778, 'x': 556, 'y': 556, 'z': 500,
  '{': 389, '|': 280, '}': 389, '~': 584,
};

// ── Times-Roman ──────────────────────────────────────────────────────────────
const TIMES_ROMAN_WIDTHS = {
  ' ': 250, '!': 333, '"': 408, '#': 500, '$': 500, '%': 833, '&': 778,
  "'": 180, '(': 333, ')': 333, '*': 500, '+': 564, ',': 250, '-': 333,
  '.': 250, '/': 278,
  '0': 500, '1': 500, '2': 500, '3': 500, '4': 500,
  '5': 500, '6': 500, '7': 500, '8': 500, '9': 500,
  ':': 278, ';': 278, '<': 564, '=': 564, '>': 564, '?': 444, '@': 921,
  'A': 722, 'B': 667, 'C': 667, 'D': 722, 'E': 611, 'F': 556, 'G': 722,
  'H': 722, 'I': 333, 'J': 389, 'K': 722, 'L': 611, 'M': 889, 'N': 722,
  'O': 722, 'P': 556, 'Q': 722, 'R': 667, 'S': 556, 'T': 611, 'U': 722,
  'V': 722, 'W': 944, 'X': 722, 'Y': 722, 'Z': 611,
  '[': 333, '\\': 278, ']': 333, '^': 469, '_': 500, '`': 333,
  'a': 444, 'b': 500, 'c': 444, 'd': 500, 'e': 444, 'f': 333, 'g': 500,
  'h': 500, 'i': 278, 'j': 278, 'k': 500, 'l': 278, 'm': 778, 'n': 500,
  'o': 500, 'p': 500, 'q': 500, 'r': 333, 's': 389, 't': 278, 'u': 500,
  'v': 500, 'w': 722, 'x': 500, 'y': 500, 'z': 444,
  '{': 480, '|': 200, '}': 480, '~': 541,
};

// Courier is monospace — every glyph is 600 units wide.
const COURIER_WIDTH = 600;

// Map font name → width table. Oblique/Italic variants share regular widths.
const FONT_WIDTHS = {
  'Helvetica':            HELVETICA_WIDTHS,
  'Helvetica-Bold':       HELVETICA_BOLD_WIDTHS,
  'Helvetica-Oblique':    HELVETICA_WIDTHS,
  'Helvetica-BoldOblique': HELVETICA_BOLD_WIDTHS,
  'Times-Roman':          TIMES_ROMAN_WIDTHS,
  'Times-Bold':           TIMES_ROMAN_WIDTHS,
  'Times-Italic':         TIMES_ROMAN_WIDTHS,
  'Times-BoldItalic':     TIMES_ROMAN_WIDTHS,
};

/**
 * Returns the 1000-unit advance width for a single character.
 * Falls back to 556 (Helvetica average) for unknown fonts or characters.
 * @param {string} char  Single character
 * @param {string} fontName  e.g. 'Helvetica', 'Helvetica-Bold', 'Courier'
 * @returns {number}
 */
export function getCharWidth(char, fontName) {
  if (fontName && fontName.startsWith('Courier')) return COURIER_WIDTH;
  const table = FONT_WIDTHS[fontName] || HELVETICA_WIDTHS;
  return table[char] !== undefined ? table[char] : 556;
}

/**
 * Measures the rendered width of a string in points.
 * @param {string} text
 * @param {string} fontName  e.g. 'Helvetica', 'Helvetica-Bold'
 * @param {number} fontSize  in pt
 * @returns {number} width in pt
 */
export function measureTextWidth(text, fontName, fontSize) {
  if (!text) return 0;
  let total = 0;
  for (const ch of text) {
    total += getCharWidth(ch, fontName);
  }
  return total * fontSize / 1000;
}

/**
 * Greedy word-wrap: splits text into lines that each fit within maxWidth.
 * Single words wider than maxWidth are broken character-by-character.
 *
 * @param {string} text
 * @param {number} maxWidth  in pt
 * @param {string} fontName
 * @param {number} fontSize  in pt
 * @returns {string[]}  Array of lines (at least one element)
 */
export function wrapText(text, maxWidth, fontName, fontSize) {
  if (!text) return [''];

  const words = text.split(' ');
  const lines = [];
  let current = '';

  for (const word of words) {
    const candidate = current ? current + ' ' + word : word;

    if (measureTextWidth(candidate, fontName, fontSize) <= maxWidth) {
      current = candidate;
    } else {
      // Flush current line
      if (current) lines.push(current);

      // Single word wider than maxWidth — break it character by character
      if (measureTextWidth(word, fontName, fontSize) > maxWidth) {
        let partial = '';
        for (const ch of word) {
          const next = partial + ch;
          if (measureTextWidth(next, fontName, fontSize) > maxWidth) {
            if (partial) lines.push(partial);
            partial = ch;
          } else {
            partial = next;
          }
        }
        current = partial;
      } else {
        current = word;
      }
    }
  }

  if (current) lines.push(current);
  return lines.length > 0 ? lines : [''];
}
