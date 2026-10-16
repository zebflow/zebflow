/**
 * Host data for a story (capability `host-data`): `{ inputs, collections }` patches as
 * `player.apply` takes them. Patches waiting for a player (not mounted, or its story still
 * loading) merge the way the player merges calls within one frame: inputs field by field per
 * target, and the newest whole list of each collection. The player checks every value itself.
 *
 * Own keys only, into fresh prototype-less objects: a key such as `__proto__` stays an ordinary
 * key and reaches the player, which refuses it (`host-input`) as it does without the wait.
 */
export function mergePatch(into, patch) {
  if (!patch || typeof patch !== "object") return into;
  const inputs = Object.create(null);
  const collections = Object.create(null);
  for (const source of [into?.inputs, patch.inputs]) {
    for (const target of ownKeys(source)) {
      const fields = source[target];
      if (!isRecord(fields)) { inputs[target] = fields; continue; }
      const merged = isRecord(inputs[target]) ? inputs[target] : (inputs[target] = Object.create(null));
      for (const field of ownKeys(fields)) merged[field] = fields[field];
    }
  }
  for (const source of [into?.collections, patch.collections]) for (const name of ownKeys(source)) collections[name] = source[name];
  return { inputs, collections };
}

/** The patch a PotoPlayer's `inputs` and `collections` props describe, or undefined. */
export function declaredPatch(live) {
  if (!live || (!live.inputs && !live.collections)) return undefined;
  return { ...(live.inputs ? { inputs: live.inputs } : {}), ...(live.collections ? { collections: live.collections } : {}) };
}

function ownKeys(value) { return value && typeof value === "object" ? Object.keys(value) : []; }
function isRecord(value) { return !!value && typeof value === "object" && !Array.isArray(value); }
