/**
 * L0 – PDF Object Primitives
 *
 * Each factory returns an object with:
 *   .size()              → exact byte count when serialized
 *   .write(buf, offset)  → writes bytes into Uint8Array, returns bytes written
 *
 * The atomic write kernel – all serialization flows through this:
 */
export function writeStr(str, buf, offset) {
  for (let i = 0; i < str.length; i++) buf[offset++] = str.charCodeAt(i);
  return str.length;
}

export function writeBytes(bytes, buf, offset) {
  buf.set(bytes, offset);
  return bytes.length;
}

// --- Escape a PDF string: (), \ must be escaped ---
function escapePdfStr(str) {
  let out = "";
  for (let i = 0; i < str.length; i++) {
    const c = str[i];
    if (c === "(") out += "\\(";
    else if (c === ")") out += "\\)";
    else if (c === "\\") out += "\\\\";
    else if (c === "\n") out += "\\n";
    else if (c === "\r") out += "\\r";
    else if (c === "\t") out += "\\t";
    else out += c;
  }
  return out;
}

// --- Number formatting: no trailing zeros, no scientific notation ---
function fmtNum(n) {
  if (Number.isInteger(n)) return String(n);
  // Round to 4 decimal places, strip trailing zeros
  return parseFloat(n.toFixed(4)).toString();
}

// /Name
export function pName(name) {
  // name should already include leading slash e.g. "/Type"
  const s = name.startsWith("/") ? name : "/" + name;
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// (string)
export function pStr(str) {
  const escaped = "(" + escapePdfStr(str) + ")";
  return {
    size() { return escaped.length; },
    write(buf, offset) { return writeStr(escaped, buf, offset); },
    toString() { return escaped; },
  };
}

// integer
export function pInt(n) {
  const s = String(Math.round(n));
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// real
export function pReal(n) {
  const s = fmtNum(n);
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// true | false
export function pBool(b) {
  const s = b ? "true" : "false";
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// null
export function pNull() {
  return {
    size() { return 4; },
    write(buf, offset) { return writeStr("null", buf, offset); },
    toString() { return "null"; },
  };
}

// id gen R  (indirect reference)
export function pRef(id, gen = 0) {
  const s = `${id} ${gen} R`;
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// [ obj obj ... ]
export function pArray(items) {
  // Pre-build string representation
  const parts = items.map(i => i.toString());
  const s = "[ " + parts.join(" ") + " ]";
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// << /Key value ... >>
export function pDict(entries) {
  // entries: plain object { Key: pdfObj, ... }
  let s = "<<";
  for (const [k, v] of Object.entries(entries)) {
    const key = k.startsWith("/") ? k : "/" + k;
    s += "\n" + key + " " + v.toString();
  }
  s += "\n>>";
  return {
    size() { return s.length; },
    write(buf, offset) { return writeStr(s, buf, offset); },
    toString() { return s; },
  };
}

// << dict >> stream\n bytes \nendstream
export function pStream(dictEntries, bytes) {
  // bytes: Uint8Array of raw stream content
  const dictWithLen = { ...dictEntries, Length: pInt(bytes.length) };
  const dictStr = pDict(dictWithLen).toString();
  // "stream\n" + bytes + "\nendstream"
  const prefix = dictStr + "\nstream\n";
  const suffix = "\nendstream";
  const totalSize = prefix.length + bytes.length + suffix.length;
  return {
    size() { return totalSize; },
    write(buf, offset) {
      let n = writeStr(prefix, buf, offset);
      n += writeBytes(bytes, buf, offset + n);
      n += writeStr(suffix, buf, offset + n);
      return n;
    },
    toString() {
      // For embedding inside indirect obj, prefix only (no bytes in toString)
      return prefix + "...[" + bytes.length + " bytes]..." + suffix;
    },
    _bytes: bytes,
    _prefix: prefix,
    _suffix: suffix,
  };
}

// id gen obj\n content \nendobj
export function pIndirect(id, gen, content) {
  const header = `${id} ${gen} obj\n`;
  const footer = "\nendobj";
  return {
    id,
    gen,
    size() { return header.length + content.size() + footer.length; },
    write(buf, offset) {
      let n = writeStr(header, buf, offset);
      n += content.write(buf, offset + n);
      n += writeStr(footer, buf, offset + n);
      return n;
    },
  };
}
