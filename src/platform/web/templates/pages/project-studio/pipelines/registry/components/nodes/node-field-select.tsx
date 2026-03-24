import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";

function normalizeOpt(opt: any) {
  return typeof opt === "object" ? opt : { value: opt, label: opt };
}

export default function NodeFieldSelect({ field, value, onChange }) {
  return (
    <Field label={field.label} description={field.help}>
      <Select value={String(value ?? "")} onChange={(e) => onChange(e.currentTarget.value)}>
        {(field.options || []).map((opt: any, i: number) => {
          const o = normalizeOpt(opt);
          return (
            <SelectOption
              key={`${o.value}-${i}`}
              value={String(o.value ?? "")}
              label={String(o.label ?? o.value ?? "")}
            />
          );
        })}
      </Select>
    </Field>
  );
}
