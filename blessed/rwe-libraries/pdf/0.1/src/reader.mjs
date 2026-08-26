/**
 * L3 – PdfReader (stub)
 *
 * Future implementation: parse xref table, extract object graph, return PdfDocument.
 *
 * Algorithm outline for M6:
 *   1. Scan backward from %%EOF → find startxref offset
 *   2. Parse xref table → Map<id, byteOffset>
 *   3. Parse trailer dict → get /Root id
 *   4. Lazy-parse indirect objects on demand via offset map
 *   5. Walk page tree (/Catalog → /Pages → /Page[]) → extract page sizes
 *   6. Decode content streams → extract text operators (BT...ET blocks)
 */

/**
 * @param {Uint8Array} bytes
 * @returns {never}
 */
export function readPdf(_bytes) {
  throw new Error("PdfReader not implemented (M6)");
}
