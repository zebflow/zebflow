/**
 * One list of packages, assembled from the three payloads that describe them.
 *
 * The catalogue, the enabled-library list and the dependency lock each hold a
 * piece of the same truth, and each names things its own way: the hub calls a
 * library `zebflow.codemirror`, the library list calls it `zeb/codemirror`.
 * Left unmerged they appear as separate rows, which is how the same library
 * ended up listed in two places under two names.
 */

/** `zebflow.codemirror` and `zeb/codemirror` are the same package. */
export function packageIdentity(value: string): string {
  return String(value || "")
    .replace(/^zebflow[./]/, "")
    .replace(/^zeb[./]/, "")
    .replace(/[./]/g, "-")
    .toLowerCase();
}

/**
 * Merges catalogue, installed state and lock health into the shape the browser
 * draws: every row knows its kind, whether this project has it, whether a newer
 * version exists, and whether what is on disk still matches the lock.
 */
export function buildHubItems({ assets, libraries, lock }) {
  const items = new Map();

  for (const asset of Array.isArray(assets) ? assets : []) {
    const id = packageIdentity(asset?.package_id);
    items.set(id, {
      ...asset,
      package_id: asset?.package_id,
      installed: false,
      updatable: false,
      status: "",
    });
  }

  // An enabled library is this project's copy of a catalogue entry, not a
  // separate thing to list.
  for (const lib of Array.isArray(libraries) ? libraries : []) {
    const id = packageIdentity(lib?.name);
    const existing = items.get(id);
    const installedVersion = lib?.installed_version || "";
    const available = lib?.packed_version || "";
    const merged = {
      ...(existing || {
        package_id: lib?.name,
        asset_kind: "rwe_library",
        title: lib?.name,
        summary: lib?.description,
        repository_title: "Blessed",
        source: "local",
      }),
      title: existing?.title || lib?.name,
      summary: existing?.summary || lib?.description,
      latest_version: available || existing?.latest_version,
      installed: !!lib?.enabled,
      installed_version: installedVersion,
      updatable: !!lib?.enabled && !!available && !!installedVersion && installedVersion !== available,
      status: existing?.status || "",
    };
    items.set(id, merged);
  }

  // The lock is the only thing that knows whether an installed package is still
  // intact. A problem belongs on the package, not in a list of its own.
  for (const entry of Array.isArray(lock?.items) ? lock.items : []) {
    const id = packageIdentity(entry?.name || entry?.package_id);
    const existing = items.get(id);
    if (!existing) continue;
    items.set(id, {
      ...existing,
      installed: true,
      status: entry?.status || existing.status,
    });
  }

  return [...items.values()].sort((a, b) =>
    String(a?.title || a?.package_id).localeCompare(String(b?.title || b?.package_id)),
  );
}
