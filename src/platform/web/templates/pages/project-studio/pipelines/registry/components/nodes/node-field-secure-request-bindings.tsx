import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

function normalizeValue(value: unknown): Record<string, string> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>).map(([key, item]) => [
      key,
      typeof item === "string" ? item : String(item ?? ""),
    ])
  );
}

export default function NodeFieldSecureRequestBindings({ field, value, onChange }) {
  const bindings = normalizeValue(value);
  const credential = field?.secureRequestCredential ?? null;
  const variables = Array.isArray(field?.secureRequestVariables)
    ? field.secureRequestVariables
    : [];

  function updateBinding(name: string, nextValue: string) {
    onChange({ ...bindings, [name]: nextValue });
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-col gap-3">
        {!credential ? (
          <div className="rounded-md border border-dashed border-ui-border px-3 py-3 text-sm text-body-soft">
            Select a secure request profile first. Its required variables will appear here.
          </div>
        ) : variables.length === 0 ? (
          <div className="rounded-md border border-ui-border bg-ui-bg px-3 py-3 text-sm text-body-soft">
            This profile does not declare any runtime variables.
          </div>
        ) : (
          variables.map((item: any) => {
            const name = String(item?.name || "").trim();
            const label = String(item?.label || "").trim() || name;
            const description = String(item?.description || "").trim();
            const valueType = String(item?.value_type || "").trim();
            const required = item?.required === true;
            return (
              <div key={name} className="rounded-md border border-ui-border bg-ui-bg px-3 py-3">
                <div className="mb-2 flex flex-wrap items-center gap-2">
                  <div className="text-sm font-medium text-body">{label}</div>
                  <code className="rounded bg-surface-2 px-1.5 py-0.5 text-xs text-body-soft">{name}</code>
                  {required ? (
                    <span className="rounded bg-danger/10 px-1.5 py-0.5 text-[11px] font-medium text-danger">Required</span>
                  ) : (
                    <span className="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] font-medium text-body-soft">Optional</span>
                  )}
                  {valueType ? (
                    <span className="rounded bg-surface-2 px-1.5 py-0.5 text-[11px] font-medium text-body-soft">{valueType}</span>
                  ) : null}
                </div>
                {description ? (
                  <p className="mb-2 text-xs leading-relaxed text-body-soft">{description}</p>
                ) : null}
                <Input
                  type="text"
                  value={bindings[name] || ""}
                  placeholder={String(item?.default_expr || "").trim() || "input.player_id"}
                  onInput={(e) => updateBinding(name, e.currentTarget.value)}
                />
                <p className="mt-1 text-[11px] text-body-soft">
                  Enter a JS expression, for example <code>input.player_id</code> or <code>ctx.nodes.n3.unit.code</code>.
                </p>
              </div>
            );
          })
        )}
      </div>
    </Field>
  );
}
