import { cx, Link, useState } from "zeb";

function navRowCx(expanded: boolean, isLight: boolean, routeClass: string) {
  const active = routeClass?.includes("is-active");
  return cx(
    "group flex items-center gap-3 px-3 py-2 text-sm transition-colors",
    expanded ? "justify-start" : "justify-center",
    !active &&
      (isLight
        ? "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
        : "border-l-2 border-dark-menus text-dark-text1 hover:bg-gray-100/10 hover:text-gray-100"),
    active &&
      (isLight
        ? "bg-orange-500 font-medium text-white shadow-sm"
        : "border-l-2 border-solid border-dark-accent1 bg-dark-accent1/10 font-medium text-dark-accent1 shadow-sm"),
    routeClass,
  );
}

export default function PlatformSidebar(props) {
  const nav = props?.nav ?? {};
  const links = nav?.links ?? {};
  const classes = nav?.classes ?? {};
  const theme = props?.theme === "light" ? "light" : "dark";
  const isLight = theme === "light";
  const [expanded, setExpanded] = useState(false);

  return (
    <aside className="fixed left-0 top-0 z-50 flex h-full flex-col overflow-visible">
      <div
        className={cx(
          "flex h-full flex-col overflow-visible border-r shadow-lg transition-all duration-200 ease-out",
          expanded ? "w-60 shadow-2xl" : "w-16 shadow-md",
          isLight ? "border-gray-200 bg-white" : "border-dark-border bg-dark-menus",
        )}
      >
        <div
          className={cx(
            "flex items-center gap-3 border-b px-3 py-3",
            isLight ? "border-gray-200" : "border-dark-border",
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
              <span className="block text-base font-black tracking-tight text-body">
                ZEBFLOW
              </span>
              <span className="block font-mono text-[10px] uppercase tracking-[0.18em] text-body-soft">
                Project Studio
              </span>
            </span>
          </Link>
          {expanded && (
            <button
              type="button"
              className={cx(
                "inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full border transition-colors",
                isLight
                  ? "border-gray-200 bg-gray-50 text-gray-500 hover:bg-gray-100"
                  : "border-gray-600 bg-gray-800 text-gray-400 hover:bg-gray-700",
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
                isLight
                  ? "border-dark-border bg-gray-50 text-gray-500 hover:bg-gray-100"
                  : "border-dark-border bg-gray-800 text-gray-400 hover:bg-gray-700",
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
            className={navRowCx(expanded, isLight, classes.pipelines ?? "")}
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
            href={links.dashboard ?? "#"}
            aria-label="Dashboard"
            className={navRowCx(expanded, isLight, classes.dashboard ?? "")}
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
            className={navRowCx(expanded, isLight, classes.credentials ?? "")}
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
            className={navRowCx(expanded, isLight, classes.databases ?? "")}
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
            className={navRowCx(expanded, isLight, classes.files ?? "")}
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
            className={navRowCx(expanded, isLight, classes.settings ?? "")}
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
              "font-mono text-[0.65rem] tracking-wider text-gray-500 transition-opacity",
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
