import { useState, useRef, useEffect, cx } from "zeb";
import Field from "@/components/ui/field";

function normalizeOpt(opt: any) {
  return typeof opt === "object" ? opt : { value: opt, label: opt };
}

export default function NodeFieldDatalist({ field, value, onChange }) {
  const options = (field.options || []).map(normalizeOpt);
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const selectedLabel =
    options.find((o) => String(o.value) === String(value ?? ""))?.label ||
    String(value ?? "");

  const filtered = search.trim()
    ? options.filter((o) =>
        String(o.label ?? o.value ?? "")
          .toLowerCase()
          .includes(search.toLowerCase())
      )
    : options;

  useEffect(() => {
    if (!open) return;
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setSearch("");
      }
    }
    document.addEventListener("pointerdown", handleClickOutside);
    return () => document.removeEventListener("pointerdown", handleClickOutside);
  }, [open]);

  useEffect(() => {
    if (open && inputRef.current) inputRef.current.focus();
  }, [open]);

  return (
    <Field label={field.label} description={field.help}>
      <div ref={containerRef} className="relative">
        <button
          type="button"
          onClick={() => { setOpen(!open); setSearch(""); }}
          className="flex h-9 w-full items-center justify-between rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-1 text-sm text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40 focus-visible:ring-offset-2"
        >
          <span className={cx("truncate", !value && "text-ui-text-muted")}>
            {value ? selectedLabel : (field.placeholder || "Select...")}
          </span>
          <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4 shrink-0 text-ui-text-muted opacity-50">
            <path d="M7 10l5 5 5-5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </button>

        {open && (
          <div className="absolute z-50 mt-1 w-full rounded-md border border-ui-border bg-ui-bg shadow-lg">
            {options.length > 5 && (
              <div className="p-1.5 border-b border-ui-border">
                <input
                  ref={inputRef}
                  type="text"
                  value={search}
                  placeholder="Search..."
                  onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") { setOpen(false); setSearch(""); }
                    if (e.key === "Enter" && filtered.length === 1) {
                      onChange(String(filtered[0].value ?? ""));
                      setOpen(false);
                      setSearch("");
                    }
                  }}
                  className="w-full h-7 rounded border border-ui-border bg-ui-bg text-ui-text px-2 text-xs focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
                />
              </div>
            )}
            <div className="max-h-48 overflow-y-auto py-1">
              {filtered.length === 0 && (
                <div className="px-3 py-2 text-xs text-ui-text-muted">No matches</div>
              )}
              {filtered.map((o, i) => (
                <button
                  key={`${o.value}-${i}`}
                  type="button"
                  onClick={() => {
                    onChange(String(o.value ?? ""));
                    setOpen(false);
                    setSearch("");
                  }}
                  className={cx(
                    "w-full text-left px-3 py-1.5 text-sm hover:bg-ui-hover cursor-pointer",
                    String(o.value) === String(value ?? "")
                      ? "bg-ui-hover text-ui-text font-medium"
                      : "text-ui-text"
                  )}
                >
                  {String(o.label ?? o.value ?? "")}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </Field>
  );
}
