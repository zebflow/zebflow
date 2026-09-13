import { cx, useState } from "zeb/react";

/**
 * The frame both galleries share: header with the theme toggle, a sidebar of
 * groups, one visible section. The theme toggle flips `.dark` on the frame,
 * which is all a theme is.
 *
 * Two galleries, because two kits: the studio's `components/ui/` at
 * /dev/design-system and `zeb/ui` at /dev/design-system/ui. They export the
 * same names, and the compiler's flat bundle declares each name once per
 * page — so they cannot share one. For the same reason this shell imports
 * no primitive from either kit: its theme switch and rule are plain markup.
 */

const KITS = [
  { href: "/dev/design-system", label: "Studio kit", hint: "components/ui/" },
  { href: "/dev/design-system/ui", label: "zeb/ui", hint: "what project pages import" },
];

export default function GalleryShell({ kit, title, groups, initial, footnote, children }) {
  const [group, setGroup] = useState(initial);
  const [dark, setDark] = useState(true);

  return (
    <div className={cx("flex h-screen flex-col overflow-hidden bg-background text-foreground", dark ? "dark" : "")}>
      <header className="flex h-12 shrink-0 items-center gap-4 border-b border-border bg-card px-6">
        <a href="/home" className="flex items-center gap-1.5 font-mono text-xs text-muted-foreground transition-colors hover:text-foreground">
          ← home
        </a>
        <div className="h-4 w-px bg-border" />
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-bold tracking-tight text-foreground">{title}</span>
          <span className="font-mono text-[0.65rem] uppercase tracking-widest text-muted-foreground">Zebflow Platform</span>
        </div>
        <div className="ml-6 flex items-center gap-1">
          {KITS.map((k) => (
            <a
              key={k.href}
              href={k.href}
              className={cx(
                "rounded-md px-2.5 py-1 text-xs transition-colors",
                k.href === kit ? "bg-accent text-accent-foreground font-medium" : "text-muted-foreground hover:text-foreground",
              )}
              title={k.hint}
            >
              {k.label}
            </a>
          ))}
        </div>
        <div className="flex-1" />
        <button
          type="button"
          role="switch"
          aria-checked={dark ? "true" : "false"}
          onClick={() => setDark(!dark)}
          className="inline-flex items-center gap-2 text-sm text-foreground"
        >
          <span className={cx("relative inline-flex h-5 w-9 items-center rounded-full border border-transparent transition-colors", dark ? "bg-primary" : "bg-input")}>
            <span className={cx("block h-4 w-4 rounded-full bg-background shadow-sm transition-transform", dark ? "translate-x-4" : "translate-x-0")} />
          </span>
          {dark ? "dark" : "light"}
        </button>
      </header>

      <div className="flex min-h-0 flex-1">
        <nav className="flex w-52 shrink-0 flex-col gap-0.5 border-r border-sidebar-border bg-sidebar px-2 pb-6 pt-4 text-sidebar-foreground">
          {groups.map((g) => (
            <button
              key={g.id}
              type="button"
              onClick={() => setGroup(g.id)}
              className={cx(
                "w-full rounded-lg px-3 py-2 text-left transition-colors",
                group === g.id ? "bg-sidebar-accent text-sidebar-accent-foreground" : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-foreground",
              )}
            >
              <span className={cx("block text-sm", group === g.id ? "font-semibold" : "")}>{g.label}</span>
              <span className="block font-mono text-[0.62rem] text-muted-foreground">{g.hint}</span>
            </button>
          ))}
          <div className="mt-auto px-1 pt-6">
            <div className="h-px w-full bg-border" />
            <p className="mt-4 font-mono text-[0.65rem] leading-relaxed text-muted-foreground">{footnote}</p>
          </div>
        </nav>

        <main data-ds-content="true" className="flex-1 overflow-y-auto px-10 py-8" style={{ scrollbarWidth: "thin", scrollbarColor: "var(--border) transparent" }}>
          {children(group)}
        </main>
      </div>
    </div>
  );
}
