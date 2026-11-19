import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";

export default function SecureRequestVariablesEditor({
  value,
  onChange,
}: {
  value: any[];
  onChange: (next: any[]) => void;
}) {
  const items = Array.isArray(value) ? value : [];

  function updateAt(index: number, patch: Record<string, any>) {
    const next = items.map((item, itemIndex) => (itemIndex === index ? { ...item, ...patch } : item));
    onChange(next);
  }

  function removeAt(index: number) {
    onChange(items.filter((_, itemIndex) => itemIndex !== index));
  }

  function addVariable() {
    const existing = new Set(items.map((item) => String(item?.name || "").trim()));
    let counter = items.length + 1;
    let name = `VAR_${counter}`;
    while (existing.has(name)) {
      counter += 1;
      name = `VAR_${counter}`;
    }
    onChange([
      ...items,
      { name, label: "", value_type: "string", required: true, default_expr: "", description: "" },
    ]);
  }

  return (
    <div className="flex flex-col gap-2">
      {items.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">No runtime variables declared yet.</p>
      ) : null}
      {items.map((item, index) => (
        <div key={index} className="rounded-md border border-border bg-popover px-3 py-3">
          <div className="grid grid-cols-2 gap-3">
            <Field label="Variable Name">
              <Input value={String(item?.name || "")} onInput={(e: any) => updateAt(index, { name: e.target.value })} placeholder="USER_ID" />
            </Field>
            <Field label="Label">
              <Input value={String(item?.label || "")} onInput={(e: any) => updateAt(index, { label: e.target.value })} placeholder="User ID" />
            </Field>
            <Field label="Type">
              <Input value={String(item?.value_type || "")} onInput={(e: any) => updateAt(index, { value_type: e.target.value })} placeholder="string" />
            </Field>
            <Field label="Default Expr">
              <Input value={String(item?.default_expr || "")} onInput={(e: any) => updateAt(index, { default_expr: e.target.value })} placeholder="ctx.nodes.n3.unit.code" />
            </Field>
            <Field label="Description" className="col-span-2">
              <Input value={String(item?.description || "")} onInput={(e: any) => updateAt(index, { description: e.target.value })} placeholder="Shown in the HTTP request node binding editor" />
            </Field>
            <label className="col-span-2 inline-flex items-center gap-2 text-sm text-foreground">
              <input
                type="checkbox"
                checked={item?.required !== false}
                onChange={(e: any) => updateAt(index, { required: !!e.target.checked })}
              />
              Required binding
            </label>
          </div>
          <div className="mt-3 flex justify-end">
            <Button type="button" variant="ghost" size="xs" onClick={() => removeAt(index)}>Remove Variable</Button>
          </div>
        </div>
      ))}
      <div>
        <Button type="button" variant="outline" size="xs" onClick={addVariable}>+ Add Variable</Button>
      </div>
    </div>
  );
}
