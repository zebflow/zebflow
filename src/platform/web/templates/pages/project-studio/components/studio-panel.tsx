import { cx } from "zeb/react";

export function StudioPanel({ className, children }) {
  return (
    <section className={cx("rounded-lg border border-border bg-surface shadow-sm", className)}>
      {children}
    </section>
  );
}

export function StudioPanelHeader({ title, description, trailing, className }) {
  return (
    <div className={cx("flex items-start justify-between gap-3 border-b border-border px-3.5 py-3", className)}>
      <div className="min-w-0">
        <h2 className="truncate text-[0.86rem] font-semibold leading-tight text-body">{title}</h2>
        {description ? (
          <p className="mt-1 text-[0.74rem] leading-5 text-body-soft">{description}</p>
        ) : null}
      </div>
      {trailing ? <div className="shrink-0">{trailing}</div> : null}
    </div>
  );
}

export function StudioPanelBody({ className, children }) {
  return <div className={cx("p-3.5", className)}>{children}</div>;
}

export function StudioMetricGrid({ className, children }) {
  return <dl className={cx("grid gap-2 md:grid-cols-2", className)}>{children}</dl>;
}

export function StudioMetric({ label, value, mono = false, className }) {
  return (
    <div className={cx("rounded-md border border-border-soft bg-surface-2 px-3 py-2", className)}>
      <dt className="font-mono text-[0.63rem] font-medium uppercase tracking-[0.14em] text-body-muted">{label}</dt>
      <dd className={cx("mt-1 min-w-0 break-words text-[0.78rem] font-medium leading-5 text-body", mono && "font-mono text-[0.72rem]")}>
        {value}
      </dd>
    </div>
  );
}

export function StudioStatusBadge({ status }) {
  const value = String(status || "unknown");
  const tone =
    value === "online" || value === "ok" || value === "enabled"
      ? "border-emerald-500/25 bg-emerald-500/10 text-emerald-400"
      : value === "dangling" || value === "warning"
        ? "border-amber-500/25 bg-amber-500/10 text-amber-400"
        : "border-border bg-surface-2 text-body-soft";
  return (
    <span className={cx("inline-flex rounded-full border px-2 py-0.5 font-mono text-[0.62rem] font-semibold uppercase tracking-[0.12em]", tone)}>
      {value}
    </span>
  );
}

export function StudioChip({ children }) {
  return (
    <span className="inline-flex rounded-md border border-border bg-surface px-2 py-1 text-[0.68rem] leading-none text-body-soft">
      {children}
    </span>
  );
}

export function StudioEmptyState({ children }) {
  return (
    <div className="rounded-md border border-dashed border-border bg-surface-2 px-3 py-5 text-center text-[0.76rem] leading-5 text-body-soft">
      {children}
    </div>
  );
}
