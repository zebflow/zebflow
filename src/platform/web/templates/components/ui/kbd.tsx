import { cx } from "zeb/react";

/**
 * Keyboard key chip — inline mono badge styled like a physical key.
 * Use for showing keyboard shortcut hints.
 */
export default function Kbd({ children, className = "" }) {
  return (
    <kbd className={cx(
      "inline-flex items-center justify-center",
      "font-mono text-[0.65rem] leading-none",
      "px-[0.45em] py-[0.18em]",
      "border border-border border-b-2",
      "rounded-[0.3em]",
      "bg-muted text-muted-foreground",
      "whitespace-nowrap align-middle",
      className,
    )}>
      {children}
    </kbd>
  );
}
