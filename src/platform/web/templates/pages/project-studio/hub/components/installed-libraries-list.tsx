import CheckboxField from "@/components/ui/checkbox-field";

/**
 * The RWE libraries installed in this project, each one a checkbox for whether
 * a project bundle carries it.
 */
export default function InstalledLibrariesList({ libraries, selected, onToggle }) {
  const chosen = Array.isArray(selected) ? selected : [];
  return (
    <div className="space-y-2">
      <p className="m-0 text-xs font-medium uppercase tracking-wider text-muted-foreground">Installed libraries</p>
      {libraries.length ? (
        <div className="grid gap-2">
          {libraries.map((item) => {
            const name = item?.name || "";
            const checked = chosen.includes(name);
            return (
              <label key={name} className="flex items-center justify-between gap-3 rounded-md border border-border bg-popover px-3 py-2 text-sm text-foreground">
                <span className="min-w-0">
                  <span className="block truncate">{name}</span>
                  <span className="block text-xs text-muted-foreground">{item?.version || "default"} · {item?.source || "offline"}</span>
                </span>
                <CheckboxField label="" checked={checked} onChange={() => onToggle(name)} />
              </label>
            );
          })}
        </div>
      ) : (
        <p className="m-0 text-xs text-muted-foreground">No project-enabled RWE libraries.</p>
      )}
    </div>
  );
}
