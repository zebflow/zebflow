import Field from "@/components/ui/field";
import Button from "@/components/ui/button";

export default function NodeFieldMethodButtons({ field, value, onChange }) {
  const options = (field.options || []).map((o: any) =>
    typeof o === "object" ? o.value : String(o)
  );
  const current = String(value || "GET").toUpperCase();

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex flex-wrap gap-1">
        {options.map((method: string) => (
          <Button
            key={method}
            type="button"
            variant={current === method.toUpperCase() ? "primary" : "outline"}
            size="xs"
            onClick={() => onChange(method.toUpperCase())}
          >
            {method}
          </Button>
        ))}
      </div>
    </Field>
  );
}
