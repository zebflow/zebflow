import Checkbox from "@/components/ui/checkbox";

/**
 * The three install consent flags, as the install request carries them.
 *
 * `execute_schema: true` with `include_schema: false` is a request error, so
 * turning the schema off turns execution off with it and locks the control
 * rather than letting the form assemble a body the API refuses.
 */

function ConsentRow({ id, title, checked, disabled, onToggle, when, whenNot }) {
  return (
    <div
      className={
        disabled
          ? "rounded-lg border border-border bg-accent/40 px-3 py-2.5 opacity-70"
          : "rounded-lg border border-border bg-popover px-3 py-2.5"
      }
    >
      <Checkbox
        id={id}
        name={id}
        label={title}
        checked={!!checked}
        disabled={!!disabled}
        labelClassName="text-sm font-medium text-foreground"
        onChange={(event) => onToggle(!!event?.target?.checked)}
      />
      <p className="m-0 mt-1.5 pl-5 text-xs text-muted-foreground">{checked ? when : whenNot}</p>
    </div>
  );
}

export default function InstallConsent({ scope, onChange, disabled = false }) {
  const includeCode = scope?.include_code !== false;
  const includeSchema = scope?.include_schema !== false;
  const executeSchema = includeSchema && scope?.execute_schema !== false;
  const installsNothing = !includeCode && !includeSchema;

  return (
    <section className="rounded-lg border border-border bg-accent/30 p-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <p className="m-0 text-sm font-semibold text-foreground">What you are consenting to</p>
        <p className="m-0 text-xs text-muted-foreground">Every part is on by default. Turn off what you do not want.</p>
      </div>

      <div className="mt-3 space-y-2">
        <ConsentRow
          id="consent-include-code"
          title="Install the code"
          checked={includeCode}
          disabled={disabled}
          onToggle={(next) => onChange({ include_code: next })}
          when="Pipelines, pages and docs are written into the project, and the pipelines the bundle names active are registered and activated."
          whenNot="No source is written. Only the data model arrives, and no pipeline is registered or activated."
        />
        <ConsentRow
          id="consent-include-schema"
          title="Install the schema"
          checked={includeSchema}
          disabled={disabled}
          onToggle={(next) =>
            onChange(next ? { include_schema: true } : { include_schema: false, execute_schema: false })
          }
          when="The schema and seed .sql files are written into the project repository."
          whenNot="No .sql file is written, and nothing can be run — there would be nothing to run."
        />
        <ConsentRow
          id="consent-execute-schema"
          title="Run the schema now"
          checked={executeSchema}
          disabled={disabled || !includeSchema}
          onToggle={(next) => onChange({ execute_schema: next })}
          when="The SQL below is replayed into stores this install creates, so the project arrives with its tables and its seed rows already in place."
          whenNot="The .sql files land in the repository and nothing runs. You run them yourself, when you are ready."
        />
      </div>

      {includeSchema ? null : (
        <p className="m-0 mt-2 rounded-md border border-border bg-popover px-3 py-2 text-xs text-muted-foreground">
          Running the schema is locked off because the schema is not being written. Asking to run SQL
          that is never written is a request the API refuses rather than quietly reinterprets.
        </p>
      )}

      {installsNothing ? (
        <p className="m-0 mt-2 rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-xs font-medium text-red-700">
          With neither the code nor the schema, this install would install nothing. Turn one of them
          back on.
        </p>
      ) : null}
    </section>
  );
}
