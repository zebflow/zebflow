/**
 * L1 – PdfWriter
 *
 * Two-pass serializer:
 *   Pass 1: walk all objects → track byte offsets (used for xref)
 *   Pass 2: allocate one Uint8Array → write header/body/xref/trailer
 *
 * No heap realloc. No temporary strings. No GC pressure.
 */
import { writeStr, pIndirect } from "./primitives.mjs";

const HEADER = "%PDF-1.7\n%\xFF\xFF\xFF\xFF\n"; // 4 high bytes signal binary content to tools

export class PdfWriter {
  constructor() {
    this._objects = []; // [{ id, gen, content }] in insertion order
    this._nextId = 1;
  }

  /** Register an indirect object. Returns its numeric id. */
  add(content) {
    const id = this._nextId++;
    this._objects.push({ id, gen: 0, content });
    return id;
  }

  /** Two-pass serialization → Uint8Array */
  serialize(rootId) {
    const objs = this._objects;
    const offsets = new Array(this._nextId).fill(0); // offsets[id] = byte offset

    // --- Pass 1: compute total size + record offsets ---
    let size = HEADER.length;

    const indirects = objs.map(({ id, gen, content }) => pIndirect(id, gen, content));

    for (let i = 0; i < indirects.length; i++) {
      offsets[objs[i].id] = size;
      size += indirects[i].size() + 1; // +1 for trailing \n
    }

    // xref section
    const xrefOffset = size;
    const xrefCount = this._nextId; // 0..nextId-1
    const xrefSection = buildXref(offsets, xrefCount);
    size += xrefSection.length;

    // trailer
    const trailer = buildTrailer(xrefCount, rootId, xrefOffset);
    size += trailer.length;

    // --- Pass 2: allocate + fill ---
    const buf = new Uint8Array(size);
    let offset = 0;

    offset += writeStr(HEADER, buf, offset);

    for (let i = 0; i < indirects.length; i++) {
      offset += indirects[i].write(buf, offset);
      buf[offset++] = 0x0a; // \n between objects
    }

    offset += writeStr(xrefSection, buf, offset);
    offset += writeStr(trailer, buf, offset);

    return buf;
  }
}

// --- Build the xref table string ---
// Each entry is exactly 20 bytes: "nnnnnnnnnn ggggg f/n \r\n"
// Free head entry (id 0) is always: 0000000000 65535 f \r\n
function buildXref(offsets, count) {
  let s = "xref\n";
  s += `0 ${count}\n`;
  // entry 0: free head
  s += "0000000000 65535 f \r\n";
  for (let id = 1; id < count; id++) {
    const off = String(offsets[id]).padStart(10, "0");
    s += `${off} 00000 n \r\n`;
  }
  return s;
}

function buildTrailer(size, rootId, xrefOffset) {
  return (
    `trailer\n<< /Size ${size} /Root ${rootId} 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`
  );
}
