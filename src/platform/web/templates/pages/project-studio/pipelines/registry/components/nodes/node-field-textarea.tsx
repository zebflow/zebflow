import Field from "@/components/ui/field";
import Textarea from "@/components/ui/textarea";

export default function NodeFieldTextarea({ field, value, onChange }) {
  return (
    <Field label={field.label} description={field.help}>
      <Textarea
        value={String(value ?? "")}
        rows={field.rows || 5}
        placeholder={field.placeholder}
        readOnly={field.readonly}
        disabled={field.readonly}
        onInput={(e) => onChange(e.currentTarget.value)}
      />
    </Field>
  );
}
