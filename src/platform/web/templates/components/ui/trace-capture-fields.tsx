import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { traceCaptureFields } from "@/components/lib/trace-capture";

/** Controlled fields shared by project defaults and per-pipeline overrides. */
export default function TraceCaptureFields({ values, onChange, defaults, scope = "project" }) {
  return (
    <fieldset className="col-span-full grid gap-3 border-t border-border pt-4">
      <legend className="text-sm font-semibold">Log data capture</legend>
      <p className="text-xs text-muted-foreground">
        Logged previews only; nodes and function calls receive complete execution data.
        {scope === "pipeline" ? " Leave a field blank to inherit the project default." : " Leave a field blank to use the built-in default."}
        {" "}Changes apply to future runs.
      </p>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {traceCaptureFields.map((field) => {
          const inherited = defaults?.[field.key] ?? field.defaultValue;
          const id = `${scope}-trace-${field.key}`;
          return (
            <Field key={field.key} id={id} label={field.label}>
              <Input
                id={id}
                name={`trace_capture.${field.key}`}
                type="number"
                min={field.min}
                max={field.max}
                step="1"
                placeholder={`Inherit (${inherited})`}
                value={values?.[field.key] ?? ""}
                onInput={(e) => onChange({ ...values, [field.key]: e.currentTarget.value })}
              />
              <small className="pipeline-editor-field-help">{field.help} Default: {inherited}.</small>
            </Field>
          );
        })}
      </div>
    </fieldset>
  );
}
