import { cx } from "zeb/react";
import { kindOf, verbOf } from "@/components/hub/hub-kinds";

/** Where a package came from, in the one word the contract uses. */
function originLabel(item) {
  const source = String(item?.source || "").toLowerCase();
  if (source.includes("local") || source.includes("blessed")) return "blessed";
  if (source.includes("static")) return "static";
  return item?.repository_title ? String(item.repository_title).toLowerCase() : "public";
}

/**
 * One package in the list.
 *
 * Carries the verb as a badge because it is the difference between something
 * you can update later and something that becomes yours forever, and a reader
 * choosing between two rows has no other way to tell.
 */
export default function HubItemRow({ item, selected, onSelect }) {
  const kind = kindOf(item?.asset_kind);
  const verb = verbOf(item?.asset_kind);
  const problem = item?.status && item.status !== "resolved" ? item.status : "";

  return (
    <button
      type="button"
      data-hub-item={item?.package_id}
      data-hub-kind={kind.id}
      onClick={() => onSelect(item)}
      className={cx(
        "flex w-full items-start gap-2.5 border-b border-border px-3 py-2.5 text-left transition-colors",
        selected ? "bg-accent" : "hover:bg-accent/50",
      )}
    >
      <span className={cx("mt-[2px] shrink-0 text-[0.9rem] leading-none", kind.tone)} aria-hidden="true">
        {kind.glyph}
      </span>

      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-2">
          <span className="truncate text-[0.82rem] font-medium text-foreground">
            {item?.title || item?.package_id}
          </span>
          <span
            className={cx(
              "shrink-0 rounded px-1.5 py-[1px] text-[0.6rem] font-semibold uppercase tracking-[0.08em]",
              verb === "install"
                ? "bg-accent text-muted-foreground"
                : "bg-warning/15 text-warning",
            )}
            title={
              verb === "install"
                ? "Managed: recorded in zeb.lock, updatable, removable"
                : "Copied into your project: it becomes your file, with no update path"
            }
          >
            {verb}
          </span>
        </span>
        <span className="mt-0.5 block truncate text-[0.72rem] text-muted-foreground">
          {item?.summary || item?.description || kind.one} · {originLabel(item)}
        </span>
      </span>

      {problem ? (
        <span className="shrink-0 text-[0.66rem] font-medium text-warning" title={problem}>
          {problem === "missing" ? "missing" : "digest"}
        </span>
      ) : item?.packed_version || item?.latest_version ? (
        <span className="shrink-0 font-mono text-[0.66rem] text-muted-foreground">
          {item.packed_version || item.latest_version}
        </span>
      ) : null}
    </button>
  );
}
