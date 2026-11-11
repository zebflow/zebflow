// Images for the PDF writer: the bytes behind an image node, what they are, and how a PDF shows them.
//
// The format comes from the bytes, never from a name: a JPEG goes in as it is (/DCTDecode); a PNG without
// transparency goes in as its own compressed data with the PNG predictor (/FlateDecode); a PNG with
// transparency is decoded here, colour and alpha apart, because PDF keeps alpha in a separate /SMask image.
// Everything is synchronous, like the rest of the writer.

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

// The bytes of an image node's `data`: a base64 data URL, a Uint8Array, an ArrayBuffer or an array of byte values.
export function imageBytes(data) {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (Array.isArray(data)) return Uint8Array.from(data);
  if (typeof data === "string") {
    const m = /^data:[^,;]*(;[^,]*)?,(.*)$/s.exec(data);
    if (!m || !/;base64/i.test(m[1] || "")) throw new Error("image data must be a base64 data URL, a Uint8Array, an ArrayBuffer or an array of bytes");
    const binary = atob(m[2].replace(/\s+/g, ""));
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  }
  throw new Error("image data must be a base64 data URL, a Uint8Array, an ArrayBuffer or an array of bytes");
}

// A key for the whole content: equal keys are then checked byte by byte, so two images never merge by accident.
export function fingerprint(bytes) {
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i++) h = Math.imul(h ^ bytes[i], 0x01000193);
  return bytes.length + ":" + (h >>> 0).toString(16);
}

export function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

function sniff(b) {
  if (b.length > 3 && b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "jpeg";
  if (b.length > 8 && PNG_SIGNATURE.every((v, i) => b[i] === v)) return "png";
  const tag = (o) => String.fromCharCode(b[o], b[o + 1], b[o + 2], b[o + 3]);
  if (b.length > 12 && tag(0) === "RIFF" && tag(8) === "WEBP") return "webp";
  if (b.length > 4 && tag(0) === "GIF8") return "gif";
  return null;
}

// What a PDF image object needs: { width, height, colorSpace, bits, filter, decodeParms, decode, bytes, smask }.
export function pdfImage(bytes) {
  const kind = sniff(bytes);
  if (kind === "jpeg") return jpegImage(bytes);
  if (kind === "png") return pngImage(bytes);
  if (kind === "webp" || kind === "gif") throw new Error(kind + " images cannot be shown in a PDF; convert the image to PNG or JPEG");
  throw new Error("image data is not a PNG or a JPEG");
}

// ---- JPEG: the size and colour components from the frame header; the bytes go in unchanged.
function jpegImage(b) {
  let i = 2, adobe = false;
  while (i + 4 <= b.length) {
    if (b[i] !== 0xff) throw new Error("JPEG data is damaged");
    const marker = b[i + 1];
    if (marker === 0xff) { i++; continue; }
    if (marker === 0x01 || (marker >= 0xd0 && marker <= 0xd7)) { i += 2; continue; }
    const len = (b[i + 2] << 8) | b[i + 3];
    if (marker === 0xee && len >= 7 && String.fromCharCode(b[i + 4], b[i + 5], b[i + 6], b[i + 7], b[i + 8]) === "Adobe") adobe = true;
    const sof = marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc;
    if (sof) {
      const height = (b[i + 5] << 8) | b[i + 6], width = (b[i + 7] << 8) | b[i + 8], comps = b[i + 9];
      if (!width || !height) throw new Error("JPEG data has no size");
      const colorSpace = comps === 1 ? "DeviceGray" : comps === 4 ? "DeviceCMYK" : "DeviceRGB";
      // Adobe writes CMYK JPEGs inverted; readers expect the Decode array that undoes it.
      const decode = comps === 4 && adobe ? [1, 0, 1, 0, 1, 0, 1, 0] : null;
      return { width, height, colorSpace, bits: 8, filter: "DCTDecode", decodeParms: null, decode, bytes: b, smask: null };
    }
    i += 2 + len;
  }
  throw new Error("JPEG data has no frame header");
}

// ---- PNG
function readPng(b) {
  const out = { idat: [], palette: null, trns: null };
  let i = 8;
  while (i + 8 <= b.length) {
    const len = ((b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]) >>> 0;
    const type = String.fromCharCode(b[i + 4], b[i + 5], b[i + 6], b[i + 7]);
    const data = b.subarray(i + 8, i + 8 + len);
    if (type === "IHDR") {
      out.width = ((data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3]) >>> 0;
      out.height = ((data[4] << 24) | (data[5] << 16) | (data[6] << 8) | data[7]) >>> 0;
      out.bitDepth = data[8]; out.colorType = data[9]; out.interlace = data[12];
    } else if (type === "PLTE") out.palette = data;
    else if (type === "tRNS") out.trns = data;
    else if (type === "IDAT") out.idat.push(data);
    else if (type === "IEND") break;
    i += 12 + len;
  }
  if (!out.width || !out.height || !out.idat.length) throw new Error("PNG data is damaged");
  let size = 0;
  for (const part of out.idat) size += part.length;
  out.data = new Uint8Array(size);
  let at = 0;
  for (const part of out.idat) { out.data.set(part, at); at += part.length; }
  return out;
}

const CHANNELS = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };

function pngImage(b) {
  const png = readPng(b);
  const { width, height, bitDepth, colorType } = png;
  if (!(colorType in CHANNELS)) throw new Error("PNG colour type " + colorType + " is not valid");
  if (png.interlace) throw new Error("interlaced PNG images are not supported yet; save the image without interlacing");
  // Opaque grey or RGB: the PNG's own compressed data is a valid PDF stream with the PNG predictor.
  if ((colorType === 0 || colorType === 2) && !png.trns) {
    const colors = CHANNELS[colorType];
    return { width, height, colorSpace: colors === 1 ? "DeviceGray" : "DeviceRGB", bits: bitDepth, filter: "FlateDecode",
      decodeParms: { Predictor: 15, Colors: colors, BitsPerComponent: bitDepth, Columns: width }, decode: null, bytes: png.data, smask: null };
  }
  const { color, colors, alpha } = decodePng(png);
  return { width, height, colorSpace: colors === 1 ? "DeviceGray" : "DeviceRGB", bits: 8, filter: null, decodeParms: null, decode: null,
    bytes: color, smask: alpha ? { width, height, bytes: alpha } : null };
}

// Colour as 8-bit grey or RGB, and alpha (null when every pixel is opaque).
function decodePng(png) {
  const { width, height, bitDepth, colorType, palette, trns } = png;
  const channels = CHANNELS[colorType];
  const rowBytes = Math.ceil((width * channels * bitDepth) / 8);
  const bpp = Math.max(1, (channels * bitDepth) >> 3);
  const raw = inflate(png.data, height * (rowBytes + 1));
  const rows = unfilter(raw, height, rowBytes, bpp);
  const colors = colorType === 0 || colorType === 4 ? 1 : 3;
  const color = new Uint8Array(width * height * colors);
  const alpha = new Uint8Array(width * height);
  const max = (1 << bitDepth) - 1;
  let opaque = true;
  // One sample at its own depth, as an integer.
  const sample = (row, index) => {
    if (bitDepth === 8) return rows[row * rowBytes + index];
    if (bitDepth === 16) return (rows[row * rowBytes + index * 2] << 8) | rows[row * rowBytes + index * 2 + 1];
    const bit = index * bitDepth, byte = rows[row * rowBytes + (bit >> 3)];
    return (byte >> (8 - bitDepth - (bit & 7))) & max;
  };
  const to8 = (v) => (bitDepth === 16 ? v >> 8 : bitDepth === 8 ? v : Math.round((v * 255) / max));
  const key16 = (o) => (trns[o] << 8) | trns[o + 1];
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const p = y * width + x, s = x * channels;
      let a = 255;
      if (colorType === 3) {
        const index = sample(y, s);
        if (!palette || index * 3 + 2 >= palette.length) throw new Error("PNG palette is missing an entry");
        color[p * 3] = palette[index * 3]; color[p * 3 + 1] = palette[index * 3 + 1]; color[p * 3 + 2] = palette[index * 3 + 2];
        if (trns && index < trns.length) a = trns[index];
      } else if (colorType === 0 || colorType === 4) {
        const v = sample(y, s);
        color[p] = to8(v);
        if (colorType === 4) a = to8(sample(y, s + 1));
        else if (trns && trns.length >= 2 && v === key16(0)) a = 0;
      } else {
        const r = sample(y, s), g = sample(y, s + 1), bl = sample(y, s + 2);
        color[p * 3] = to8(r); color[p * 3 + 1] = to8(g); color[p * 3 + 2] = to8(bl);
        if (colorType === 6) a = to8(sample(y, s + 3));
        else if (trns && trns.length >= 6 && r === key16(0) && g === key16(2) && bl === key16(4)) a = 0;
      }
      alpha[p] = a;
      if (a !== 255) opaque = false;
    }
  }
  return { color, colors, alpha: opaque ? null : alpha };
}

// Undo the PNG row filters (None, Sub, Up, Average, Paeth); returns the rows without their filter bytes.
function unfilter(raw, height, rowBytes, bpp) {
  const out = new Uint8Array(height * rowBytes);
  for (let y = 0; y < height; y++) {
    const type = raw[y * (rowBytes + 1)], src = y * (rowBytes + 1) + 1, dst = y * rowBytes, prev = dst - rowBytes;
    for (let i = 0; i < rowBytes; i++) {
      const left = i >= bpp ? out[dst + i - bpp] : 0;
      const up = y > 0 ? out[prev + i] : 0;
      const upLeft = y > 0 && i >= bpp ? out[prev + i - bpp] : 0;
      let v = raw[src + i];
      if (type === 1) v += left;
      else if (type === 2) v += up;
      else if (type === 3) v += (left + up) >> 1;
      else if (type === 4) {
        const pa = Math.abs(up - upLeft), pb = Math.abs(left - upLeft), pc = Math.abs(left + up - 2 * upLeft);
        v += pa <= pb && pa <= pc ? left : pb <= pc ? up : upLeft;
      } else if (type !== 0) throw new Error("PNG row filter " + type + " is not valid");
      out[dst + i] = v & 0xff;
    }
  }
  return out;
}

// ---- Inflate: zlib (RFC 1950) around DEFLATE (RFC 1951), the compression of a PNG's image data.
const LEN_BASE = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LEN_EXTRA = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];
const DIST_BASE = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577];
const DIST_EXTRA = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];
const ORDER = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

// A canonical Huffman code from code lengths: how many codes per length, and the symbols in code order.
function huffman(lengths) {
  const counts = new Uint16Array(16), symbols = new Uint16Array(lengths.length), offs = new Uint16Array(16);
  for (const l of lengths) counts[l]++;
  counts[0] = 0;
  for (let l = 1; l < 16; l++) offs[l] = offs[l - 1] + counts[l - 1];
  lengths.forEach((l, s) => { if (l) symbols[offs[l]++] = s; });
  return { counts, symbols };
}

function inflate(data, expected) {
  if (data.length < 2 || (data[0] & 0x0f) !== 8 || ((data[0] << 8) | data[1]) % 31 !== 0) throw new Error("PNG image data is not zlib-compressed");
  if (data[1] & 0x20) throw new Error("PNG image data asks for a preset dictionary");
  let pos = 2, buf = 0, cnt = 0, out = new Uint8Array(Math.max(expected, 1024)), n = 0;
  const bits = (k) => {
    while (cnt < k) {
      if (pos >= data.length) throw new Error("PNG image data ends early");
      buf |= data[pos++] << cnt; cnt += 8;
    }
    const v = buf & ((1 << k) - 1);
    buf >>>= k; cnt -= k;
    return v;
  };
  const decode = (t) => {
    let code = 0, first = 0, index = 0;
    for (let l = 1; l < 16; l++) {
      code |= bits(1);
      const c = t.counts[l];
      if (code - first < c) return t.symbols[index + code - first];
      index += c; first += c; first <<= 1; code <<= 1;
    }
    throw new Error("PNG image data has a bad code");
  };
  const room = (k) => {
    if (n + k <= out.length) return;
    const grown = new Uint8Array(Math.max(out.length * 2, n + k));
    grown.set(out.subarray(0, n)); out = grown;
  };
  let fixedLit = null, fixedDist = null, last = 0;
  while (!last) {
    last = bits(1);
    const type = bits(2);
    if (type === 0) {
      buf = 0; cnt = 0;
      if (pos + 4 > data.length) throw new Error("PNG image data ends early");
      const len = data[pos] | (data[pos + 1] << 8);
      pos += 4;
      if (pos + len > data.length) throw new Error("PNG image data ends early");
      room(len); out.set(data.subarray(pos, pos + len), n); n += len; pos += len;
      continue;
    }
    let lit, dist;
    if (type === 1) {
      if (!fixedLit) {
        const l = new Array(288);
        for (let i = 0; i < 288; i++) l[i] = i < 144 ? 8 : i < 256 ? 9 : i < 280 ? 7 : 8;
        fixedLit = huffman(l); fixedDist = huffman(new Array(30).fill(5));
      }
      lit = fixedLit; dist = fixedDist;
    } else if (type === 2) {
      const hlit = bits(5) + 257, hdist = bits(5) + 1, hclen = bits(4) + 4;
      const cl = new Array(19).fill(0);
      for (let i = 0; i < hclen; i++) cl[ORDER[i]] = bits(3);
      const clTree = huffman(cl), lengths = [];
      while (lengths.length < hlit + hdist) {
        const sym = decode(clTree);
        if (sym < 16) lengths.push(sym);
        else if (sym === 16) {
          if (!lengths.length) throw new Error("PNG image data has a bad code");
          const prev = lengths[lengths.length - 1];
          for (let r = 3 + bits(2); r > 0; r--) lengths.push(prev);
        } else for (let r = sym === 17 ? 3 + bits(3) : 11 + bits(7); r > 0; r--) lengths.push(0);
      }
      lit = huffman(lengths.slice(0, hlit)); dist = huffman(lengths.slice(hlit, hlit + hdist));
    } else throw new Error("PNG image data has a bad block");
    for (;;) {
      const sym = decode(lit);
      if (sym < 256) { room(1); out[n++] = sym; continue; }
      if (sym === 256) break;
      const li = sym - 257;
      if (li >= 29) throw new Error("PNG image data has a bad code");
      const len = LEN_BASE[li] + bits(LEN_EXTRA[li]);
      const di = decode(dist);
      if (di >= 30) throw new Error("PNG image data has a bad code");
      const d = DIST_BASE[di] + bits(DIST_EXTRA[di]);
      if (d > n) throw new Error("PNG image data has a bad distance");
      room(len);
      for (let k = 0; k < len; k++, n++) out[n] = out[n - d];
    }
  }
  return out.subarray(0, n);
}
