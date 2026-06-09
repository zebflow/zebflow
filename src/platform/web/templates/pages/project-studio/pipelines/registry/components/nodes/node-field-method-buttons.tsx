import Field from "@/components/ui/field";
import Button from "@/components/ui/button";

export default function NodeFieldMethodButtons({ field, value, onChange }) {
  const raw = (field.options || []).map((o: any) =>
    typeof o === "object" ? o : { value: String(o), label: String(o) }
  );
  const current = String(value ?? raw[0]?.value ?? "");

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-wrap gap-1">
        {raw.map((opt: any) => (
          <Button
            key={opt.value}
            type="button"
            variant={current === opt.value ? "primary" : "outline"}
            size="xs"
            onClick={() => onChange(opt.value)}
          >
            {opt.label || opt.value}
          </Button>
        ))}
      </div>
    </Field>
  );
}
