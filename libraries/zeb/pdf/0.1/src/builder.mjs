/**
 * Fluent Builder API
 * 
 * Chainable API for building PDF documents:
 * 
 *   createDocument({ title: 'Invoice' })
 *     .page()
 *       .text('Hello World')
 *       .image({ data: '...', x: 400, y: 100 })
 *     .toBytes()
 */
import { renderSync } from "./render/to-pdf.mjs";
import { NODE_TYPES, PAGE_SIZES } from "./ir/nodes.mjs";
import { wrapText } from "./ir/layout.mjs";

let _nodeIdCounter = 0;
function genId() {
  return `node_${++_nodeIdCounter}`;
}

export function createDocument(options = {}) {
  return new DocumentBuilder(options);
}

export function createTable(options = {}) {
  return new TableBuilder(null, options);
}

class DocumentBuilder {
  constructor(options = {}) {
    this._id = genId();
    this._meta = options.meta || {};
    this._styles = options.styles || {};
    this._settings = options.settings || {};
    this._children = [];
    this._currentPage = null;  // tracks last PageBuilder for textFlow()
    this._defaultHeader = null;
    this._defaultFooter = null;
  }

  page(options = {}) {
    const page = new PageBuilder(this, options);
    this._children.push(page._node);
    this._currentPage = page;
    // Cache first header/footer seen as document defaults for overflow pages
    if (page._node.header && !this._defaultHeader) this._defaultHeader = page._node.header;
    if (page._node.footer && !this._defaultFooter) this._defaultFooter = page._node.footer;
    return page;
  }

  addPage(options = {}) {
    return this.page(options);
  }

  /**
   * Flow long text across pages, wrapping at maxWidth (or page effective width).
   * Automatically adds new pages when content overflows the bottom margin.
   *
   * @param {string} text
   * @param {object} options
   *   @param {number}  [options.maxWidth]       Wrap width in pt (defaults to page effective width)
   *   @param {object}  [options.style]          CSS-style object: font-family, font-size, font-weight, color…
   *   @param {string}  [options.className]
   *   @param {object}  [options.pageOptions]    Options forwarded to addPage() when a new page is needed
   * @returns {DocumentBuilder} this (for chaining)
   */
  textFlow(text, options = {}) {
    if (!this._currentPage) {
      this.page(options.pageOptions || {});
    }

    // Apply marginTop spacing before the first line
    const marginTop    = options.marginTop    || 0;
    const marginBottom = options.marginBottom || 0;
    if (marginTop) this._currentPage._yCursor -= marginTop;

    const style = options.style || {};
    const fontFamily = style['font-family'] || 'Helvetica';
    const fontWeight = style['font-weight'] || 'normal';
    const fontStyle = style['font-style'] || 'normal';
    // Resolve full font name the same way renderText() does
    let fontName = fontFamily;
    if (fontWeight === 'bold' && fontStyle === 'italic') fontName = fontFamily + '-BoldOblique';
    else if (fontWeight === 'bold') fontName = fontFamily + '-Bold';
    else if (fontStyle === 'italic') fontName = fontFamily + '-Oblique';

    const fontSize = style['font-size'] || 12;
    const lineHeight = style['line-height'] || 1.4;
    const lineStep = fontSize * lineHeight;

    const effectiveWidth = () =>
      this._currentPage._pageWidth
      - this._currentPage._margin.left
      - this._currentPage._margin.right;

    const maxWidth = options.maxWidth || effectiveWidth();
    const lines = wrapText(text, maxWidth, fontName, fontSize);

    for (const line of lines) {
      // If no room for another line on this page, start a new one
      if (this._currentPage._yCursor < this._currentPage._margin.bottom + this._currentPage._footerReserve + lineStep) {
        const overflowOpts = {
          size: this._currentPage._node.size,
          margin: this._currentPage._node.margin,
          ...( options.pageOptions || {} ),
          header: (options.pageOptions && options.pageOptions.header != null)
            ? options.pageOptions.header
            : this._defaultHeader,
          footer: (options.pageOptions && options.pageOptions.footer != null)
            ? options.pageOptions.footer
            : this._defaultFooter,
        };
        this.page(overflowOpts);
      }
      const curPage = this._currentPage;
      const textNode = {
        type: NODE_TYPES.TEXT,
        id: genId(),
        value: line,
        x: curPage._margin.left,
        y: curPage._yCursor,
        _flow: true,
      };
      if (options.style) textNode.style = options.style;
      if (options.className) textNode.className = options.className;
      curPage._node.children.push(textNode);
      curPage._yCursor -= lineStep;
    }

    // Apply marginBottom spacing after last line
    if (marginBottom) this._currentPage._yCursor -= marginBottom;

    return this;
  }

  /**
   * Flow a table across pages, repeating the header on every overflow page.
   *
   * @param {TableBuilder|object} tableInput  A TableBuilder instance or raw table IR node
   * @param {object} options
   *   @param {object} [options.pageOptions]  Options forwarded to page() when overflow occurs
   * @returns {DocumentBuilder} this (for chaining)
   */
  tableFlow(tableInput, options = {}) {
    const tableNode = tableInput?._node ?? tableInput;
    const headerRow = tableNode.header;
    const bodyRows  = tableNode.body || [];
    const headerH   = headerRow ? 22 : 0;

    if (!this._currentPage) this.page(options.pageOptions || {});

    const overflowPageOpts = () => ({
      size:   this._currentPage._node.size,
      margin: this._currentPage._node.margin,
      ...(options.pageOptions || {}),
      header: (options.pageOptions && options.pageOptions.header != null)
        ? options.pageOptions.header
        : this._defaultHeader,
      footer: (options.pageOptions && options.pageOptions.footer != null)
        ? options.pageOptions.footer
        : this._defaultFooter,
    });

    let chunk = [];

    const commitChunk = () => {
      const fragment = { ...tableNode, header: headerRow, body: [...chunk], _flow: true };
      this._currentPage._node.children.push(fragment);
      const used = headerH + chunk.reduce((s, r) => s + (parseFloat(r.style?.height) || 20), 0);
      this._currentPage._yCursor -= used;
      chunk = [];
    };

    for (const row of bodyRows) {
      const rowH      = parseFloat(row.style?.height) || 20;
      const available = this._currentPage._yCursor - this._currentPage._margin.bottom - this._currentPage._footerReserve;

      if (chunk.length > 0 && available < rowH) {
        commitChunk();
        this.page(overflowPageOpts());
      }
      chunk.push(row);
    }

    commitChunk();
    return this;
  }

  toBytes() {
    const doc = {
      type: NODE_TYPES.DOCUMENT,
      id: this._id,
      meta: this._meta,
      styles: this._styles,
      settings: this._settings,
      children: this._children,
    };
    return renderSync(doc);
  }

  toBlob() {
    return new Blob([this.toBytes()], { type: "application/pdf" });
  }

  toUrl() {
    return URL.createObjectURL(this.toBlob());
  }
}

class PageBuilder {
  constructor(parent, options = {}) {
    this._parent = parent;
    this._node = {
      type: NODE_TYPES.PAGE,
      id: genId(),
      className: options.className ? `page ${options.className}` : 'page',
      size: options.size || 'A4',
      orientation: options.orientation,
      margin: options.margin,
      children: [],
    };

    if (options.style)  this._node.style  = options.style;
    if (options.header) this._node.header = options.header;
    if (options.footer) this._node.footer = options.footer;

    // ── yCursor tracking for flow-mode text() and textFlow() ─────────────────
    const size = options.size || 'A4';
    const dims = typeof size === 'string' ? (PAGE_SIZES[size] || PAGE_SIZES.A4) : size;
    this._pageWidth  = dims.width;
    this._pageHeight = dims.height;

    const m = options.margin;
    if (typeof m === 'number') {
      this._margin = { top: m, right: m, bottom: m, left: m };
    } else if (m && typeof m === 'object') {
      this._margin = {
        top:    m.top    != null ? m.top    : 72,
        right:  m.right  != null ? m.right  : 72,
        bottom: m.bottom != null ? m.bottom : 72,
        left:   m.left   != null ? m.left   : 72,
      };
    } else {
      this._margin = { top: 72, right: 72, bottom: 72, left: 72 };
    }

    const headerReserve  = options.header ? 30 : 0;
    this._footerReserve  = options.footer ? 30 : 0;
    this._yCursor = this._pageHeight - this._margin.top - headerReserve;
  }

  text(value, options = {}) {
    const textNode = {
      type: NODE_TYPES.TEXT,
      id: genId(),
      value: value,
    };

    if (options.x !== undefined) textNode.x = options.x;
    if (options.maxWidth !== undefined) textNode.maxWidth = options.maxWidth;
    if (options.className) textNode.className = options.className;
    if (options.style) textNode.style = options.style;
    if (options.runs) textNode.runs = options.runs;

    // Flow mode: auto-assign y from cursor and advance it
    if (options.y !== undefined) {
      textNode.y = options.y;
    } else {
      textNode.y = this._yCursor;
      const style = options.style || {};
      const fontSize   = style['font-size']   || 12;
      const lineHeight = style['line-height'] || 1.4;
      const lineStep   = fontSize * lineHeight;

      if (options.maxWidth != null) {
        // Resolve font name to measure accurately
        const fontFamily  = style['font-family'] || 'Helvetica';
        const fontWeight  = style['font-weight'] || 'normal';
        const fontStyle2  = style['font-style']  || 'normal';
        let fontName = fontFamily;
        if (fontWeight === 'bold' && fontStyle2 === 'italic') fontName = fontFamily + '-BoldOblique';
        else if (fontWeight === 'bold') fontName = fontFamily + '-Bold';
        else if (fontStyle2 === 'italic') fontName = fontFamily + '-Oblique';

        const lines = wrapText(String(value), options.maxWidth, fontName, fontSize);
        this._yCursor -= lines.length * lineStep;
      } else {
        this._yCursor -= lineStep;
      }
    }

    this._node.children.push(textNode);
    return this;
  }

  image(options = {}) {
    const imageNode = {
      type: NODE_TYPES.IMAGE,
      id: genId(),
      data: options.data,
    };
    
    if (options.x !== undefined) imageNode.x = options.x;
    if (options.y !== undefined) imageNode.y = options.y;
    if (options.width !== undefined) imageNode.width = options.width;
    if (options.height !== undefined) imageNode.height = options.height;
    if (options.format) imageNode.format = options.format;
    
    this._node.children.push(imageNode);
    return this;
  }

  line(options = {}) {
    const lineNode = {
      type: NODE_TYPES.LINE,
      id: genId(),
      x1: options.x1,
      y1: options.y1,
      x2: options.x2,
      y2: options.y2,
    };
    
    if (options.style) lineNode.style = options.style;
    if (options.width !== undefined) lineNode.width = options.width;
    if (options.color !== undefined) lineNode.color = options.color;
    
    this._node.children.push(lineNode);
    return this;
  }

  rect(options = {}) {
    const rectNode = {
      type: NODE_TYPES.RECT,
      id: genId(),
      x: options.x,
      y: options.y,
      width: options.width,
      height: options.height,
    };
    
    if (options.fill !== undefined) rectNode.fill = options.fill;
    if (options.stroke !== undefined) rectNode.stroke = options.stroke;
    if (options.strokeWidth !== undefined) rectNode.strokeWidth = options.strokeWidth;
    if (options.radius !== undefined) rectNode.radius = options.radius;
    
    this._node.children.push(rectNode);
    return this;
  }

  table(options = {}) {
    const table = new TableBuilder(this, options);
    this._node.children.push(table._node);
    return table;
  }

  absolute(options = {}, children = []) {
    const absNode = {
      type: NODE_TYPES.ABSOLUTE,
      id: genId(),
      children: [],
    };
    
    if (options.x !== undefined) absNode.x = options.x;
    if (options.y !== undefined) absNode.y = options.y;
    if (options.width !== undefined) absNode.width = options.width;
    if (options.zIndex !== undefined) absNode.zIndex = options.zIndex;
    
    this._node.children.push(absNode);
    return new ContainerBuilder(this, absNode);
  }

  float(options = {}) {
    const floatNode = {
      type: NODE_TYPES.FLOAT,
      id: genId(),
      side:   options.side   || 'right',
      margin: options.margin ?? 12,
      width:  options.width  || 120,
      children: [],
    };
    this._node.children.push(floatNode);
    return new ContainerBuilder(this, floatNode);
  }

  columns(options = {}) {
    const colsNode = {
      type: NODE_TYPES.COLUMNS,
      id: genId(),
      children: [],
    };
    if (options.widths !== undefined)  colsNode.widths = options.widths;
    if (options.count  !== undefined)  colsNode.count  = options.count;
    if (options.gap    !== undefined)  colsNode.gap    = options.gap;
    if (options.rule   !== undefined)  colsNode.rule   = options.rule;
    if (options.className)             colsNode.className = options.className;
    this._node.children.push(colsNode);
    return new ColumnsBuilder(this, colsNode);
  }

  container(options = {}) {
    const containerNode = {
      type: NODE_TYPES.CONTAINER,
      id: genId(),
      children: [],
    };
    
    if (options.x !== undefined) containerNode.x = options.x;
    if (options.y !== undefined) containerNode.y = options.y;
    if (options.width !== undefined) containerNode.width = options.width;
    if (options.height !== undefined) containerNode.height = options.height;
    if (options.overflow !== undefined) containerNode.overflow = options.overflow;
    
    this._node.children.push(containerNode);
    return new ContainerBuilder(this, containerNode);
  }

  end() {
    return this._parent;
  }

  toBytes() {
    return this._parent.toBytes();
  }

  toBlob() {
    return this._parent.toBlob();
  }

  toUrl() {
    return this._parent.toUrl();
  }

  addPage(options = {}) {
    return this._parent.addPage(options);
  }
}

class TableBuilder {
  constructor(parent, options = {}) {
    this._parent = parent;
    this._node = {
      type: NODE_TYPES.TABLE,
      id: genId(),
      className: options.className,
      columnWidths: options.columnWidths || [],
      columnAligns: options.columnAligns || [],
      style: options.style || {},
      header: null,
      body: [],
    };
  }

  header(cells, className = 'header') {
    this._node.header = {
      type: NODE_TYPES.ROW,
      id: genId(),
      className: className,
      cells: cells.map((value, i) => ({
        type: NODE_TYPES.CELL,
        id: genId(),
        className: 'cell',
        value: typeof value === 'string' ? value : (value.value || ''),
        ...(typeof value === 'object' ? value : {}),
      })),
    };
    return this;
  }

  row(cells, options = {}) {
    const baseClass = 'row';
    const rowNode = {
      type: NODE_TYPES.ROW,
      id: genId(),
      className: options.className ? `${baseClass} ${options.className}` : baseClass,
      style: options.style,
      cells: cells.map((value, i) => {
        if (typeof value === 'string') {
          return { type: NODE_TYPES.CELL, id: genId(), className: 'cell', value };
        }
        const extra = value.className ? ` ${value.className}` : '';
        return { type: NODE_TYPES.CELL, id: genId(), className: `cell${extra}`, ...value };
      }),
    };
    this._node.body.push(rowNode);
    return this;
  }

  footerRow(cells, className = 'footer') {
    const rowNode = {
      type: NODE_TYPES.ROW,
      id: genId(),
      className: className,
      cells: cells.map((value, i) => {
        if (typeof value === 'string') {
          return { type: NODE_TYPES.CELL, id: genId(), className: 'cell', value };
        }
        const extra = value.className ? ` ${value.className}` : '';
        return { type: NODE_TYPES.CELL, id: genId(), className: `cell${extra}`, ...value };
      }),
    };
    this._node.body.push(rowNode);
    return this;
  }

  end() {
    return this._parent ?? this;
  }

  toBytes() {
    return this._parent ? this._parent.toBytes() : null;
  }

  toBlob() {
    return this._parent ? this._parent.toBlob() : null;
  }

  toUrl() {
    return this._parent ? this._parent.toUrl() : null;
  }

  addPage(options = {}) {
    return this._parent ? this._parent.addPage(options) : null;
  }
}

class ColumnsBuilder {
  constructor(parent, colsNode) {
    this._parent = parent;
    this._node   = colsNode;
  }

  // Each .column() call adds one container child = one column's content area
  column(options = {}) {
    const containerNode = {
      type: NODE_TYPES.CONTAINER,
      id: genId(),
      children: [],
    };
    if (options.style) containerNode.style = options.style;
    this._node.children.push(containerNode);
    return new ContainerBuilder(this, containerNode);
  }

  end()           { return this._parent; }
  toBytes()       { return this._parent.toBytes(); }
  toBlob()        { return this._parent.toBlob(); }
  toUrl()         { return this._parent.toUrl(); }
  addPage(o = {}) { return this._parent.addPage(o); }
}

class ContainerBuilder {
  constructor(parent, containerNode) {
    this._parent = parent;
    this._containerNode = containerNode;
  }

  text(value, options = {}) {
    const textNode = {
      type: NODE_TYPES.TEXT,
      id: genId(),
      value: value,
    };
    
    if (options.x !== undefined) textNode.x = options.x;
    if (options.y !== undefined) textNode.y = options.y;
    if (options.className) textNode.className = options.className;
    if (options.style) textNode.style = options.style;
    
    this._containerNode.children.push(textNode);
    return this;
  }

  image(options = {}) {
    const imageNode = {
      type: NODE_TYPES.IMAGE,
      id: genId(),
      data: options.data,
    };
    
    if (options.x !== undefined) imageNode.x = options.x;
    if (options.y !== undefined) imageNode.y = options.y;
    if (options.width !== undefined) imageNode.width = options.width;
    if (options.height !== undefined) imageNode.height = options.height;
    
    this._containerNode.children.push(imageNode);
    return this;
  }

  rect(options = {}) {
    const rectNode = {
      type: NODE_TYPES.RECT,
      id: genId(),
      x: options.x,
      y: options.y,
      width: options.width,
      height: options.height,
    };

    if (options.fill !== undefined) rectNode.fill = options.fill;
    if (options.stroke !== undefined) rectNode.stroke = options.stroke;
    if (options.strokeWidth !== undefined) rectNode.strokeWidth = options.strokeWidth;
    if (options.radius !== undefined) rectNode.radius = options.radius;

    this._containerNode.children.push(rectNode);
    return this;
  }

  line(options = {}) {
    const lineNode = {
      type: NODE_TYPES.LINE,
      id: genId(),
      x1: options.x1,
      y1: options.y1,
      x2: options.x2,
      y2: options.y2,
    };
    if (options.width !== undefined) lineNode.width = options.width;
    if (options.color !== undefined) lineNode.color = options.color;
    this._containerNode.children.push(lineNode);
    return this;
  }

  table(options = {}) {
    const tbl = new TableBuilder(this, options);
    this._containerNode.children.push(tbl._node);
    return tbl;
  }

  columns(options = {}) {
    const colsNode = {
      type: NODE_TYPES.COLUMNS,
      id: genId(),
      children: [],
    };
    if (options.widths !== undefined)  colsNode.widths = options.widths;
    if (options.count  !== undefined)  colsNode.count  = options.count;
    if (options.gap    !== undefined)  colsNode.gap    = options.gap;
    if (options.rule   !== undefined)  colsNode.rule   = options.rule;
    this._containerNode.children.push(colsNode);
    return new ColumnsBuilder(this, colsNode);
  }

  // Insert vertical whitespace (advances y cursor by `pt` points)
  spacer(pt) {
    const node = { type: NODE_TYPES.TEXT, id: genId(), value: '', _spacer: pt };
    this._containerNode.children.push(node);
    return this;
  }

  end() {
    return this._parent;
  }

  toBytes() {
    return this._parent.toBytes();
  }

  toBlob() {
    return this._parent.toBlob();
  }

  toUrl() {
    return this._parent.toUrl();
  }

  addPage(options = {}) {
    return this._parent.addPage(options);
  }
}

export function text(value, options = {}) {
  return {
    type: NODE_TYPES.TEXT,
    id: genId(),
    value,
    ...options,
  };
}

export function image(options = {}) {
  return {
    type: NODE_TYPES.IMAGE,
    id: genId(),
    ...options,
  };
}

export function line(options = {}) {
  return {
    type: NODE_TYPES.LINE,
    id: genId(),
    ...options,
  };
}

export function rect(options = {}) {
  return {
    type: NODE_TYPES.RECT,
    id: genId(),
    ...options,
  };
}

export function table(options = {}) {
  return {
    type: NODE_TYPES.TABLE,
    id: genId(),
    ...options,
  };
}

export function page(options = {}) {
  return {
    type: NODE_TYPES.PAGE,
    id: genId(),
    ...options,
  };
}
