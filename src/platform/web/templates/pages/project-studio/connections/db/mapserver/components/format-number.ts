/** A count for the layer pages: grouped digits, `—` when there is none. */
export function formatNumber(n) {
  if (n == null) return "—";
  if (typeof n === "number") return n.toLocaleString();
  return String(n);
}
