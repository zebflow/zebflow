/**
 * Formatting every page shares.
 *
 * These were copied into six pages apiece, and the copies had drifted: one
 * stopped at MB, another carried on to TB, a third rounded differently. A
 * reader comparing two screens could not tell whether a difference was the
 * data or the page.
 */

/** Byte counts, up to TB, with one decimal above bytes. */
export function formatBytes(bytes: any): string {
  const value = Number(bytes || 0);
  if (!Number.isFinite(value) || value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${Math.round(size)} B` : `${size.toFixed(1)} ${units[unit]}`;
}

/**
 * A unix timestamp in seconds as `YYYY-MM-DD HH:MM:SS`.
 *
 * Answers `-` for a missing or unreadable value rather than `Invalid Date`,
 * which is what several of the copies rendered.
 */
export function formatTs(ts: any): string {
  const seconds = Number(ts || 0);
  if (!seconds) return "-";
  const at = new Date(seconds * 1000);
  if (Number.isNaN(at.getTime())) return "-";
  return at.toISOString().slice(0, 19).replace("T", " ");
}

// `slugify` deliberately does not live here. The hub strips every character
// that is not a letter or digit and caps the result at 80; connections keeps
// `.`, `_` and `-` because a connection slug may contain them. Same name,
// different rule — merging them would change what one of the callers means.
