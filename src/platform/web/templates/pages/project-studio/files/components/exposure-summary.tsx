import { cx } from "zeb/react";
import { formatBytes } from "@/components/lib/format";
import { ExposureMark } from "@/pages/project-studio/files/components/exposure-mark";

function sum(rules, access) {
  const picked = rules.filter((rule) => rule.access === access);
  return {
    folders: picked.length,
    files: picked.reduce((n, rule) => n + (rule.files || 0), 0),
    bytes: picked.reduce((n, rule) => n + (rule.bytes || 0), 0),
    rules: picked,
  };
}

/**
 * What this project exposes, at the top of the explorer, every time: runs as a
 * site, readable by anyone, and everything else private. A rule jumps the
 * explorer to its folder.
 */
export function ExposureSummary({ exposure, onOpen }) {
  const rules = Array.isArray(exposure.rules) ? exposure.rules : [];
  const execute = sum(rules, "public_execute");
  const read = sum(rules, "public_read");
  const rows = [
    { key: "execute", label: "EXECUTE", totals: execute },
    { key: "read", label: "PUBLIC", totals: read },
  ];
  return (
    <div className="mx-3 mt-2 rounded-md border border-border bg-card text-[0.74rem]">
      {rows.map(({ key, label, totals }) => (
        <div key={key} className="flex flex-wrap items-center gap-2 border-b border-border px-3 py-1.5">
          <span className={cx("font-semibold w-16", key === "execute" ? "text-red-400" : "text-amber-400")}>{label}</span>
          <span className="text-muted-foreground">
            {totals.folders} rule{totals.folders === 1 ? "" : "s"} · {totals.files} file{totals.files === 1 ? "" : "s"} · {formatBytes(totals.bytes)}
          </span>
          {totals.rules.map((rule) => (
            <button
              key={rule.path}
              type="button"
              className="bg-transparent border-0 p-0 cursor-pointer"
              title={`Open ${rule.path}`}
              onClick={() => onOpen(rule.scope === "prefix" ? rule.path : rule.path.split("/").slice(0, -1).join("/"))}
            >
              <span className="inline-flex items-center gap-1">
                <span className="font-mono text-foreground">{rule.path}</span>
                <ExposureMark access={rule.access} serve={rule.serve} />
              </span>
            </button>
          ))}
        </div>
      ))}
      <div className="flex flex-wrap items-center gap-2 px-3 py-1.5 text-muted-foreground">
        <span className="font-semibold w-16">PRIVATE</span>
        <span>everything else</span>
        {exposure.fileHost ? (
          <span className="ml-auto">file host: <span className="font-mono">{exposure.fileHost}</span></span>
        ) : null}
      </div>
    </div>
  );
}
