export function StudioTabNav({ children }: any) {
  return (
    <nav className="shrink-0 flex items-stretch border-b border-border bg-surface px-[0.625rem]">
      {/* tw-variants hint: active StudioTabLink classes always scanned by RWE Tailwind engine */}
      <span hidden tw-variants="text-body border-b-accent" />
      {children}
    </nav>
  );
}

export function StudioTabLink({ href, active, children }: any) {
  return (
    <Link
      href={href ?? "#"}
      className={cx(
        "inline-flex items-center gap-1 px-[0.8rem] py-0.5 border-b",
        "text-xs font-mono uppercase tracking-widest",
        "text-body-soft hover:text-body hover:bg-surface-3",
        active
          ? "text-body border-b-accent"
          : "border-b-transparent"
      )}
    >
      {children}
    </Link>
  );
}
