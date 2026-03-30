import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";

/**
 * Specialized claims editor for `auth.token.create`.
 *
 * Each row renders: [claim name] [value / $.path] [Public ✓] [×]
 *
 * The "Public" checkbox appends/strips the `:public` suffix on the value string.
 * Public claims are the only ones exposed in the browser via `ctx.auth`.
 * Private claims (no checkbox) are signed into the JWT but never reach the browser DOM.
 */
export default function NodeFieldClaimsPairs({ field, value, onChange }) {
  const raw: Record<string, string> =
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, string>)
      : {};

  const pairs: [string, string][] = Object.entries(raw);

  function isPublic(val: string): boolean {
    return typeof val === "string" && val.endsWith(":public");
  }

  function baseVal(val: string): string {
    return isPublic(val) ? val.slice(0, -7) : val;
  }

  function commit(next: [string, string][]) {
    onChange(Object.fromEntries(next));
  }

  function updateKey(idx: number, newKey: string) {
    const next = [...pairs] as [string, string][];
    next[idx] = [newKey, next[idx][1]];
    commit(next);
  }

  function updateValue(idx: number, newBase: string) {
    const next = [...pairs] as [string, string][];
    const suffix = isPublic(next[idx][1]) ? ":public" : "";
    next[idx] = [next[idx][0], newBase + suffix];
    commit(next);
  }

  function togglePublic(idx: number, checked: boolean) {
    const next = [...pairs] as [string, string][];
    const base = baseVal(next[idx][1]);
    next[idx] = [next[idx][0], checked ? base + ":public" : base];
    commit(next);
  }

  function removePair(idx: number) {
    commit(pairs.filter((_, i) => i !== idx) as [string, string][]);
  }

  function addPair() {
    const existingKeys = new Set(pairs.map(([k]) => k));
    let key = "";
    let n = 0;
    while (existingKeys.has(key)) key = `claim${++n}`;
    onChange({ ...raw, [key]: "" });
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-1.5">
        {pairs.length > 0 && (
          <div className="flex gap-1.5 items-center text-xs text-muted-foreground px-0.5">
            <span className="flex-1">Claim name</span>
            <span className="flex-1">Value / $.path</span>
            <span className="w-14 text-center">Public</span>
            <span className="w-5" />
          </div>
        )}
        {pairs.map(([k, v], idx) => (
          <div key={idx} className="flex gap-1.5 items-center">
            <Input
              type="text"
              value={k}
              placeholder="claim_name"
              onInput={(e) => updateKey(idx, e.currentTarget.value)}
            />
            <Input
              type="text"
              value={baseVal(v)}
              placeholder="$.field or literal"
              onInput={(e) => updateValue(idx, e.currentTarget.value)}
            />
            <div className="w-14 flex justify-center">
              <Checkbox
                checked={isPublic(v)}
                title="Expose this claim in the browser via ctx.auth"
                onChange={(e) => togglePublic(idx, e.currentTarget.checked)}
              />
            </div>
            <Button
              variant="ghost"
              size="xs"
              onClick={() => removePair(idx)}
              title="Remove"
            >
              ×
            </Button>
          </div>
        ))}
        <div>
          <Button variant="outline" size="xs" onClick={addPair}>
            + Add Claim
          </Button>
        </div>
      </div>
    </Field>
  );
}
