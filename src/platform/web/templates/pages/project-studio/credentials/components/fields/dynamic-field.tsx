import { Select, SelectOption } from "@/components/ui/select";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import TagsInput from "@/pages/project-studio/credentials/components/fields/tags-input";
import { generateHex } from "@/pages/project-studio/credentials/components/credential-lib";

export default function DynamicField({ field, secret, onChange }: { field: any; secret: Record<string, any>; onChange: (key: string, val: any) => void }) {
  const value = secret[field.key] ?? field.default ?? "";
  const className = field.fullWidth ? "col-span-2" : "";
  const ft = field.type || "text";

  if (ft === "select") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <Select value={value} onChange={(e) => onChange(field.key, e.target.value)}>
          {(field.options || []).map((o) => <SelectOption key={o.value} value={o.value} label={o.label} />)}
        </Select>
      </Field>
    );
  }

  if (ft === "textarea") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <textarea
          value={value}
          onChange={(e) => onChange(field.key, e.target.value)}
          rows={field.rows || 4}
          placeholder={field.placeholder || ""}
          className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40 disabled:opacity-50"
        />
      </Field>
    );
  }

  if (ft === "tags") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <TagsInput
          value={Array.isArray(secret[field.key]) ? secret[field.key] : []}
          onChange={(val) => onChange(field.key, val)}
          placeholder={field.placeholder || "Add tag…"}
        />
      </Field>
    );
  }

  // text, password, number
  const inputType = ft === "password" ? "password" : ft === "number" ? "number" : "text";

  if (field.generate) {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <div className="flex gap-1.5">
          <Input
            type={inputType}
            value={value}
            onChange={(e) => onChange(field.key, e.target.value)}
            placeholder={field.placeholder || ""}
            className="flex-1"
          />
          <Button type="button" variant="outline" size="sm" onClick={() => onChange(field.key, generateHex(32))}>Generate</Button>
        </div>
      </Field>
    );
  }

  return (
    <Field label={field.label} description={field.help} className={className}>
      <Input
        type={inputType}
        value={value}
        onChange={(e) => onChange(field.key, e.target.value)}
        placeholder={field.placeholder || ""}
      />
    </Field>
  );
}
