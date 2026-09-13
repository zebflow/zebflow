import { cx, Link, useState } from "zeb/react";

function navRowCx(expanded: boolean, routeClass: string) {
  const active = routeClass?.includes("is-active");
  return cx(
    "group flex items-center gap-3 px-3 py-1.5 text-[0.78rem] transition-colors",
    expanded ? "justify-start" : "justify-center",
    !active &&
      ("border-l-2 border-transparent text-muted-foreground hover:bg-accent hover:text-foreground"),
    active &&
      ("border-l-2 border-solid border-primary bg-primary/10 font-medium text-primary shadow-sm"),
    routeClass,
  );
}

export default function PlatformSidebar(props) {
  const nav = props?.nav ?? {};
  const links = nav?.links ?? {};
  const classes = nav?.classes ?? {};
  const [expanded, setExpanded] = useState(false);

  return (
    <aside className="fixed left-0 top-0 z-50 flex h-full flex-col overflow-visible">
      <div
        className={cx(
          "flex h-full flex-col overflow-visible border-r shadow-lg transition-all duration-200 ease-out",
          expanded ? "w-60 shadow-2xl" : "w-16 shadow-md",
          "border-border bg-popover",
        )}
      >
        <div
          className={cx(
            "flex items-center gap-3 border-b px-3 py-2.5",
            "border-border",
            expanded ? "justify-between" : "justify-center",
          )}
        >
          <Link
            href="/home"
            className={cx(
              "flex min-w-0 items-center gap-3",
              expanded ? "justify-start" : "justify-center",
            )}
          >
            <img src="/assets/branding/logo.svg" alt="Zebflow logo" className="h-9 w-9 shrink-0" />
            <span className={cx("min-w-0", !expanded && "hidden")}>
              <span className="block text-[0.95rem] font-semibold tracking-tight text-foreground">
                zebflow
              </span>
              <span className="block font-mono text-[9.5px] uppercase tracking-[0.16em] text-muted-foreground">
                Project Studio
              </span>
            </span>
          </Link>
          {expanded && (
            <button
              type="button"
              className={cx(
                "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full border transition-colors",
                "border-border bg-muted text-muted-foreground hover:bg-accent",
              )}
              aria-label="Collapse sidebar"
              aria-expanded={expanded}
              onClick={() => setExpanded(false)}
            >
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5 rotate-180">
                <path
                  d="M8 5l8 7-8 7"
                  stroke="currentColor"
                  style={{ strokeWidth: "5" }}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </button>
          )}
        </div>
        {!expanded && (
          <div className="flex justify-center py-2 mt-2">
            <button
              type="button"
              className={cx(
                "inline-flex h-7 w-7 items-center justify-center rounded-full border transition-colors",
                "border-border bg-muted text-muted-foreground hover:bg-accent",
              )}
              aria-label="Expand sidebar"
              onClick={() => setExpanded(true)}
            >
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path
                  d="M8 5l8 7-8 7"
                  stroke="currentColor"
                  style={{ strokeWidth: "5" }}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </button>
          </div>
        )}

        <nav className="flex flex-1 flex-col gap-1 overflow-visible p-2">
          <Link
            href={links.pipelines_registry ?? "#"}
            aria-label="Pipelines"
            className={navRowCx(expanded, classes.pipelines ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <circle cx="7" cy="7" r="2.2" stroke="currentColor" strokeWidth="1.6" />
                <circle cx="17" cy="7" r="2.2" stroke="currentColor" strokeWidth="1.6" />
                <circle cx="12" cy="17" r="2.2" stroke="currentColor" strokeWidth="1.6" />
                <path
                  d="M9.2 8.4l1.9 5.2M14.8 8.4l-1.9 5.2"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Pipelines</span>
          </Link>

          <Link
            href={links.hub ?? "#"}
            aria-label="Hub"
            className={navRowCx(expanded, classes.hub ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path d="M5 8.5h14v10H5z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round" />
                <path d="M8 8.5V6.8a2 2 0 114 0v1.7M12 8.5V6.8a2 2 0 114 0v1.7" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
                <path d="M9 12h6M12 9.5v5" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Hub</span>
          </Link>

          <Link
            href={links.dashboard ?? "#"}
            aria-label="Dashboard"
            className={navRowCx(expanded, classes.dashboard ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path d="M4 13h6v7H4zM14 4h6v16h-6z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Dashboard</span>
          </Link>

          <Link
            href={links.credentials ?? "#"}
            aria-label="Credentials"
            className={navRowCx(expanded, classes.credentials ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path
                  d="M12 14a3 3 0 100-6 3 3 0 000 6zM6 10V8a6 6 0 1112 0v2M5 10h14v9H5z"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Credentials</span>
          </Link>

          <Link
            href={links.db_connections ?? "#"}
            aria-label="Databases"
            className={navRowCx(expanded, classes.databases ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" strokeWidth="1.8" />
                <path d="M5 6v8c0 1.7 3.1 3 7 3s7-1.3 7-3V6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Databases</span>
          </Link>

          <Link
            href={links.files ?? "#"}
            aria-label="Files"
            className={navRowCx(expanded, classes.files ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path d="M4 6h6l2 2h8v10H4z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Files</span>
          </Link>

          <Link
            href={links.settings ?? "#"}
            aria-label="Settings"
            className={navRowCx(expanded, classes.settings ?? "")}
          >
            <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center">
              <svg viewBox="0 0 24 24" fill="none" className="h-5 w-5">
                <path
                  d="M12 15.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7z"
                  stroke="currentColor"
                  strokeWidth="1.8"
                />
                <path
                  d="M19 12a7 7 0 01-.1 1.1l1.8 1.4-1.8 3.1-2.2-.8a7.3 7.3 0 01-1.9 1.1l-.3 2.3h-3.6l-.3-2.3a7.3 7.3 0 01-1.9-1.1l-2.2.8-1.8-3.1 1.8-1.4A7 7 0 015 12c0-.4 0-.8.1-1.1L3.3 9.5l1.8-3.1 2.2.8c.6-.5 1.2-.9 1.9-1.1l.3-2.3h3.6l.3 2.3c.7.2 1.3.6 1.9 1.1l2.2-.8 1.8 3.1-1.8 1.4c.1.3.1.7.1 1.1z"
                  stroke="currentColor"
                  strokeWidth="1.4"
                  strokeLinejoin="round"
                />
              </svg>
            </span>
            <span className={cx("whitespace-nowrap", !expanded && "hidden")}>Settings</span>
          </Link>
        </nav>

        <div className="flex justify-center p-2">
          <span
            className={cx(
              "font-mono text-[0.65rem] tracking-wider text-muted-foreground transition-opacity",
              !expanded && "hidden",
            )}
            aria-hidden={!expanded}
          >
            v0.1.1
          </span>
        </div>
      </div>
    </aside>
  );
}
