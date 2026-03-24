/**
 * L2 – PdfDocument
 *
 * High-level builder. Builds a valid PDF page tree with text content.
 * Uses only the 14 built-in PDF fonts (no embedding required).
 *
 * Coordinate system: PDF origin is bottom-left.
 * A4 = 595 × 842 pt. Use (72, 770) for top-left margin.
 */
import { PdfWriter } from "./writer.mjs";
import { pName, pDict, pArray, pRef, pInt, pStr, pStream } from "./primitives.mjs";

// Standard page sizes in points (1pt = 1/72 inch)
export const PageSize = {
  A4:     [595, 842],
  Letter: [612, 792],
  A3:     [842, 1191],
  A5:     [420, 595],
};

// The 14 built-in PDF Type1 fonts — no embedding needed
export const Font = {
  Helvetica:            "Helvetica",
  HelveticaBold:        "Helvetica-Bold",
  HelveticaOblique:     "Helvetica-Oblique",
  TimesRoman:           "Times-Roman",
  TimesBold:            "Times-Bold",
  Courier:              "Courier",
  CourierBold:          "Courier-Bold",
  Symbol:               "Symbol",
};

class PdfPage {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this._ops = []; // content stream PDF operator strings
    this._font = "Helvetica";
    this._fontSize = 12;
  }

  setFont(fontName, size) {
    this._font = fontName;
    this._fontSize = size;
  }

  /**
   * Draw text at PDF coordinates (x from left, y from bottom).
   * For top-left origin: y = pageHeight - marginTop - fontSize
   */
  drawText(x, y, text) {
    // BT = Begin Text, Tf = set font+size, Td = move cursor, Tj = show string, ET = End Text
    this._ops.push(
      `BT\n/F1 ${this._fontSize} Tf\n${x} ${y} Td\n${pStr(text).toString()} Tj\nET`
    );
  }

  /** Build raw content stream bytes (ASCII) */
  _buildContentBytes() {
    const s = this._ops.join("\n");
    const encoder = new TextEncoder();
    return encoder.encode(s);
  }
}

export class PdfDocument {
  constructor() {
    this._pages = [];
  }

  /**
   * Add a page. Returns a PdfPage to draw on.
   * @param {[number,number]} size  - [width, height] in pt, default A4
   */
  addPage(size = PageSize.A4) {
    const page = new PdfPage(size[0], size[1]);
    this._pages.push(page);
    return page;
  }

  /** Serialize to Uint8Array */
  toBytes() {
    const w = new PdfWriter();

    // --- Object insertion order must satisfy forward-ref constraints:
    //
    //   id 1: font dict
    //   id 2: pages node  ← reserved first so page objects can ref it as parent
    //   id 3..N: content streams + page dicts (interleaved per page)
    //   id N+1: catalog
    //
    // To reserve id 2 for Pages before we know its kids, we use a late-binding
    // content holder: PdfWriter stores the content object reference, not a snapshot.
    // We swap in the real pDict after building page objects.

    // Font resource (built-in Type1, shared across all pages for M0)
    const fontId = w.add(pDict({
      Type:     pName("/Font"),
      Subtype:  pName("/Type1"),
      BaseFont: pName("/Helvetica"),
    }));

    const resources = pDict({
      Font: pDict({ F1: pRef(fontId) }),
    });

    // Reserve the Pages node id with a placeholder — will be replaced below
    const placeholder = pDict({ Type: pName("/Pages") });
    const pagesNodeId = w.add(placeholder);

    // Build content streams + page objects — each page refs pagesNodeId as parent
    const pageObjIds = [];
    for (const page of this._pages) {
      const contentBytes = page._buildContentBytes();
      const contentId = w.add(pStream({}, contentBytes));

      const pageId = w.add(pDict({
        Type:      pName("/Page"),
        Parent:    pRef(pagesNodeId),
        MediaBox:  pArray([pInt(0), pInt(0), pInt(page.width), pInt(page.height)]),
        Contents:  pRef(contentId),
        Resources: resources,
      }));
      pageObjIds.push(pageId);
    }

    // Replace placeholder with real Pages node (now we know all kid ids)
    w._objects[pagesNodeId - 1].content = pDict({
      Type:  pName("/Pages"),
      Kids:  pArray(pageObjIds.map(id => pRef(id))),
      Count: pInt(pageObjIds.length),
    });

    // Catalog (document root)
    const catalogId = w.add(pDict({
      Type:  pName("/Catalog"),
      Pages: pRef(pagesNodeId),
    }));

    return w.serialize(catalogId);
  }

  /** Returns a Blob (application/pdf) */
  toBlob() {
    return new Blob([this.toBytes()], { type: "application/pdf" });
  }

  /** Returns an object URL string — call URL.revokeObjectURL(url) when done */
  toUrl() {
    return URL.createObjectURL(this.toBlob());
  }
}
