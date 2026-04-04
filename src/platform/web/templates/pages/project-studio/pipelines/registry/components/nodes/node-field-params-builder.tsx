import { useState, useEffect } from "zeb";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

const PARAM_TYPES = ["string", "number", "boolean", "object", "array"];

interface ParamDef {
  type: string;
  description: string;
  default?: any;
}

function parseValue(value: unknown): [string, ParamDef][] {
  let obj: Record<string, any> = {};
  if (typeof value === "string" && value.trim()) {
    try { obj = JSON.parse(value); } catch { obj = {}; }
  } else if (value && typeof value === "object" && !Array.isArray(value)) {
    obj = value as Record<string, any>;
  }
  return Object.entries(obj).map(([k, v]) => [
    k,
    {
      type: String((v as any)?.type || "string"),
      description: String((v as any)?.description || ""),
      ...(((v as any)?.default !== undefined) ? { default: (v as any).default } : {}),
    },
  ]);
}

function toObject(entries: [string, ParamDef][]): Record<string, any> {
  const out: Record<string, any> = {};
  for (const [k, v] of entries) {
    if (!k.trim()) continue;
    out[k] = { type: v.type, ...(v.description ? { description: v.description } : {}) };
  }
  return out;
}

export default function NodeFieldParamsBuilder({ field, value, onChange }) {
  const [entries, setEntries] = useState<[string, ParamDef][]>(() => parseValue(value));

  // Sync when external value changes (e.g. dialog opened for different node)
  useEffect(() => {
    setEntries(parseValue(value));
  }, [JSON.stringify(value)]);

  function emit(next: [string, ParamDef][]) {
    setEntries(next);
    onChange(toObject(next));
  }

  function updateName(idx: number, name: string) {
    const next = entries.map((e, i) => i === idx ? [name, e[1]] as [string, ParamDef] : e);
    emit(next);
  }

  function updateType(idx: number, type: string) {
    const next = entries.map((e, i) => i === idx ? [e[0], { ...e[1], type }] as [string, ParamDef] : e);
    emit(next);
  }

  function updateDesc(idx: number, description: string) {
    const next = entries.map((e, i) => i === idx ? [e[0], { ...e[1], description }] as [string, ParamDef] : e);
    emit(next);
  }

  function remove(idx: number) {
    emit(entries.filter((_, i) => i !== idx));
  }

  function addParam() {
    const existing = new Set(entries.map(([k]) => k));
    let key = "param";
    let n = 1;
    while (existing.has(key)) key = `param${++n}`;
    emit([...entries, [key, { type: "string", description: "" }]]);
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-1">
        {entries.length > 0 && (
          <div className="rounded border border-dark-border overflow-hidden">
            <div className="grid px-2 py-1 bg-dark-accent3/40 border-b border-dark-border"
              style={{ gridTemplateColumns: "1fr 7rem 1fr 1.5rem" }}>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Name</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Type</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Description</span>
              <span />
            </div>
            {entries.map(([name, def], idx) => (
              <div
                key={idx}
                className="grid items-center gap-1.5 px-2 py-1.5 border-b border-dark-border last:border-0"
                style={{ gridTemplateColumns: "1fr 7rem 1fr 1.5rem" }}
              >
                <Input
                  value={name}
                  placeholder="param_name"
                  onInput={(e) => updateName(idx, e.currentTarget.value)}
                />
                <select
                  value={def.type}
                  onChange={(e) => updateType(idx, e.currentTarget.value)}
                  className="h-8 w-full rounded border border-dark-border bg-dark-accent3 px-2 text-[0.78rem] text-body focus:outline-none focus:ring-1 focus:ring-accent"
                >
                  {PARAM_TYPES.map((t) => (
                    <option key={t} value={t}>{t}</option>
                  ))}
                </select>
                <Input
                  value={def.description}
                  placeholder="description (optional)"
                  onInput={(e) => updateDesc(idx, e.currentTarget.value)}
                />
                <button
                  type="button"
                  onClick={() => remove(idx)}
                  className="flex items-center justify-center w-6 h-6 rounded text-body-soft hover:text-red-400 hover:bg-dark-accent3 transition-colors text-sm"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
        <div>
          <Button variant="outline" size="xs" onClick={addParam}>
            + Add param
          </Button>
        </div>
      </div>
    </Field>
  );
}
