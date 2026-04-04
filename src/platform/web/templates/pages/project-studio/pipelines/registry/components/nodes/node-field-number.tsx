import { useState } from "zeb";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";

export default function NodeFieldNumber({ field, value, onChange }) {
  const [raw, setRaw] = useState(value != null ? String(value) : "");

  const handleInput = (e) => {
    const str = e.currentTarget.value;
    setRaw(str);
    const n = str === "" ? null : Number(str);
    onChange(n === null || isNaN(n) ? null : n);
  };

  return (
    <Field label={field.label} description={field.help}>
      <Input
        type="number"
        value={raw}
        placeholder={field.placeholder ?? "0"}
        min={field.min ?? undefined}
        max={field.max ?? undefined}
        step={field.step ?? 1}
        readOnly={field.readonly}
        disabled={field.readonly}
        onInput={handleInput}
      />
    </Field>
  );
}
