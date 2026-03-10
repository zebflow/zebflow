export default function ColorSwatch({ name, value, className }) {
  return (
    <div className={cx("flex flex-col gap-2", className)}>
      <div
        className="h-10 w-full rounded-lg border border-[var(--studio-border)]"
        style={{ background: value }}
      />
      <div className="space-y-0.5">
        <div className="text-[0.7rem] font-mono leading-tight text-[var(--studio-text)] truncate">{name}</div>
        <div className="text-[0.65rem] font-mono leading-tight text-[var(--studio-text-soft)] opacity-70">{value}</div>
      </div>
    </div>
  );
}
