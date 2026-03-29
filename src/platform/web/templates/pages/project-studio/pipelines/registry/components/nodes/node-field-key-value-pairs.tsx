import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

export default function NodeFieldKeyValuePairs({ field, value, onChange }) {
  const raw: Record<string, string> =
    value && typeof value === "object" && !Array.isArray(value)
      ? (value as Record<string, string>)
      : {};

  const pairs: [string, string][] = Object.entries(raw);

  function updatePair(idx: number, newKey: string, newVal: string) {
    const next = [...pairs];
    next[idx] = [newKey, newVal];
    onChange(Object.fromEntries(next));
  }

  function removePair(idx: number) {
    const next = pairs.filter((_, i) => i !== idx);
    onChange(Object.fromEntries(next));
  }

  function addPair() {
    // Find a unique empty key to avoid silently deduplicating
    const existingKeys = new Set(pairs.map(([k]) => k));
    let key = "";
    let n = 0;
    while (existingKeys.has(key)) {
      key = `key${++n}`;
    }
    onChange({ ...raw, [key]: "" });
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-1.5">
        {pairs.map(([k, v], idx) => (
          <div key={idx} className="flex gap-1.5 items-center">
            <Input
              type="text"
              value={k}
              placeholder="key"
              onInput={(e) => updatePair(idx, e.currentTarget.value, v)}
            />
            <Input
              type="text"
              value={v}
              placeholder="value"
              onInput={(e) => updatePair(idx, k, e.currentTarget.value)}
            />
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
            + Add
          </Button>
        </div>
      </div>
    </Field>
  );
}
