/**
 * How the rooms tab writes a room's times and counter names. The row and its
 * details both show times, so the rule lives once here.
 */

/** An RFC 3339 instant as the reader's local date and time; `-` when absent. */
export function formatRoomTime(value: any): string {
  if (!value) return "-";
  const at = new Date(String(value));
  return Number.isNaN(at.getTime()) ? String(value) : at.toLocaleString();
}

/** `tick_overruns` → `Tick overruns`. */
export function roomCounterLabel(key: string): string {
  const words = String(key || "").replace(/_/g, " ").trim();
  return words ? words.charAt(0).toUpperCase() + words.slice(1) : "-";
}
