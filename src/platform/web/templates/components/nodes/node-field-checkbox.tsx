import Checkbox from "@/components/ui/checkbox";
import HelpTooltip from "@/components/ui/help-tooltip";

export default function NodeFieldCheckbox({ field, value, onChange }) {
  return (
    <div className="flex items-center gap-1.5 pt-5">
      <Checkbox
        checked={Boolean(value)}
        label={field.label}
        onChange={(e) => onChange(e.currentTarget.checked)}
      />
      {field.help && <HelpTooltip text={field.help} />}
    </div>
  );
}
