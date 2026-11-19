import CheckboxField from "@/components/ui/checkbox-field";
import { formatBytes } from "@/components/lib/format";

/** The SQL files a project bundle can run on install, each one selectable. */
export default function InitialDataList({ items, available, form, patch, onToggle }) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div>
          <p className="m-0 text-xs font-medium uppercase tracking-wider text-muted-foreground">Initial data</p>
          <p className="m-0 text-xs text-muted-foreground">Selected SQL files execute during install after schemas are applied.</p>
        </div>
        <CheckboxField
          label=""
          checked={form.include_initial_data}
          disabled={!available}
          onChange={(e) => patch({
            include_initial_data: e.target.checked,
            initial_data_paths: e.target.checked ? items.map((item) => item.path).filter(Boolean) : [],
          })}
        />
      </div>
      {available ? (
        <div className="grid gap-2">
          {items.map((item) => {
            const path = item?.path || "";
            const checked = (Array.isArray(form.initial_data_paths) ? form.initial_data_paths : []).includes(path);
            return (
              <label key={path} className="flex items-center justify-between gap-3 rounded-md border border-border bg-popover px-3 py-2 text-sm text-foreground">
                <span className="min-w-0">
                  <code className="block truncate text-xs">{path}</code>
                  <span className="block text-xs text-muted-foreground">{item?.engine || "data"} · {item?.statement_count || 0} statement(s) · {formatBytes(item?.size_bytes || 0)}</span>
                </span>
                <CheckboxField label="" checked={checked} onChange={() => onToggle(path)} />
              </label>
            );
          })}
        </div>
      ) : (
        <p className="m-0 text-xs text-muted-foreground">No initial data SQL files found under initial-data, init, or seeds.</p>
      )}
    </div>
  );
}
