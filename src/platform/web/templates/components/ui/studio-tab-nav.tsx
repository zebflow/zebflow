export function StudioTabNav({ children }: any) {
  return (
    <nav className="shrink-0 flex items-stretch border-b border-border bg-dark-menus px-[0.625rem]">
      {/* tw-variants hint: active StudioTabLink classes always scanned by RWE Tailwind engine */}
      <span hidden tw-variants="text-dark-accent1 border-dark-accent1 bg-dark-accent1/10" />
      {children}
    </nav>
  );
}

export function StudioTabLink({ href, active, children }: any) {
  return (
    <Link
      href={href ?? "#"}
      className={cx(
        "inline-flex items-center gap-1 px-[0.8rem] py-0.5 border-b-2",
        "text-xs font-mono uppercase tracking-widest",
        active
          ? "text-dark-accent1 border-dark-accent1 bg-dark-accent1/10 font-medium"
          : "text-body-soft border-dark-menus hover:text-dark-accent1 hover:border-dark-accent1 hover:bg-dark-accent1/10"
      )}
    >
      {children}
    </Link>
  );
}
