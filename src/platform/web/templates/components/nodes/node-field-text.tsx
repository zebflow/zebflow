import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

export default function NodeFieldText({ field, value, onChange }) {
  return (
    <Field label={field.label} description={field.help}>
      <Input
        type="text"
        value={String(value ?? "")}
        placeholder={field.placeholder}
        readOnly={field.readonly}
        disabled={field.readonly}
        onInput={(e) => onChange(e.currentTarget.value)}
      />
    </Field>
  );
}
