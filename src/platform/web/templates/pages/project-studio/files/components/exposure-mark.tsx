import { cx } from "zeb/react";

/**
 * Exposure is never quiet (`kinds/zebfs-acl` §Authority). A public folder or
 * file pulses amber; one that runs as a site pulses red with the addresses it
 * runs on. Both stop moving for a reader who asked for reduced motion, and
 * both say it in words, so the colour is never the only signal.
 */
export function ExposureMark({ access, inherited = false, serve = [] }) {
  if (access !== "public_read" && access !== "public_execute") return null;
  const execute = access === "public_execute";
  const label = execute ? "EXECUTE" : "PUBLIC";
  const hosts = execute
    ? (Array.isArray(serve) ? serve : []).map((origin) => String(origin).replace(/^https?:\/\//, "").replace(/\/$/, ""))
    : [];
  return (
    <span
      className={cx(
        "inline-flex items-center gap-1.5 shrink-0 rounded px-1.5 py-0.5 text-[0.62rem] font-semibold tracking-wide",
        execute ? "bg-red-500/15 text-red-400" : "bg-amber-400/15 text-amber-400",
      )}
      title={execute ? `Runs as a site on ${hosts.join(", ")}` : "Anyone can read this on the project's file host"}
    >
      <span className="relative inline-flex h-2 w-2">
        <span
          className={cx(
            "absolute inline-flex h-full w-full rounded-full animate-ping motion-reduce:animate-none",
            execute ? "bg-red-500" : "bg-amber-400",
          )}
        />
        <span className={cx("relative inline-flex h-2 w-2 rounded-full", execute ? "bg-red-500" : "bg-amber-400")} />
      </span>
      {label}
      {inherited ? <span className="font-normal opacity-80">· inherited</span> : null}
      {hosts.length > 0 ? <span className="font-normal opacity-90">→ {hosts.join(", ")}</span> : null}
    </span>
  );
}

/** A private folder with something exposed beneath it says so, collapsed or not. */
export function ExposedInsideMark({ count, execute }) {
  if (!count) return null;
  return (
    <span className={cx("shrink-0 text-[0.62rem] font-semibold", execute ? "text-red-400" : "text-amber-400")}>
      ⚠ {count} exposed inside
    </span>
  );
}

/** The rule set on exactly this path, or `null` when its exposure is inherited. */
export function ownRule(rules, path) {
  return (Array.isArray(rules) ? rules : []).find((rule) => rule.path === path) ?? null;
}

/** The exposed rules strictly beneath a folder. */
export function rulesInside(rules, folderPath) {
  const prefix = `${folderPath}/`;
  return (Array.isArray(rules) ? rules : []).filter((rule) => String(rule.path).startsWith(prefix));
}
