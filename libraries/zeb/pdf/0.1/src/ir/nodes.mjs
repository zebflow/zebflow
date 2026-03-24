/**
 * IR Node Types & Validators
 * 
 * Each node type has:
 *   - type: string identifier
 *   - validate(node): throws if invalid
 *   - defaultStyles: base styles for cascade
 */

export const NODE_TYPES = {
  DOCUMENT: 'document',
  PAGE: 'page',
  TEXT: 'text',
  LINE: 'line',
  RECT: 'rect',
  IMAGE: 'image',
  TABLE: 'table',
  ROW: 'row',
  CELL: 'cell',
  CONTAINER: 'container',
  COLUMNS: 'columns',
  FLOAT: 'float',
  ABSOLUTE: 'absolute',
  RELATIVE: 'relative',
};

export const PAGE_SIZES = {
  A4: { width: 595, height: 842 },
  A3: { width: 842, height: 1191 },
  A5: { width: 420, height: 595 },
  Letter: { width: 612, height: 792 },
  Legal: { width: 612, height: 1008 },
  Tabloid: { width: 792, height: 1224 },
};

const VALID_FONTS = [
  'Helvetica', 'Helvetica-Bold', 'Helvetica-Oblique', 'Helvetica-BoldOblique',
  'Times-Roman', 'Times-Bold', 'Times-Italic', 'Times-BoldItalic',
  'Courier', 'Courier-Bold', 'Courier-Oblique', 'Courier-BoldOblique',
  'Symbol', 'ZapfDingbats',
];

const VALID_FONT_SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72];
const VALID_BORDER_WIDTHS = [0, 0.5, 1, 1.5, 2];
const VALID_PADDINGS = [0, 2, 4, 8, 12, 16, 24, 32, 48];

function isString(v) { return typeof v === 'string'; }
function isNumber(v) { return typeof v === 'number' && !isNaN(v); }
function isObject(v) { return v !== null && typeof v === 'object' && !Array.isArray(v); }
function isArray(v) { return Array.isArray(v); }
function isBoolean(v) { return typeof v === 'boolean'; }
function isFunction(v) { return typeof v === 'function'; }

function assert(condition, msg) {
  if (!condition) throw new Error(`Invalid IR node: ${msg}`);
}

function assertEnum(value, allowed, fieldName) {
  assert(allowed.includes(value), `${fieldName} must be one of: ${allowed.join(', ')}, got "${value}"`);
}

function assertFontFamily(value) {
  if (value) assertEnum(value, VALID_FONTS, 'font-family');
}

function assertFontSize(value) {
  if (value) assertEnum(value, VALID_FONT_SIZES, 'font-size');
}

export function validateNode(node) {
  assert(isObject(node), 'node must be an object');
  assert(isString(node.type), 'node.type must be a string');
  
  switch (node.type) {
    case NODE_TYPES.DOCUMENT:
      validateDocument(node);
      break;
    case NODE_TYPES.PAGE:
      validatePage(node);
      break;
    case NODE_TYPES.TEXT:
      validateText(node);
      break;
    case NODE_TYPES.LINE:
      validateLine(node);
      break;
    case NODE_TYPES.RECT:
      validateRect(node);
      break;
    case NODE_TYPES.IMAGE:
      validateImage(node);
      break;
    case NODE_TYPES.TABLE:
      validateTable(node);
      break;
    case NODE_TYPES.ROW:
      validateRow(node);
      break;
    case NODE_TYPES.CELL:
      validateCell(node);
      break;
    case NODE_TYPES.CONTAINER:
      validateContainer(node);
      break;
    case NODE_TYPES.COLUMNS:
      validateColumns(node);
      break;
    case NODE_TYPES.FLOAT:
      validateFloat(node);
      break;
    case NODE_TYPES.ABSOLUTE:
      validateAbsolute(node);
      break;
    case NODE_TYPES.RELATIVE:
      validateRelative(node);
      break;
    default:
      assert(false, `unknown node type: ${node.type}`);
  }
}

function validateDocument(node) {
  if (node.meta) {
    assert(isObject(node.meta), 'document.meta must be an object');
  }
  if (node.styles) {
    assert(isObject(node.styles), 'document.styles must be an object');
  }
  if (node.settings) {
    assert(isObject(node.settings), 'document.settings must be an object');
  }
  if (node.children) {
    assert(isArray(node.children), 'document.children must be an array');
    node.children.forEach(validateNode);
  }
}

function validatePage(node) {
  if (node.size) {
    if (isString(node.size)) {
      assert(PAGE_SIZES[node.size], `unknown page size: ${node.size}`);
    } else {
      assert(isObject(node.size), 'page.size must be string or {width, height}');
      assert(isNumber(node.size.width), 'page.size.width must be a number');
      assert(isNumber(node.size.height), 'page.size.height must be a number');
    }
  }
  if (node.orientation) {
    assertEnum(node.orientation, ['portrait', 'landscape'], 'orientation');
  }
  if (node.rotate) {
    assertEnum(node.rotate, [0, 90, 180, 270], 'rotate');
  }
  if (node.margin) {
    assert(isObject(node.margin), 'page.margin must be an object');
    ['top', 'right', 'bottom', 'left'].forEach(k => {
      if (node.margin[k] !== undefined) assert(isNumber(node.margin[k]), `page.margin.${k} must be a number`);
    });
  }
  if (node.header) {
    assert(isObject(node.header), 'page.header must be an object');
  }
  if (node.footer) {
    assert(isObject(node.footer), 'page.footer must be an object');
  }
  if (node.children) {
    assert(isArray(node.children), 'page.children must be an array');
    node.children.forEach(validateNode);
  }
}

function validateText(node) {
  assert(isString(node.value) || isArray(node.runs), 'text must have value (string) or runs (array)');
  if (node.runs) {
    assert(isArray(node.runs), 'text.runs must be an array');
    node.runs.forEach(run => {
      assert(isObject(run), 'text.run must be an object');
      assert(isString(run.text), 'text.run.text must be a string');
    });
  }
  if (node.x !== undefined) assert(isNumber(node.x), 'text.x must be a number');
  if (node.y !== undefined) assert(isNumber(node.y), 'text.y must be a number');
  if (node.className) assert(isString(node.className), 'className must be a string');
  if (node.style) assert(isObject(node.style), 'text.style must be an object');
}

function validateLine(node) {
  assert(isNumber(node.x1), 'line.x1 must be a number');
  if (node.y1 !== undefined) assert(isNumber(node.y1), 'line.y1 must be a number');
  assert(isNumber(node.x2), 'line.x2 must be a number');
  if (node.y2 !== undefined) assert(isNumber(node.y2), 'line.y2 must be a number');
  if (node.style) assertEnum(node.style, ['solid', 'dashed', 'dotted'], 'line.style');
  if (node.width !== undefined) assert(isNumber(node.width), 'line.width must be a number');
  if (node.color) assert(isString(node.color), 'line.color must be a string');
}

function validateRect(node) {
  assert(isNumber(node.x), 'rect.x must be a number');
  assert(isNumber(node.y), 'rect.y must be a number');
  assert(isNumber(node.width), 'rect.width must be a number');
  assert(isNumber(node.height), 'rect.height must be a number');
  if (node.fill !== undefined) assert(isString(node.fill) || node.fill === null, 'rect.fill must be string or null');
  if (node.stroke !== undefined) assert(isString(node.stroke) || node.stroke === null, 'rect.stroke must be string or null');
  if (node.strokeWidth !== undefined) assert(isNumber(node.strokeWidth), 'rect.strokeWidth must be a number');
  if (node.radius !== undefined) assert(isNumber(node.radius), 'rect.radius must be a number');
}

function validateImage(node) {
  assert(isString(node.data) || node.data instanceof Uint8Array, 'image.data must be string or Uint8Array');
  if (node.x !== undefined) assert(isNumber(node.x), 'image.x must be a number');
  if (node.y !== undefined) assert(isNumber(node.y), 'image.y must be a number');
  if (node.width !== undefined) assert(isNumber(node.width), 'image.width must be a number');
  if (node.height !== undefined) assert(isNumber(node.height), 'image.height must be a number');
  if (node.format) assertEnum(node.format, ['png', 'jpeg', 'webp'], 'image.format');
  if (node.quality !== undefined) assert(node.quality >= 0 && node.quality <= 1, 'image.quality must be 0-1');
}

function validateTable(node) {
  if (node.columnWidths) {
    assert(isArray(node.columnWidths), 'table.columnWidths must be an array');
  }
  if (node.columnAligns) {
    assert(isArray(node.columnAligns), 'table.columnAligns must be an array');
  }
  if (node.header) {
    validateNode(node.header);
  }
  if (node.body) {
    assert(isArray(node.body), 'table.body must be an array');
    node.body.forEach(validateNode);
  }
  if (node.className) assert(isString(node.className), 'className must be a string');
  if (node.style) assert(isObject(node.style), 'style must be an object');
}

function validateRow(node) {
  assert(isArray(node.cells), 'row.cells must be an array');
  node.cells.forEach(validateNode);
  if (node.height !== undefined && node.height !== 'auto') {
    assert(isNumber(node.height), 'row.height must be number or "auto"');
  }
  if (node.className) assert(isString(node.className), 'className must be a string');
}

function validateCell(node) {
  if (node.value !== undefined) assert(isString(node.value), 'cell.value must be a string');
  if (node.colspan !== undefined) assert(isNumber(node.colspan), 'cell.colspan must be a number');
  if (node.rowspan !== undefined) assert(isNumber(node.rowspan), 'cell.rowspan must be a number');
  if (node.children) {
    assert(isArray(node.children), 'cell.children must be an array');
    node.children.forEach(validateNode);
  }
  if (node.className) assert(isString(node.className), 'className must be a string');
  if (node.style) assert(isObject(node.style), 'cell.style must be an object');
}

function validateContainer(node) {
  if (node.x !== undefined) assert(isNumber(node.x), 'container.x must be a number');
  if (node.y !== undefined) assert(isNumber(node.y), 'container.y must be a number');
  if (node.width !== undefined) assert(isNumber(node.width), 'container.width must be a number');
  if (node.height !== undefined) assert(isNumber(node.height), 'container.height must be a number');
  if (node.overflow) assertEnum(node.overflow, ['hidden', 'visible', 'auto'], 'container.overflow');
  if (node.children) {
    assert(isArray(node.children), 'container.children must be an array');
    node.children.forEach(validateNode);
  }
}

function validateColumns(node) {
  // Must have either widths[] or count
  if (node.widths !== undefined) {
    assert(isArray(node.widths) && node.widths.length > 0, 'columns.widths must be a non-empty array');
  } else {
    assert(isNumber(node.count), 'columns must have count (number) or widths (array)');
  }
  if (node.gap !== undefined) assert(isNumber(node.gap), 'columns.gap must be a number');
  if (node.rule !== undefined) assert(isBoolean(node.rule), 'columns.rule must be boolean');
  if (node.children) {
    assert(isArray(node.children), 'columns.children must be an array');
    node.children.forEach(validateNode);
  }
}

function validateFloat(node) {
  assertEnum(node.side, ['left', 'right', 'start', 'end'], 'float.side');
  if (node.margin !== undefined) assert(isNumber(node.margin), 'float.margin must be a number');
  if (node.clear) assertEnum(node.clear, ['both', 'left', 'right'], 'float.clear');
  assert(isArray(node.children), 'float.children must be an array');
  node.children.forEach(validateNode);
}

function validateAbsolute(node) {
  if (node.x !== undefined) assert(isNumber(node.x), 'absolute.x must be a number');
  if (node.y !== undefined) assert(isNumber(node.y), 'absolute.y must be a number');
  if (node.width !== undefined) assert(isNumber(node.width), 'absolute.width must be a number');
  if (node.zIndex !== undefined) assert(isNumber(node.zIndex), 'absolute.zIndex must be a number');
  assert(isArray(node.children), 'absolute.children must be an array');
  node.children.forEach(validateNode);
}

function validateRelative(node) {
  if (node.dx !== undefined) assert(isNumber(node.dx), 'relative.dx must be a number');
  if (node.dy !== undefined) assert(isNumber(node.dy), 'relative.dy must be a number');
  assert(isArray(node.children), 'relative.children must be an array');
  node.children.forEach(validateNode);
}

export function getPageDimensions(pageNode, documentSettings = {}) {
  let width, height;
  
  if (pageNode.size) {
    if (isString(pageNode.size)) {
      ({ width, height } = PAGE_SIZES[pageNode.size]);
    } else {
      ({ width, height } = pageNode.size);
    }
  } else if (documentSettings.pageSize) {
    const docSize = documentSettings.pageSize;
    if (isString(docSize)) {
      ({ width, height } = PAGE_SIZES[docSize]);
    } else {
      ({ width, height } = docSize);
    }
  } else {
    ({ width, height } = PAGE_SIZES.A4);
  }
  
  const orientation = pageNode.orientation || documentSettings.pageOrientation || 'portrait';
  if (orientation === 'landscape') {
    return { width: height, height: width };
  }
  
  return { width, height };
}

export function getEffectiveMargin(pageNode, documentSettings = {}) {
  const defaultMargin = documentSettings.margin || 72;
  const pageMargin = pageNode.margin || {};
  
  return {
    top: pageMargin.top !== undefined ? pageMargin.top : (typeof defaultMargin === 'number' ? defaultMargin : defaultMargin.top || 72),
    right: pageMargin.right !== undefined ? pageMargin.right : (typeof defaultMargin === 'number' ? defaultMargin : defaultMargin.right || 72),
    bottom: pageMargin.bottom !== undefined ? pageMargin.bottom : (typeof defaultMargin === 'number' ? defaultMargin : defaultMargin.bottom || 72),
    left: pageMargin.left !== undefined ? pageMargin.left : (typeof defaultMargin === 'number' ? defaultMargin : defaultMargin.left || 72),
  };
}
