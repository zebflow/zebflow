import { useState, useEffect } from "zeb";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

const PARAM_TYPES = [
  "string",
  "number",
  "integer",
  "boolean",
  "object",
  "array",
  "any",
  "file",
  "bytes",
  "blob",
  "string[]",
  "number[]",
  "integer[]",
  "boolean[]",
  "object[]",
  "file[]",
  "bytes[]",
  "blob[]",
];

interface ParamDef {
  type: string;
  description: string;
  required?: boolean;
  default?: any;
}

function typeFromSchema(schema: any): string {
  const zebType = String(schema?.["x-zebflow-type"] || "");
  if (zebType) return zebType;
  if (schema?.type === "array") {
    return `${typeFromSchema(schema?.items || { type: "any" })}[]`;
  }
  return String(schema?.type || "any");
}

function parseValue(value: unknown): [string, ParamDef][] {
  let schema: Record<string, any> = {};
  if (typeof value === "string" && value.trim()) {
    try { schema = JSON.parse(value); } catch { schema = {}; }
  } else if (value && typeof value === "object" && !Array.isArray(value)) {
    schema = value as Record<string, any>;
  }
  const props = schema.properties && typeof schema.properties === "object" ? schema.properties : schema;
  const required = new Set(Array.isArray(schema.required) ? schema.required.map(String) : []);
  return Object.entries(props).map(([k, v]) => [
    k,
    {
      type: typeFromSchema(v),
      description: String((v as any)?.description || ""),
      required: required.has(k) || Boolean((v as any)?.required),
      ...(((v as any)?.default !== undefined) ? { default: (v as any).default } : {}),
    },
  ]);
}

function schemaForType(type: string): Record<string, any> {
  if (type.endsWith("[]")) {
    return { type: "array", items: schemaForType(type.slice(0, -2)) };
  }
  if (type === "any") return {};
  if (type === "file" || type === "bytes" || type === "blob") {
    return { type: "object", "x-zebflow-type": type };
  }
  return { type };
}

function toObject(entries: [string, ParamDef][]): Record<string, any> {
  const properties: Record<string, any> = {};
  const required: string[] = [];
  for (const [k, v] of entries) {
    const name = k.trim();
    if (!name) continue;
    properties[name] = {
      ...schemaForType(v.type),
      ...(v.description ? { description: v.description } : {}),
    };
    if (v.required) required.push(name);
  }
  return { type: "object", required, properties };
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

  function updateRequired(idx: number, required: boolean) {
    const next = entries.map((e, i) => i === idx ? [e[0], { ...e[1], required }] as [string, ParamDef] : e);
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
    emit([...entries, [key, { type: "string", description: "", required: false }]]);
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-1">
        {entries.length > 0 && (
          <div className="rounded border border-dark-border overflow-hidden">
            <div className="grid px-2 py-1 bg-dark-accent3/40 border-b border-dark-border"
              style={{ gridTemplateColumns: "1fr 7rem 4rem 1fr 1.5rem" }}>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Name</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Type</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Req</span>
              <span className="text-[0.65rem] font-semibold uppercase tracking-wide text-body-muted">Description</span>
              <span />
            </div>
            {entries.map(([name, def], idx) => (
              <div
                key={idx}
                className="grid items-center gap-1.5 px-2 py-1.5 border-b border-dark-border last:border-0"
                style={{ gridTemplateColumns: "1fr 7rem 4rem 1fr 1.5rem" }}
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
                <label className="flex items-center justify-center">
                  <input
                    type="checkbox"
                    checked={Boolean(def.required)}
                    onChange={(e) => updateRequired(idx, e.currentTarget.checked)}
                    className="h-4 w-4 accent-accent"
                    aria-label={`Mark ${name || "field"} as required`}
                  />
                </label>
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
