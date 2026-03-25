import Checkbox from "@/components/ui/checkbox";

interface MultiCheckboxOption {
  value: string;
  label: string;
  description?: string;
}

interface Props {
  field: any;
  value: unknown;
  onChange: (val: unknown) => void;
}

export default function NodeFieldMultiCheckbox({ field, value, onChange }: Props) {
  const selected: string[] = Array.isArray(value) ? (value as string[]) : [];
  const options: MultiCheckboxOption[] = Array.isArray(field.options) ? field.options : [];

  function toggle(val: string) {
    const next = selected.includes(val)
      ? selected.filter((v) => v !== val)
      : [...selected, val];
    onChange(next);
  }

  if (options.length === 0) {
    return (
      <div className="text-xs text-body-muted py-1">
        No tools available.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1.5">
      {options.map((opt) => (
        <label key={opt.value} className="flex items-start gap-2 cursor-pointer">
          <Checkbox
            checked={selected.includes(opt.value)}
            onChange={() => toggle(opt.value)}
            className="mt-0.5 shrink-0"
          />
          <span className="flex flex-col min-w-0">
            <span className="text-sm text-body">{opt.label}</span>
            {opt.description && (
              <span className="text-xs text-body-muted leading-tight">{opt.description}</span>
            )}
          </span>
        </label>
      ))}
    </div>
  );
}
