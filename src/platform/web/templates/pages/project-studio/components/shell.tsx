/**
 * Project studio chrome only — not a global layout. Import from here:
 * `@/pages/project-studio/components/shell`. Shared UI for all apps stays under `@/components/`.
 *
 * Layout uses Tailwind utilities (RWE `data-rwe-tw`); `--studio-*` / `--zf-ui-*` come from `[data-studio-theme]` in `pages/project-studio/styles.css` (SSR-safe).
 */
import { useState, useEffect, Link, cx } from "zeb";
import PlatformSidebar from "@/pages/project-studio/components/platform-sidebar";
import { initProjectShellBehavior } from "@/pages/project-studio/components/studio-shell-behavior";
import ConsolePanel from "@/components/ui/console-panel";
import Button from "@/components/ui/button";
import { HomeIcon, MoonIcon, SunIcon, TerminalIcon } from "@/pages/project-studio/components/icons";
import { GitRepoPanel } from "@/pages/project-studio/components/git-repo-panel";
import { SessionPanel } from "@/pages/project-studio/components/session-panel";
import { ConsoleOutput } from "@/pages/project-studio/components/console-output";
import { AutoOverlay } from "@/pages/project-studio/components/auto-overlay";
import { StudioChromeProvider, useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";

function ConsoleSlot({ owner, project, children }) {
  const { consoleOpen } = useStudioChrome();
  return (
    <ConsolePanel isOpen={consoleOpen} owner={owner} project={project}>
      {children}
    </ConsolePanel>
  );
}

export default function ProjectStudioShell(props) {
  const [theme, setTheme] = useState("dark");
  const nav = props?.nav ?? {};
  const owner = props?.owner ?? "";
  const project = props?.project ?? "";
  const isLight = theme === "light";

  useEffect(() => {
    initProjectShellBehavior();
  }, []);

  return (
    <div
      data-studio-theme={isLight ? "light" : "dark"}
      className={cx(
        "flex h-screen w-screen flex-col overflow-hidden",
        isLight ? "bg-slate-50" : "bg-slate-950",
      )}
    >
      <StudioChromeProvider>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <PlatformSidebar nav={nav} theme={theme} />

          <main
            className={cx(
              "ml-16 flex h-screen min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
              isLight ? "bg-slate-50" : "bg-slate-950",
            )}
          >
            <header
              className={cx(
                "relative z-10 flex h-10 shrink-0 items-center border-b px-4 backdrop-blur-md",
                isLight ? "border-slate-200 bg-white/90" : "border-slate-700/60 bg-slate-950/80",
              )}
            >
              <div className="flex h-full w-full items-center justify-between">
                <nav className="flex min-w-0 items-center gap-2 text-[0.78rem] leading-none">
                  <Link
                    href="/home"
                    className={cx(
                      "transition-colors",
                      isLight ? "text-slate-500 hover:text-slate-900" : "text-slate-400 hover:text-slate-100",
                    )}
                    aria-label="Go to home"
                  >
                    <HomeIcon />
                  </Link>
                  <span
                    className={cx("select-none", isLight ? "text-slate-300" : "text-slate-600")}
                  >
                    /
                  </span>
                  <Link
                    href={props?.projectHref ?? "#"}
                    className={cx(
                      "truncate transition-colors",
                      isLight ? "text-slate-500 hover:text-slate-900" : "text-slate-400 hover:text-slate-100",
                    )}
                  >
                    {props?.projectLabel ?? "Project"}
                  </Link>
                  <span
                    className={cx("select-none", isLight ? "text-slate-300" : "text-slate-600")}
                  >
                    /
                  </span>
                  <span
                    className={cx("font-medium", isLight ? "text-slate-900" : "text-slate-100")}
                    data-rwe-breadcrumb
                  >
                    {props?.currentMenu ?? "Workspace"}
                  </span>
                </nav>

                <div className="flex items-center gap-1.5">
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => setTheme(isLight ? "dark" : "light")}
                    title="Toggle theme"
                    className={cx(
                      isLight
                        ? "border-slate-200 bg-white text-slate-700"
                        : "border-slate-600 bg-slate-900 text-slate-200",
                    )}
                  >
                    {isLight ? <SunIcon /> : <MoonIcon />}
                  </Button>

                  <Button
                    variant="outline"
                    size="icon"
                    title="Console (` to toggle)"
                    data-console-trigger="true"
                    data-owner={owner}
                    data-project={project}
                    className={cx(
                      isLight
                        ? "border-slate-200 bg-white text-slate-700"
                        : "border-slate-600 bg-slate-900 text-slate-200",
                    )}
                  >
                    <TerminalIcon />
                  </Button>

                  <GitRepoPanel owner={owner} project={project} />
                  <SessionPanel owner={owner} project={project} />
                </div>
              </div>
            </header>

            <section className="flex min-h-0 flex-1 flex-col overflow-hidden" data-rwe-outlet>
              {props?.children}
            </section>
          </main>
        </div>

        <ConsoleSlot owner={owner} project={project}>
          <ConsoleOutput />
        </ConsoleSlot>

        <AutoOverlay />
      </StudioChromeProvider>
    </div>
  );
}
