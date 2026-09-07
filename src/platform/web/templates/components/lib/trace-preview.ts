/**
 * Human-readable trace previews. Unquoted ellipses denote engine summaries;
 * payload strings remain JSON-quoted, so a real "..." stays distinguishable.
 * This is display text, not a replacement for the stored structured trace.
 */
export function formatTracePreview(value, depth = 0) {
  const indent = "  ".repeat(depth);
  const childIndent = `${indent}  `;
  function container(open, close, lines) {
    return lines.length ? `${open}\n${childIndent}${lines.join(`,\n${childIndent}`)}\n${indent}${close}` : `${open}${close}`;
  }
  if (value && typeof value === "object" && !Array.isArray(value)) {
    const kind = value.__zf_trace_summary;
    if (kind === "bytes") return `… (${value.reason || "capture budget exhausted"})`;
    if (kind === "string") {
      return `${JSON.stringify(value.preview || "")} … (${value.chars ?? "unknown"} characters total)`;
    }
    if (kind === "array" || kind === "numeric_array") {
      const preview = Array.isArray(value.preview) ? value.preview : [];
      const lines = preview.map((item) => formatTracePreview(item, depth + 1));
      const omitted = value.omitted ?? Math.max(0, Number(value.len || 0) - preview.length);
      lines.push(`… (+${omitted} items${value.reason ? `; ${value.reason}` : ""})`);
      return container("[", "]", lines);
    }
    if (kind === "object") {
      const preview = value.preview && typeof value.preview === "object" ? value.preview : {};
      const entries = Object.entries(preview);
      const lines = entries.map(([key, item]) => `${JSON.stringify(key)}: ${formatTracePreview(item, depth + 1)}`);
      lines.push(`… (+${Math.max(0, Number(value.keys || 0) - entries.length)} keys${value.reason ? `; ${value.reason}` : ""})`);
      return container("{", "}", lines);
    }
    return container("{", "}", Object.entries(value).map(([key, item]) => `${JSON.stringify(key)}: ${formatTracePreview(item, depth + 1)}`));
  }
  if (Array.isArray(value)) return container("[", "]", value.map((item) => formatTracePreview(item, depth + 1)));
  return JSON.stringify(value) ?? "null";
}
