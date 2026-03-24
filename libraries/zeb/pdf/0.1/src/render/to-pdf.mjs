/**
 * IR → PDF Renderer
 * 
 * Converts the IR tree to PDF bytes using the primitives and writer.
 */
import { PdfWriter } from "../writer.mjs";
import {
  pName, pDict, pArray, pRef, pInt, pReal, pStr, pStream, pIndirect,
  writeStr, writeBytes,
} from "../primitives.mjs";
import { validateNode, getPageDimensions, getEffectiveMargin, NODE_TYPES } from "../ir/nodes.mjs";
import { StyleEngine, colorToRgb, parseColor } from "../ir/stylesheet.mjs";
import { measureTextWidth, wrapText } from "../ir/layout.mjs";

const FONT_MAP = {
  'Helvetica': '/Helvetica',
  'Helvetica-Bold': '/Helvetica-Bold',
  'Helvetica-Oblique': '/Helvetica-Oblique',
  'Helvetica-BoldOblique': '/Helvetica-BoldOblique',
  'Times-Roman': '/Times-Roman',
  'Times-Bold': '/Times-Bold',
  'Times-Italic': '/Times-Italic',
  'Times-BoldItalic': '/Times-BoldItalic',
  'Courier': '/Courier',
  'Courier-Bold': '/Courier-Bold',
  'Courier-Oreliq': '/Courier-Oreliq',
  'Courier-BoldOblique': '/Courier-BoldOblique',
  'Symbol': '/Symbol',
  'ZapfDingbats': '/ZapfDingbats',
};

const BUILT_IN_FONTS = [
  'Helvetica', 'Helvetica-Bold', 'Helvetica-Oblique', 'Helvetica-BoldOblique',
  'Times-Roman', 'Times-Bold', 'Times-Italic', 'Times-BoldItalic',
  'Courier', 'Courier-Bold', 'Courier-Oblique', 'Courier-BoldOblique',
  'Symbol', 'ZapfDingbats',
];

export class IrRenderer {
  constructor() {
    this._fontResources = new Map();
    this._imageResources = new Map();
    this._nextFontId = 1;
    this._nextImageId = 1;
    this._writtenImageIds = new Set();
    this.writer = null;
  }

  getFontId(fontName) {
    if (!this._fontResources.has(fontName)) {
      const id = this._nextFontId++;
      const pdfFontName = FONT_MAP[fontName] || `/${fontName}`;
      this._fontResources.set(fontName, { id, pdfName: pdfFontName });
    }
    return this._fontResources.get(fontName);
  }

  getImageId(imageData) {
    const key = imageData instanceof Uint8Array
      ? `bin_${imageData.length}`
      : imageData.slice(0, 100);
    
    if (!this._imageResources.has(key)) {
      const id = this._nextImageId++;
      this._imageResources.set(key, { id, data: imageData });
    }
    return this._imageResources.get(key);
  }

  measureText(text, fontFamily, fontSize) {
    return {
      width: measureTextWidth(text, fontFamily, fontSize),
      height: fontSize * 1.2,
    };
  }

  parseImageData(data) {
    if (typeof data === 'string') {
      const match = data.match(/^data:image\/(\w+);base64,(.+)$/);
      if (match) {
        const format = match[1];
        const base64 = match[2];
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }
        return { bytes, format };
      }
    }
    return { bytes: data instanceof Uint8Array ? data : new Uint8Array(), format: 'png' };
  }

  renderDocument(irDoc) {
    validateNode(irDoc);
    
    const writer = new PdfWriter();
    this.writer = writer;
    const context = {
      document: irDoc,
      settings: irDoc.settings || {},
      styles: irDoc.styles || {},
      styleEngine: new StyleEngine(irDoc.styles || {}),
      renderer: this,
      writer,
    };

    const placeholder = pDict({ Type: pName("/Pages") });
    const pagesNodeId = writer.add(placeholder);

    const pageObjIds = [];
    let pageIndex = 0;
    const totalPages = irDoc.children ? irDoc.children.filter(c => c.type === NODE_TYPES.PAGE).length : 0;

    for (const child of (irDoc.children || [])) {
      if (child.type === NODE_TYPES.PAGE) {
        const pageResult = this.renderPage(child, context, pagesNodeId, pageIndex, totalPages);
        pageObjIds.push(pageResult.pageId);
        pageIndex++;
      }
    }

    writer._objects[pagesNodeId - 1].content = pDict({
      Type: pName("/Pages"),
      Kids: pArray(pageObjIds.map(id => pRef(id))),
      Count: pInt(pageObjIds.length),
    });

    const catalogId = writer.add(pDict({
      Type: pName("/Catalog"),
      Pages: pRef(pagesNodeId),
    }));

    if (irDoc.meta) {
      const infoEntries = [];
      if (irDoc.meta.title) infoEntries.push(['Title', pStr(irDoc.meta.title)]);
      if (irDoc.meta.author) infoEntries.push(['Author', pStr(irDoc.meta.author)]);
      if (irDoc.meta.subject) infoEntries.push(['Subject', pStr(irDoc.meta.subject)]);
      if (irDoc.meta.keywords) infoEntries.push(['Keywords', pStr(irDoc.meta.keywords.join(', '))]);
      if (irDoc.meta.creator) infoEntries.push(['Creator', pStr(irDoc.meta.creator)]);
      if (irDoc.meta.producer) infoEntries.push(['Producer', pStr(irDoc.meta.producer)]);
      
      if (infoEntries.length > 0) {
        writer.add(pDict(Object.fromEntries(infoEntries)));
      }
    }

    return writer.serialize(catalogId);
  }

  renderPage(pageNode, context, parentPagesId, pageIndex, totalPages) {
    const { width, height } = getPageDimensions(pageNode, context.settings);
    const margin = getEffectiveMargin(pageNode, context.settings);
    
    const effectiveWidth = width - margin.left - margin.right;
    const effectiveHeight = height - margin.top - margin.bottom;
    
    const contentOps = [];
    let yCursor = height - margin.top;

    const pageAncestors = [
      { node: context.document, sibIdx: 0, sibCount: 1 },
      { node: pageNode, sibIdx: pageIndex, sibCount: totalPages },
    ];

    // Resolve page-level style and draw background fill if set
    const pageStyle = context.styleEngine.resolve(pageNode, [{ node: context.document, sibIdx: 0, sibCount: 1 }], pageIndex, totalPages);
    const pageBg = pageStyle['background-color'];
    if (pageBg && pageBg !== 'transparent' && pageBg !== '#ffffff') {
      const rgb = colorToRgb(pageBg);
      contentOps.push(`${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} rg`);
      contentOps.push(`0 0 ${width} ${height} re f`);
    }

    if (pageNode.header) {
      const headerResult = this.renderHeaderFooter(pageNode.header, context, margin, width, height, 'header', pageIndex, totalPages);
      contentOps.push(headerResult.ops);
      yCursor -= headerResult.height;
    }

    if (pageNode.footer) {
      context._footerMargin = margin.bottom;
      context._footerHeight = 30;
    }

    const contentHeight = pageNode.footer
      ? yCursor - margin.bottom - context._footerHeight
      : yCursor - margin.bottom;

    // Node types that always participate in page flow (advance yCursor when rendered)
    const FLOW_TYPES = new Set(['container', 'columns', 'table', 'float', 'relative']);

    const pageChildren = pageNode.children || [];
    for (let ci = 0; ci < pageChildren.length; ci++) {
      const child = pageChildren[ci];
      const result = this.renderNode(child, context, {
        x: margin.left,
        y: yCursor,
        width: effectiveWidth,
        height: contentHeight,
        pageWidth: width,
        pageHeight: height,
        margin,
      }, pageAncestors, ci, pageChildren.length);
      if (result) {
        contentOps.push(result.ops);
        // Only advance yCursor for flow nodes: structural flow types (container, columns, table…)
        // OR leaf nodes explicitly tagged _flow:true (nodes from textFlow/tableFlow).
        // Positioned decorations (rect, text/line/image with user-set coords) don't advance the cursor.
        const isFlowNode = FLOW_TYPES.has(child.type) || child._flow === true;
        if (isFlowNode && result.y !== undefined) {
          yCursor = result.y;
        }
      }
    }

    if (pageNode.footer) {
      const footerResult = this.renderHeaderFooter(pageNode.footer, context, margin, width, height, 'footer', pageIndex, totalPages);
      contentOps.push(footerResult.ops);
    }

    const contentBytes = new TextEncoder().encode(contentOps.join('\n'));
    const contentId = context.renderer.writer.add(pStream({}, contentBytes));
    const pageId = context.renderer.writer.add(pDict({
      Type: pName("/Page"),
      Parent: pRef(parentPagesId),
      MediaBox: pArray([pInt(0), pInt(0), pInt(width), pInt(height)]),
      Contents: pRef(contentId),
      Resources: context.renderer._buildResources(),
    }));

    return { pageId, contentId };
  }

  _buildResources() {
    const fontDictEntries = [];
    for (const [fontName, fontInfo] of this._fontResources) {
      fontDictEntries.push([`F${fontInfo.id}`, pDict({
        Type: pName('/Font'),
        Subtype: pName('/Type1'),
        BaseFont: pName(fontInfo.pdfName),
        Encoding: pName('/WinAnsiEncoding'),
      })]);
    }

    const imageXObjectEntries = [];
    for (const [key, imgInfo] of this._imageResources) {
      const imgId = imgInfo.id;
      if (!this._writtenImageIds.has(imgId)) {
        const { bytes, format } = this.parseImageData(imgInfo.data);
        const filter = format === 'jpeg' ? [pName('/DCTDecode')] : [];

        const imgDict = {
          Type: pName('/XObject'),
          Subtype: pName('/Image'),
          Width: pInt(imgInfo.width || 100),
          Height: pInt(imgInfo.height || 100),
          ColorSpace: pName('/DeviceRGB'),
          BitsPerComponent: pInt(8),
        };

        if (filter.length > 0) {
          imgDict.Filter = pArray(filter);
        }

        const pdfId = this.writer.add(pStream(imgDict, bytes));
        imgInfo.pdfId = pdfId;
        this._writtenImageIds.add(imgId);
      }
      if (imgInfo.pdfId !== undefined) {
        imageXObjectEntries.push([`I${imgInfo.id}`, pRef(imgInfo.pdfId)]);
      }
    }

    const resources = {};
    if (fontDictEntries.length > 0 || this._fontResources.size === 0) {
      resources.Font = pDict(Object.fromEntries(fontDictEntries.length ? fontDictEntries : [[ 'F1', pDict({ Type: pName('/Font'), Subtype: pName('/Type1'), BaseFont: pName('/Helvetica') }) ]]));
    }
    if (imageXObjectEntries.length > 0) {
      resources.XObject = pDict(Object.fromEntries(imageXObjectEntries));
    }

    return pDict(resources);
  }

  renderNode(node, context, layout, ancestors = [], sibIdx = 0, sibCount = 1) {
    const style = context.styleEngine.resolve(node, ancestors, sibIdx, sibCount);

    switch (node.type) {
      case NODE_TYPES.TEXT:
        return this.renderText(node, context, layout, style);
      case NODE_TYPES.IMAGE:
        return this.renderImage(node, context, layout, style);
      case NODE_TYPES.LINE:
        return this.renderLine(node, context, layout, style);
      case NODE_TYPES.RECT:
        return this.renderRect(node, context, layout, style);
      case NODE_TYPES.TABLE:
        return this.renderTable(node, context, layout, style, ancestors, sibIdx, sibCount);
      case NODE_TYPES.ABSOLUTE:
        return this.renderAbsolute(node, context, layout, style, ancestors, sibIdx, sibCount);
      case NODE_TYPES.FLOAT:
        return this.renderFloat(node, context, layout, style, ancestors, sibIdx, sibCount);
      case NODE_TYPES.CONTAINER:
        return this.renderContainer(node, context, layout, style, ancestors, sibIdx, sibCount);
      case NODE_TYPES.COLUMNS:
        return this.renderColumns(node, context, layout, style, ancestors, sibIdx, sibCount);
      default:
        return null;
    }
  }

  renderText(textNode, context, layout, style) {
    const fontFamily = style['font-family'] || 'Helvetica';
    const fontSize = style['font-size'] || 12;
    const fontWeight = style['font-weight'] || 'normal';
    const fontStyle = style['font-style'] || 'normal';
    const lineHeight = style['line-height'] || 1.4;
    const textColor = style['color'] || '#000000';

    let fullFontName = fontFamily;
    if (fontWeight === 'bold' && fontStyle === 'italic' && !fontFamily.includes('Bold')) {
      fullFontName = fontFamily + '-BoldOblique';
    } else if (fontWeight === 'bold' && !fontFamily.includes('Bold')) {
      fullFontName = fontFamily + '-Bold';
    } else if (fontStyle === 'italic' && !fontFamily.includes('Oblique') && !fontFamily.includes('Italic')) {
      fullFontName = fontFamily + '-Oblique';
    }

    const fontInfo = this.getFontId(fullFontName);
    const fontId = `F${fontInfo.id}`;

    const x = textNode.x !== undefined ? textNode.x : layout.x;
    const startY = textNode.y !== undefined ? textNode.y : layout.y;
    const lineStep = fontSize * lineHeight;

    const ops = [];

    // ── Runs mode (mixed-style inline text) ──────────────────────────────────
    if (textNode.runs) {
      let y = startY;
      for (const run of textNode.runs) {
        const runStyle = run.style || {};
        const runFont = runStyle['font-family'] || fontFamily;
        const runFontSize = runStyle['font-size'] || fontSize;
        const runFontInfo = this.getFontId(runFont);
        const rgb = colorToRgb(runStyle['color'] || textColor);
        ops.push(`BT`);
        ops.push(`/F${runFontInfo.id} ${runFontSize} Tf`);
        ops.push(`${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} rg`);
        ops.push(`${x} ${y} Td`);
        ops.push(`(${this.escapePdfString(run.text)}) Tj`);
        ops.push(`ET`);
      }
      return { ops: ops.join('\n'), y };
    }

    // ── Single-value text (with optional maxWidth wrapping) ──────────────────
    const text = textNode.value || '';
    const rgb = colorToRgb(textColor);

    // Determine wrapping: use textNode.maxWidth if set, else auto-wrap to layout.width
    const maxWidth = textNode.maxWidth != null ? textNode.maxWidth : layout.width;
    const lines = maxWidth != null
      ? wrapText(text, maxWidth, fullFontName, fontSize)
      : [text];

    let y = startY;
    for (const line of lines) {
      ops.push(`BT`);
      ops.push(`/${fontId} ${fontSize} Tf`);
      ops.push(`${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} rg`);
      ops.push(`${x} ${y} Td`);
      ops.push(`(${this.escapePdfString(line)}) Tj`);
      ops.push(`ET`);
      y -= lineStep;
    }

    return { ops: ops.join('\n'), y };
  }

  renderImage(imageNode, context, layout, style) {
    const imageData = imageNode.data;
    const imgInfo = this.getImageId(imageData);
    
    let width = imageNode.width || 100;
    let height = imageNode.height || 100;
    
    if (imgInfo.width && imgInfo.height) {
      width = imgInfo.width;
      height = imgInfo.height;
    }

    const x = imageNode.x !== undefined ? imageNode.x : layout.x;
    const y = imageNode.y !== undefined ? imageNode.y : layout.y;

    const ops = [
      `q`,
      `${width} 0 0 ${height} ${x} ${y} cm`,
      `/I${imgInfo.id} Do`,
      `Q`,
    ];

    return { ops: ops.join('\n'), y: y - height };
  }

  renderLine(lineNode, context, layout, style) {
    const x1 = lineNode.x1;
    const y1 = lineNode.y1 !== undefined ? lineNode.y1 : layout.y;
    const x2 = lineNode.x2;
    const y2 = lineNode.y2 !== undefined ? lineNode.y2 : layout.y;
    const lineWidth = lineNode.width || style['stroke-width'] || 1;
    const lineColor = lineNode.color || style['stroke'] || '#000000';
    const lineStyle = lineNode.style || style['stroke-style'] || 'solid';

    const rgb = colorToRgb(lineColor);
    let dashPattern = '';
    if (lineStyle === 'dashed') {
      dashPattern = `[${lineWidth * 3} ${lineWidth * 2}] 0 d`;
    } else if (lineStyle === 'dotted') {
      dashPattern = `[${lineWidth} ${lineWidth}] 0 d`;
    }

    const ops = [
      `${lineWidth} w`,
      dashPattern,
      `${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} RG`,
      `${x1} ${y1} m`,
      `${x2} ${y2} l`,
      `S`,
    ].filter(Boolean).join('\n');

    // Flow divider: y1/y2 not specified → advance cursor past the line
    const isFlowDivider = lineNode.y1 === undefined && lineNode.y2 === undefined;
    const retY = isFlowDivider ? y1 - lineWidth - 6 : Math.min(y1, y2);
    return { ops, y: retY };
  }

  renderRect(rectNode, context, layout, style) {
    const x = rectNode.x;
    const y = rectNode.y;
    const width = rectNode.width;
    const height = rectNode.height;
    const fillColor = rectNode.fill !== undefined ? rectNode.fill : style['fill'];
    const strokeColor = rectNode.stroke !== undefined ? rectNode.stroke : style['stroke'];
    const strokeWidth = rectNode.strokeWidth !== undefined ? rectNode.strokeWidth : (style['stroke-width'] || 1);
    const radius = rectNode.radius || 0;

    const ops = [];

    if (fillColor && fillColor !== 'none') {
      const rgb = colorToRgb(fillColor);
      ops.push(`${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} rg`);
    }

    if (strokeColor && strokeColor !== 'none') {
      const rgb = colorToRgb(strokeColor);
      ops.push(`${rgb[0].toFixed(3)} ${rgb[1].toFixed(3)} ${rgb[2].toFixed(3)} RG`);
      ops.push(`${strokeWidth} w`);
    }

    if (radius > 0) {
      ops.push(`${x + radius} ${y} m`);
      ops.push(`${x + width - radius} ${y} l`);
      ops.push(`${x + width} ${y + radius} l`);
      ops.push(`${x + width} ${y + height - radius} l`);
      ops.push(`${x + width - radius} ${y + height} l`);
      ops.push(`${x + radius} ${y + height} l`);
      ops.push(`${x} ${y + height - radius} l`);
      ops.push(`${x} ${y + radius} l`);
      ops.push(`${x + radius} ${y} l`);
      ops.push(`h`);
    } else {
      ops.push(`${x} ${y} ${width} ${height} re`);
    }

    if (fillColor && fillColor !== 'none' && strokeColor && strokeColor !== 'none') {
      ops.push(`B`);
    } else if (fillColor && fillColor !== 'none') {
      ops.push(`f`);
    } else if (strokeColor && strokeColor !== 'none') {
      ops.push(`S`);
    }

    return { ops: ops.join('\n'), y: y - height };
  }

  renderTable(tableNode, context, layout, style, ancestors = [], sibIdx = 0, sibCount = 1) {
    const engine = context.styleEngine;
    const columnWidths = tableNode.columnWidths || [];
    const ops = [];
    let y = layout.y;

    const tableBorderWidth = parseFloat(style['border-width']) || 0;
    const tableBorderColor = style['border-color'] || '#000000';
    const tableRgb = colorToRgb(tableBorderColor);

    // Ancestor entry for this table (used by row/cell selectors)
    const tableEntry = { node: tableNode, sibIdx, sibCount };
    const tableAncestors = [...ancestors, tableEntry];

    // ── Header row ──────────────────────────────────────────────────────────
    const headerRow = tableNode.header;
    if (headerRow) {
      const headerEntry = { node: headerRow, sibIdx: 0, sibCount: 1 };
      const headerAncestors = [...tableAncestors, headerEntry];
      const headerStyle = engine.resolve(headerRow, tableAncestors, 0, 1);

      const cells = headerRow.cells || [];
      let x = layout.x;
      let colIdx = 0;

      for (let i = 0; i < cells.length; i++) {
        const cell = cells[i];
        const cellStyle = engine.resolve(cell, headerAncestors, i, cells.length);
        const span = cell.colspan || 1;
        let cellWidth = 0;
        for (let s = 0; s < span; s++) cellWidth += columnWidths[colIdx + s] || 100;
        const cellHeight = parseFloat(cellStyle['height']) || 22;
        const bgColor    = cellStyle['background-color'] || headerStyle['background-color'];

        if (bgColor && bgColor !== 'transparent' && bgColor !== '#ffffff') {
          const bgRgb = colorToRgb(bgColor);
          ops.push(`${bgRgb[0].toFixed(3)} ${bgRgb[1].toFixed(3)} ${bgRgb[2].toFixed(3)} rg`);
          ops.push(`${x} ${y - cellHeight} ${cellWidth} ${cellHeight} re f`);
        }

        const bw = parseFloat(cellStyle['border-width'] ?? tableBorderWidth) || 0;
        if (bw > 0) {
          const bc = cellStyle['border-color'] || tableBorderColor;
          const br = colorToRgb(bc);
          ops.push(`${bw} w`);
          ops.push(`${br[0].toFixed(3)} ${br[1].toFixed(3)} ${br[2].toFixed(3)} RG`);
          ops.push(`${x} ${y - cellHeight} ${cellWidth} ${cellHeight} re S`);
        }

        const textVal = cell.value || '';
        if (textVal) {
          const fw = cellStyle['font-weight'] || headerStyle['font-weight'] || 'bold';
          const ff = cellStyle['font-family'] || 'Helvetica';
          const fs = parseFloat(cellStyle['font-size']) || 11;
          const fc = cellStyle['color'] || headerStyle['color'] || '#000000';
          const fontName = fw === 'bold' ? ff + '-Bold' : ff;
          const fontInfo = this.getFontId(fontName);
          const fcRgb = colorToRgb(fc);

          const ta = cellStyle['text-align'] || tableNode.columnAligns?.[colIdx] || 'left';
          const pl = parseFloat(cellStyle['padding-left'] ?? cellStyle['padding']) || 4;
          const pr = parseFloat(cellStyle['padding-right'] ?? cellStyle['padding']) || 4;
          let textX;
          if (ta === 'right') {
            const tw = measureTextWidth(textVal, fontName, fs);
            textX = x + cellWidth - pr - tw;
          } else if (ta === 'center') {
            const tw = measureTextWidth(textVal, fontName, fs);
            textX = x + (cellWidth - tw) / 2;
          } else {
            textX = x + pl;
          }

          const pt = parseFloat(cellStyle['padding-top'] ?? cellStyle['padding']) || 4;
          const pb = parseFloat(cellStyle['padding-bottom'] ?? cellStyle['padding']) || 4;
          const va = cellStyle['vertical-align'] || 'middle';
          let textY;
          if      (va === 'top')    textY = y - pt - fs;
          else if (va === 'bottom') textY = y - cellHeight + pb;
          else                      textY = y - cellHeight / 2 - fs * 0.35;

          ops.push(`BT`);
          ops.push(`/F${fontInfo.id} ${fs} Tf`);
          ops.push(`${fcRgb[0].toFixed(3)} ${fcRgb[1].toFixed(3)} ${fcRgb[2].toFixed(3)} rg`);
          ops.push(`${textX} ${textY} Td`);
          ops.push(`(${this.escapePdfString(textVal)}) Tj`);
          ops.push(`ET`);
        }
        x += cellWidth;
        colIdx += span;
      }
      y -= (parseFloat(headerStyle['height']) || 22);
    }

    // ── Body rows ────────────────────────────────────────────────────────────
    const body = tableNode.body || [];
    for (let rowIdx = 0; rowIdx < body.length; rowIdx++) {
      const row = body[rowIdx];
      const rowEntry = { node: row, sibIdx: rowIdx, sibCount: body.length };
      const rowAncestors = [...tableAncestors, rowEntry];
      const rowStyle = engine.resolve(row, tableAncestors, rowIdx, body.length);

      const cells = row.cells || [];
      let x = layout.x;
      let colIdx = 0;

      for (let i = 0; i < cells.length; i++) {
        const cell = cells[i];
        const cellStyle = engine.resolve(cell, rowAncestors, i, cells.length);
        const span = cell.colspan || 1;
        let cellWidth = 0;
        for (let s = 0; s < span; s++) cellWidth += columnWidths[colIdx + s] || 100;
        const cellHeight = parseFloat(cellStyle['height'] || rowStyle['height']) || 20;
        const bgColor    = cellStyle['background-color'] || rowStyle['background-color'];

        if (bgColor && bgColor !== 'transparent' && bgColor !== '#ffffff') {
          const bgRgb = colorToRgb(bgColor);
          ops.push(`${bgRgb[0].toFixed(3)} ${bgRgb[1].toFixed(3)} ${bgRgb[2].toFixed(3)} rg`);
          ops.push(`${x} ${y - cellHeight} ${cellWidth} ${cellHeight} re f`);
        }

        const bw = parseFloat(cellStyle['border-width'] ?? tableBorderWidth) || 0;
        if (bw > 0) {
          const bc = cellStyle['border-color'] || tableBorderColor;
          const br = colorToRgb(bc);
          ops.push(`${bw} w`);
          ops.push(`${br[0].toFixed(3)} ${br[1].toFixed(3)} ${br[2].toFixed(3)} RG`);
          ops.push(`${x} ${y - cellHeight} ${cellWidth} ${cellHeight} re S`);
        }

        const textVal = cell.value || '';
        if (textVal) {
          const fw = cellStyle['font-weight'] || 'normal';
          const ff = cellStyle['font-family'] || 'Helvetica';
          const fs = parseFloat(cellStyle['font-size']) || 11;
          const fc = cellStyle['color'] || '#000000';
          const fontName = fw === 'bold' ? ff + '-Bold' : ff;
          const fontInfo = this.getFontId(fontName);
          const fcRgb = colorToRgb(fc);

          const ta = cellStyle['text-align'] || tableNode.columnAligns?.[colIdx] || 'left';
          const pl = parseFloat(cellStyle['padding-left'] ?? cellStyle['padding']) || 4;
          const pr = parseFloat(cellStyle['padding-right'] ?? cellStyle['padding']) || 4;
          let textX;
          if (ta === 'right') {
            const tw = measureTextWidth(textVal, fontName, fs);
            textX = x + cellWidth - pr - tw;
          } else if (ta === 'center') {
            const tw = measureTextWidth(textVal, fontName, fs);
            textX = x + (cellWidth - tw) / 2;
          } else {
            textX = x + pl;
          }

          const pt = parseFloat(cellStyle['padding-top'] ?? cellStyle['padding']) || 4;
          const pb = parseFloat(cellStyle['padding-bottom'] ?? cellStyle['padding']) || 4;
          const va = cellStyle['vertical-align'] || 'middle';
          let textY;
          if      (va === 'top')    textY = y - pt - fs;
          else if (va === 'bottom') textY = y - cellHeight + pb;
          else                      textY = y - cellHeight / 2 - fs * 0.35;

          ops.push(`BT`);
          ops.push(`/F${fontInfo.id} ${fs} Tf`);
          ops.push(`${fcRgb[0].toFixed(3)} ${fcRgb[1].toFixed(3)} ${fcRgb[2].toFixed(3)} rg`);
          ops.push(`${textX} ${textY} Td`);
          ops.push(`(${this.escapePdfString(textVal)}) Tj`);
          ops.push(`ET`);
        }
        x += cellWidth;
        colIdx += span;
      }
      y -= (parseFloat(rowStyle['height']) || 20);
    }

    return { ops: ops.join('\n'), y };
  }

  renderAbsolute(absNode, context, layout, style, ancestors = [], sibIdx = 0, sibCount = 1) {
    const x = absNode.x !== undefined ? absNode.x : layout.x;
    const ops = [];
    const childAncestors = [...ancestors, { node: absNode, sibIdx, sibCount }];
    const children = absNode.children || [];
    let y = absNode.y !== undefined ? absNode.y : layout.y;

    for (let i = 0; i < children.length; i++) {
      const result = this.renderNode(children[i], context, { ...layout, x, y }, childAncestors, i, children.length);
      if (result) {
        ops.push(result.ops);
        if (result.y !== undefined) y = result.y;
      }
    }

    return { ops: ops.join('\n'), y };
  }

  renderFloat(floatNode, context, layout, style, ancestors = [], sibIdx = 0, sibCount = 1) {
    const side       = floatNode.side || 'right';
    const floatWidth = floatNode.width || 120;

    const x = (side === 'right' || side === 'end')
      ? layout.x + layout.width - floatWidth
      : layout.x;

    const ops = [];
    const childAncestors = [...ancestors, { node: floatNode, sibIdx, sibCount }];
    const children = floatNode.children || [];
    let y = layout.y;

    for (let i = 0; i < children.length; i++) {
      const result = this.renderNode(
        children[i], context,
        { ...layout, x, width: floatWidth, y },
        childAncestors, i, children.length
      );
      if (result) {
        ops.push(result.ops);
        if (result.y !== undefined) y = result.y;
      }
    }

    return { ops: ops.join('\n'), y };
  }

  renderColumns(colsNode, context, layout, style, ancestors = [], sibIdx = 0, sibCount = 1) {
    const gap      = colsNode.gap  ?? 12;
    const rule     = colsNode.rule ?? false;
    const children = colsNode.children || [];
    const n        = colsNode.widths
      ? colsNode.widths.length
      : (colsNode.count || children.length || 1);

    // Compute per-column widths
    let colWidths;
    if (colsNode.widths && colsNode.widths.length > 0) {
      colWidths = colsNode.widths;
    } else {
      const colW = (layout.width - gap * (n - 1)) / n;
      colWidths = Array.from({ length: n }, () => colW);
    }

    const ops = [];
    const colAncestors = [...ancestors, { node: colsNode, sibIdx, sibCount }];
    let minY = layout.y; // lowest y reached across all columns

    let xCursor = layout.x;
    for (let i = 0; i < children.length; i++) {
      const w = colWidths[i] ?? colWidths[colWidths.length - 1];
      const colLayout = { ...layout, x: xCursor, width: w, y: layout.y };
      const result = this.renderNode(children[i], context, colLayout, colAncestors, i, children.length);
      if (result) {
        ops.push(result.ops);
        if (result.y !== undefined && result.y < minY) minY = result.y;
      }

      // Draw column rule between columns
      if (rule && i < children.length - 1) {
        const ruleX = xCursor + w + gap / 2;
        ops.push(`0.5 w`);
        ops.push(`0.75 0.75 0.75 RG`);
        ops.push(`${ruleX.toFixed(2)} ${minY.toFixed(2)} m`);
        ops.push(`${ruleX.toFixed(2)} ${layout.y.toFixed(2)} l`);
        ops.push(`S`);
      }

      xCursor += w + gap;
    }

    return { ops: ops.join('\n'), y: minY };
  }

  renderContainer(containerNode, context, layout, style, ancestors = [], sibIdx = 0, sibCount = 1) {
    const ops = [];
    const childAncestors = [...ancestors, { node: containerNode, sibIdx, sibCount }];
    const children = containerNode.children || [];

    // Apply padding from container style
    const cs = containerNode.style || {};
    const pl = parseFloat(cs['padding-left']  ?? cs['padding']) || 0;
    const pr = parseFloat(cs['padding-right'] ?? cs['padding']) || 0;
    const pt = parseFloat(cs['padding-top']   ?? cs['padding']) || 0;

    const childLayout = {
      ...layout,
      x:     layout.x + pl,
      width: layout.width - pl - pr,
      y:     layout.y - pt,
    };
    let y = childLayout.y;

    for (let i = 0; i < children.length; i++) {
      const child = children[i];
      // Spacer node: just advance y by the requested amount
      if (child._spacer) { y -= child._spacer; continue; }
      childLayout.y = y;
      const result = this.renderNode(child, context, childLayout, childAncestors, i, children.length);
      if (result) {
        ops.push(result.ops);
        if (result.y !== undefined) y = result.y;
      }
    }

    return { ops: ops.join('\n'), y };
  }

  renderHeaderFooter(template, context, margin, pageWidth, pageHeight, type, pageIndex = 0, totalPages = 0) {
    const ops = [];
    const height = 30;

    const templateStr = template.template || (type === 'footer' ? 'Page {page} of {total}' : '');
    const text = templateStr
      .replace('{page}', String(pageIndex + 1))
      .replace('{total}', String(totalPages));

    const align = template.align || 'center';
    const fontSize = 10;
    const fontInfo = this.getFontId('Helvetica');

    let y;
    if (type === 'footer') {
      y = margin.bottom;
    } else {
      // header: place in top margin zone, 10pt above content boundary
      y = pageHeight - margin.top + 10;
    }

    let x;
    if (align === 'right') {
      x = pageWidth - margin.right - 50;
    } else if (align === 'left') {
      x = margin.left;
    } else {
      // center: measure text width to truly center it
      const textWidth = measureTextWidth(text, 'Helvetica', fontSize);
      x = (pageWidth - textWidth) / 2;
    }

    ops.push(`BT`);
    ops.push(`/F${fontInfo.id} ${fontSize} Tf`);
    ops.push(`0 0 0 rg`);
    ops.push(`${x} ${y} Td`);
    ops.push(`(${this.escapePdfString(text)}) Tj`);
    ops.push(`ET`);

    return { ops: ops.join('\n'), height };
  }

  escapePdfString(str) {
    if (!str) return '';
    // Unicode → WinAnsi (Windows-1252) mapping for chars in 0x80–0x9F that differ from Latin-1
    const WINANS = {
      0x2013: 0x96, // en dash –
      0x2014: 0x97, // em dash —
      0x2018: 0x91, // left single quote '
      0x2019: 0x92, // right single quote '
      0x201C: 0x93, // left double quote "
      0x201D: 0x94, // right double quote "
      0x2022: 0x95, // bullet •
      0x2026: 0x85, // ellipsis …
      0x0152: 0x8C, // OE ligature Œ
      0x0153: 0x9C, // oe ligature œ
      0x2039: 0x8B, // ‹
      0x203A: 0x9B, // ›
    };
    let out = '';
    for (const ch of str) {
      const code = ch.codePointAt(0);
      if (code === 0x5C)       { out += '\\\\'; }
      else if (code === 0x28)  { out += '\\('; }
      else if (code === 0x29)  { out += '\\)'; }
      else if (code === 0x0A)  { out += '\\n'; }
      else if (code === 0x0D)  { out += '\\r'; }
      else if (code === 0x09)  { out += '\\t'; }
      else if (code < 128)     { out += ch; }
      else if (WINANS[code] !== undefined) {
        out += `\\${WINANS[code].toString(8).padStart(3, '0')}`;
      } else if (code <= 255)  {
        // Latin-1 range — pass as octal
        out += `\\${code.toString(8).padStart(3, '0')}`;
      } else {
        out += '?'; // beyond WinAnsi — substitute
      }
    }
    return out;
  }
}

export async function render(irDoc) {
  const renderer = new IrRenderer();
  return renderer.renderDocument(irDoc);
}

export function renderSync(irDoc) {
  const renderer = new IrRenderer();
  return renderer.renderDocument(irDoc);
}
