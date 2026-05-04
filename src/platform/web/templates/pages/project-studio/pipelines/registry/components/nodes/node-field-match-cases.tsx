import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

type MatchCaseRow = {
  value: string;
  pin: string;
  label: string;
};

type MatchRoutesValue = {
  cases?: MatchCaseRow[];
  default?: {
    pin?: string;
    label?: string;
  };
};

function slugifyPin(raw: string, fallback = "case"): string {
  const out = String(raw || "")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return out || fallback;
}

function normalizeCase(item: unknown, index: number): MatchCaseRow {
  if (typeof item === "string") {
    const value = item.trim();
    return { value, pin: slugifyPin(value, `case-${index + 1}`), label: value };
  }
  const source = item && typeof item === "object" ? item as Record<string, unknown> : {};
  const value = String(source.value || "").trim();
  const pin = slugifyPin(String(source.pin || value), `case-${index + 1}`);
  const label = String(source.label || value || pin).trim();
  return { value, pin, label };
}

function normalizeValue(value: unknown): Required<MatchRoutesValue> {
  const source = value && typeof value === "object" && !Array.isArray(value)
    ? value as MatchRoutesValue
    : {};
  const cases = Array.isArray(source.cases)
    ? source.cases.map(normalizeCase)
    : [];
  const rawDefault = source.default && typeof source.default === "object" ? source.default : {};
  const defaultPin = slugifyPin(String(rawDefault.pin || "default"), "default");
  const defaultLabel = String(rawDefault.label || "Default").trim() || "Default";
  return { cases, default: { pin: defaultPin, label: defaultLabel } };
}

export default function NodeFieldMatchCases({ field, value, onChange }) {
  const routes = normalizeValue(value);
  const pins = [
    ...routes.cases.map((item) => item.pin),
    routes.default.pin,
  ];
  const duplicates = new Set(pins.filter((pin, index) => pins.indexOf(pin) !== index));

  function emit(next: Required<MatchRoutesValue>) {
    onChange({
      cases: next.cases,
      default: next.default,
    });
  }

  function updateCase(index: number, patch: Partial<MatchCaseRow>) {
    const cases = routes.cases.map((item, i) => {
      if (i !== index) return item;
      const next = { ...item, ...patch };
      if (patch.value !== undefined && (!item.pin || item.pin === slugifyPin(item.value, `case-${index + 1}`))) {
        next.pin = slugifyPin(patch.value, `case-${index + 1}`);
      }
      if (patch.value !== undefined && (!item.label || item.label === item.value)) {
        next.label = String(patch.value || "");
      }
      if (patch.pin !== undefined) next.pin = slugifyPin(patch.pin, `case-${index + 1}`);
      return next;
    });
    emit({ ...routes, cases });
  }

  function moveCase(index: number, offset: number) {
    const target = index + offset;
    if (target < 0 || target >= routes.cases.length) return;
    const cases = routes.cases.slice();
    const [item] = cases.splice(index, 1);
    cases.splice(target, 0, item);
    emit({ ...routes, cases });
  }

  function removeCase(index: number) {
    emit({ ...routes, cases: routes.cases.filter((_item, i) => i !== index) });
  }

  function addCase() {
    const nextIndex = routes.cases.length + 1;
    emit({
      ...routes,
      cases: [
        ...routes.cases,
        { value: "", pin: `case-${nextIndex}`, label: "" },
      ],
    });
  }

  function updateDefault(patch: Partial<MatchRoutesValue["default"]>) {
    const nextDefault = {
      ...routes.default,
      ...patch,
    };
    if (patch.pin !== undefined) nextDefault.pin = slugifyPin(patch.pin, "default");
    emit({ ...routes, default: nextDefault });
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="match-cases-editor">
        <div className="match-cases-header">
          <span>Match value</span>
          <span>Output pin</span>
          <span>Label</span>
          <span />
        </div>

        <div className="match-cases-list">
          {routes.cases.map((item, index) => (
            <div className="match-cases-row" key={index}>
              <Input
                value={item.value}
                placeholder="billing"
                onInput={(e) => updateCase(index, { value: e.currentTarget.value })}
              />
              <Input
                value={item.pin}
                placeholder="billing"
                className={duplicates.has(item.pin) ? "match-cases-invalid" : ""}
                onInput={(e) => updateCase(index, { pin: e.currentTarget.value })}
              />
              <Input
                value={item.label}
                placeholder="Billing"
                onInput={(e) => updateCase(index, { label: e.currentTarget.value })}
              />
              <div className="match-cases-actions">
                <button type="button" title="Move up" onClick={() => moveCase(index, -1)} disabled={index === 0}>↑</button>
                <button type="button" title="Move down" onClick={() => moveCase(index, 1)} disabled={index === routes.cases.length - 1}>↓</button>
                <button type="button" title="Remove" onClick={() => removeCase(index)}>×</button>
              </div>
            </div>
          ))}
        </div>

        <button type="button" className="match-cases-add" onClick={addCase}>
          + Add case
        </button>

        <div className="match-cases-default">
          <div className="match-cases-default-title">Default route</div>
          <Input
            value={routes.default.pin}
            placeholder="default"
            className={duplicates.has(routes.default.pin) ? "match-cases-invalid" : ""}
            onInput={(e) => updateDefault({ pin: e.currentTarget.value })}
          />
          <Input
            value={routes.default.label}
            placeholder="Default"
            onInput={(e) => updateDefault({ label: e.currentTarget.value })}
          />
        </div>

        {duplicates.size > 0 ? (
          <div className="match-cases-error">
            Output pins must be unique.
          </div>
        ) : null}
      </div>
    </Field>
  );
}
