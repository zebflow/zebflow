import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

export default function NodeFieldDatalist({ field, value, onChange }) {
  const listId = `pe-dl-${field.name}`;
  return (
    <Field label={field.label} description={field.help}>
      <div className="relative">
        <Input
          value={String(value ?? "")}
          list={listId}
          placeholder={field.placeholder}
          readOnly={field.readonly}
          disabled={field.readonly}
          onInput={(e) => onChange(e.currentTarget.value)}
        />
        <datalist id={listId}>
          {(field.options || []).map((opt: any, i: number) => {
            const o = typeof opt === "object" ? opt : { value: opt, label: opt };
            return (
              <option
                key={i}
                value={String(o.value ?? "")}
                label={String(o.label ?? o.value ?? "")}
              />
            );
          })}
        </datalist>
      </div>
    </Field>
  );
}
