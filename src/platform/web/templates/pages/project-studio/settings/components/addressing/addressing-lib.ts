/**
 * Small pure helpers the Addressing panels share (`docs/contracts/addressing.md`).
 */

/** The dev URL for a path on the project's automatic host. */
export function devUrl(data: any, path: string = "/"): string {
  const base = String(data?.dev_url ?? "").replace(/\/$/, "");
  const rel = path.startsWith("/") ? path : `/${path}`;
  return `${base}${rel === "/" ? "/" : rel}`;
}

/** Copy to the clipboard; resolves false when the browser refuses. */
export async function copyText(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch (_err) {
    // fall through
  }
  return false;
}

/** A route's line as a person reads it: `host/path`. */
export function routeLabel(route: any): string {
  const host = String(route?.host ?? "");
  const path = String(route?.path ?? "/");
  return `${host}${path}`;
}

/** The surface title for a key, from the section data. */
export function surfaceTitle(data: any, key: string): string {
  const found = (Array.isArray(data?.surfaces) ? data.surfaces : []).find((s: any) => s?.key === key);
  return found?.title ?? key;
}

/** One line for a DNS + verify result. */
export function checkSummary(check: any): { tone: string; text: string } {
  if (!check) return { tone: "muted", text: "not checked yet" };
  if (!check?.dns?.ok) return { tone: "error", text: "DNS: the name does not resolve yet" };
  const ips = (check.dns.resolved ?? []).join(", ");
  if (check?.verify?.ok) return { tone: "ok", text: `DNS → ${ips} · answers from this project (${check.verify.url})` };
  return { tone: "error", text: `DNS → ${ips} · ${check?.verify?.reason ?? "not verified"}` };
}
