/**
 * Where a project answers, as the browser sees it (`docs/contracts/addressing.md`).
 *
 * Every project has the dev host `<project>.<owner>.localhost`, which any
 * browser resolves to loopback. Studio links to a project's routes use it, so
 * what a person clicks behaves the way the site will behind a real domain:
 * `href="/book"` and `Location: /admin` stay on the site.
 *
 * Client-side only: it reads `window.location` for the scheme and port.
 */

export function devHost(owner: string, project: string): string {
  return `${String(project || "").trim()}.${String(owner || "").trim()}.localhost`;
}

/** `http://project.owner.localhost:10610` — "" during server render. */
export function devOrigin(owner: string, project: string): string {
  if (typeof window === "undefined" || !owner || !project) return "";
  const port = window.location.port ? `:${window.location.port}` : "";
  return `${window.location.protocol}//${devHost(owner, project)}${port}`;
}

/**
 * The URL a route answers at on the dev host; "" during server render. A
 * route under a section's root answers on that section's dev host, the root
 * taken off (`addressing.md` §2c): pass the project's `sections`.
 */
export function projectRouteUrl(owner: string, project: string, path: string, sections: any[] = []): string {
  const origin = devOrigin(owner, project);
  if (!origin) return "";
  const norm = String(path || "/").trim() || "/";
  const normalized = norm.startsWith("/") ? norm : `/${norm}`;
  const first = normalized.split("/").filter(Boolean)[0] ?? "";
  const section = (Array.isArray(sections) ? sections : []).find((s: any) => s?.root === `/${first}`);
  if (section?.dev_host && typeof window !== "undefined") {
    const port = window.location.port ? `:${window.location.port}` : "";
    const rest = normalized.slice(first.length + 1) || "/";
    return `${window.location.protocol}//${section.dev_host}${port}${rest}`;
  }
  return `${origin}${normalized}`;
}
