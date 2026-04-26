/**
 * Project studio chrome only — not a global layout. Import from here:
 * `@/pages/project-studio/components/shell`. Shared UI for all apps stays under `@/components/`.
 *
 * Layout uses Tailwind utilities (RWE `data-rwe-tw`); `--studio-*` / `--zf-ui-*` come from `[data-studio-theme]` in `pages/project-studio/styles.css` (SSR-safe).
 */
import { useEffect, useState, Link, cx } from "zeb";
import PlatformSidebar from "@/pages/project-studio/components/platform-sidebar";
import Button from "@/components/ui/button";
import { HelpIcon, HomeIcon, MoonIcon, PreferencesIcon, SunIcon, TerminalIcon } from "@/pages/project-studio/components/icons";
import { GitRepoPanel } from "@/pages/project-studio/components/git-repo-panel";
import { SessionPanel } from "@/pages/project-studio/components/session-panel";
import { AutoOverlay } from "@/pages/project-studio/components/auto-overlay";
import { StudioChromeProvider, useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";
import { FileSearchProvider } from "@/pages/project-studio/components/file-search-context";
import ProjectConsole from "@/pages/project-studio/components/project-console";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import {
  getDefaultEditorPreferences,
  readEditorPreferences,
  subscribeEditorPreferences,
  writeEditorPreferences,
} from "@/pages/project-studio/components/editor-preferences";
import { ensureStudioClipboard } from "@/pages/project-studio/components/studio-clipboard";
import { renderMarkdown } from "zeb/markdown";

function ConsoleSlot({ owner, project }) {
  return <ProjectConsole owner={owner} project={project} />;
}

function TerminalToggleButton({ isLight }) {
  const { toggleConsole, setActivePanel } = useStudioChrome();
  return (
    <button
      type="button"
      className={cx(
        isLight ? "bg-gray-200 text-gray-700" : "bg-dark-accent3 !text-dark-menus",
        "flex h-9 w-9 items-center justify-center rounded-none",
      )}
      onClick={() => {
        setActivePanel(null);
        toggleConsole();
      }}
      aria-label="Toggle console"
      title="Console"
    >
      <TerminalIcon />
    </button>
  );
}

function HelpDialog({ owner, project, isLight, open, onClose }) {
  const [sections, setSections] = useState([]);
  const [activeSection, setActiveSection] = useState("start-here");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open || sections.length) return;
    let cancelled = false;
    setLoading(true);
    setError("");
    fetch(`/api/projects/${owner}/${project}/help`, {
      headers: { Accept: "application/json" },
      credentials: "same-origin",
    })
      .then((res) => res.json())
      .then((json) => {
        if (cancelled) return;
        const items = Array.isArray(json?.sections) ? json.sections : [];
        setSections(items);
        const allIds = flattenHelpSectionIds(items);
        if (allIds.length && !allIds.includes(activeSection)) {
          setActiveSection(allIds[0] || "start-here");
        }
      })
      .catch((err) => {
        if (!cancelled) setError(String(err?.message || err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, owner, project]);

  if (!open) return null;

  const active = findHelpSectionById(sections, activeSection) || sections[0] || null;

  return (
    <div className="fixed inset-0 z-[120] flex items-center justify-center p-4">
      <button
        type="button"
        className="absolute inset-0 bg-black/55 backdrop-blur-[1px]"
        onClick={onClose}
        aria-label="Close help"
      />
      <div
        className={cx(
          "relative flex h-[min(82vh,54rem)] w-[min(92vw,72rem)] overflow-hidden rounded-2xl border shadow-2xl",
          isLight ? "border-gray-200 bg-white" : "border-dark-border bg-dark-background",
        )}
      >
        <aside
          className={cx(
            "flex w-64 shrink-0 flex-col border-r",
            isLight ? "border-gray-200 bg-gray-50" : "border-dark-border bg-dark-menus/60",
          )}
        >
          <div className="border-b border-inherit px-4 py-3">
            <p className={cx("text-sm font-semibold", isLight ? "text-gray-900" : "text-dark-text1")}>Help</p>
          </div>
          <div className="flex-1 overflow-auto px-2 py-2">
            {sections.map((section) => (
              <HelpTreeNode
                key={section?.id}
                node={section}
                level={0}
                activeId={active?.id}
                isLight={isLight}
                onSelect={setActiveSection}
              />
            ))}
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col">
          <div
            className={cx(
              "flex items-center justify-between border-b px-5 py-3",
              isLight ? "border-gray-200 bg-white" : "border-dark-border bg-dark-background",
            )}
          >
            <div>
              <p className={cx("text-base font-semibold", isLight ? "text-gray-900" : "text-dark-text1")}>
                {active?.title || "Help"}
              </p>
            </div>
            <Button type="button" variant="ghost" size="sm" onClick={onClose}>Close</Button>
          </div>

          <div className="min-h-0 flex-1 overflow-auto px-5 py-4">
            {loading ? (
              <p className={cx("text-sm", isLight ? "text-gray-600" : "text-dark-text1")}>Loading help…</p>
            ) : error ? (
              <p className="text-sm text-red-400">{error}</p>
            ) : active ? (
              <div
                className={cx(
                  "max-w-none prose prose-sm",
                  isLight ? "" : "prose-invert",
                )}
                dangerouslySetInnerHTML={{ __html: renderMarkdown(active?.content || "") }}
              />
            ) : (
              <p className={cx("text-sm", isLight ? "text-gray-600" : "text-dark-text1")}>No help content available.</p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function flattenHelpSectionIds(nodes) {
  const out = [];
  const visit = (items) => {
    for (const item of items || []) {
      if (item?.id) out.push(item.id);
      if (Array.isArray(item?.children) && item.children.length) {
        visit(item.children);
      }
    }
  };
  visit(nodes);
  return out;
}

function findHelpSectionById(nodes, id) {
  for (const item of nodes || []) {
    if (item?.id === id) return item;
    if (Array.isArray(item?.children) && item.children.length) {
      const child = findHelpSectionById(item.children, id);
      if (child) return child;
    }
  }
  return null;
}

function HelpTreeNode({ node, level, activeId, isLight, onSelect }) {
  const active = activeId === node?.id;
  const hasChildren = Array.isArray(node?.children) && node.children.length > 0;
  return (
    <div className="mb-1">
      <button
        type="button"
        onClick={() => onSelect(node?.id)}
        className={cx(
          "flex w-full items-center rounded-lg px-3 py-2 text-left text-sm transition-colors",
          level === 0 ? "font-semibold" : level === 1 ? "font-medium" : "",
          active
            ? isLight
              ? "bg-orange-500 text-white"
              : "bg-dark-accent1/15 text-dark-accent1"
            : isLight
              ? "text-gray-700 hover:bg-gray-100"
              : "text-dark-text1 hover:bg-dark-border",
        )}
        style={{ paddingLeft: `${0.75 + level * 0.85}rem` }}
      >
        {node?.title}
      </button>
      {hasChildren ? (
        <div className="mt-1">
          {node.children.map((child) => (
            <HelpTreeNode
              key={child?.id}
              node={child}
              level={level + 1}
              activeId={activeId}
              isLight={isLight}
              onSelect={onSelect}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export default function ProjectStudioShell(props) {
  const [theme, setTheme] = useState("dark");
  const [helpOpen, setHelpOpen] = useState(false);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [editorPreferences, setEditorPreferences] = useState(getDefaultEditorPreferences());
  const nav = props?.nav ?? {};
  const owner = props?.owner ?? "";
  const project = props?.project ?? "";
  const isLight = theme === "light";

  useEffect(() => {
    setEditorPreferences(readEditorPreferences());
    ensureStudioClipboard();
    return subscribeEditorPreferences((prefs) => setEditorPreferences(prefs));
  }, []);

  return (
    <div
      data-studio-theme={isLight ? "light" : "dark"}
      className={cx(
        "flex h-screen w-screen flex-col overflow-hidden",
        isLight ? "bg-gray-50" : "bg-dark-background",
      )}
    >
      <FileSearchProvider owner={owner} project={project}>
      <StudioChromeProvider>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <PlatformSidebar nav={nav} theme={theme} />

          <main
            className={cx(
              "ml-16 flex h-screen min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
              isLight ? "bg-gray-50" : "bg-dark-background",
            )}
            tw-variants={"bg-gray-50 bg-dark-background"}
          >
            <header
              className={cx(
                "relative z-10 flex h-10 shrink-0 items-center border-b px-4 backdrop-blur-md",
                isLight ? "border-gray-200 bg-white/90" : "border-dark-border bg-dark-background",
              )}
            >
              <div className="flex h-full w-full items-center justify-between">
                <nav className="flex min-w-0 items-center gap-2 text-[0.78rem] leading-none">
                  <Link
                    href="/home"
                    className={cx(
                      "transition-colors",
                      isLight ? "text-gray-500 hover:text-gray-900" : "text-dark-text1 hover:text-gray-100",
                    )}
                    aria-label="Go to home"
                  >
                    <HomeIcon />
                  </Link>
                  <span
                    className={cx("select-none", isLight ? "text-gray-300" : "text-dark-text1")}
                  >
                    /
                  </span>
                  <Link
                    href={props?.projectHref ?? "#"}
                    className={cx(
                      "truncate transition-colors",
                      isLight ? "text-gray-500 hover:text-gray-900" : "text-dark-text1 hover:text-gray-100",
                    )}
                  >
                    {props?.projectLabel ?? "Project"}
                  </Link>
                  <span
                    className={cx("select-none", isLight ? "text-gray-300" : "text-dark-text1")}
                  >
                    /
                  </span>
                  <span
                    className={cx("font-medium", isLight ? "text-gray-900" : "text-dark-text1")}
                    data-rwe-breadcrumb
                  >
                    {props?.currentMenu ?? "Workspace"}
                  </span>
                </nav>

                <div className="flex items-center gap-0.5">
                  <button
                    type="button"
                    className="flex h-9 w-9 items-center justify-center rounded-none bg-orange-500 text-white transition-colors hover:bg-orange-400"
                    onClick={() => setPreferencesOpen(true)}
                    aria-label="Open preferences"
                    title="Preferences"
                  >
                    <PreferencesIcon className="h-5 w-5" />
                  </button>
                  <button
                    type="button"
                    className={cx(
                      isLight
                        ? "bg-gray-200 text-gray-700"
                        : "bg-dark-accent3 !text-dark-menus",
                      "flex items-center justify-center h-9 w-9 rounded-none",
                    )}
                    onClick={() => setHelpOpen(true)}
                    aria-label="Open help"
                    title="Help"
                  >
                    <HelpIcon className="w-5 h-5" />
                  </button>
                  <TerminalToggleButton isLight={isLight} />

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

        <ConsoleSlot owner={owner} project={project} />

        <AutoOverlay />
        <Dialog open={preferencesOpen} onOpenChange={setPreferencesOpen}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>Preferences</DialogTitle>
            </DialogHeader>
            <div className="space-y-4 px-6 pb-6">
              <label className="flex items-start gap-3 rounded-xl border border-border bg-surface-2 px-4 py-3">
                <input
                  type="checkbox"
                  checked={!!editorPreferences.vim}
                  onChange={(event: any) => {
                    const next = writeEditorPreferences({ vim: !!event?.target?.checked });
                    setEditorPreferences(next);
                  }}
                  className="mt-1 h-4 w-4 accent-orange-500"
                />
                <span className="min-w-0">
                  <span className="block text-sm font-medium text-body">Enable Vim mode</span>
                  <span className="block text-xs text-body-soft">
                    Applies to project-studio code editors and persists in this browser.
                  </span>
                </span>
              </label>
            </div>
          </DialogContent>
        </Dialog>
        <HelpDialog
          owner={owner}
          project={project}
          isLight={isLight}
          open={helpOpen}
          onClose={() => setHelpOpen(false)}
        />
      </StudioChromeProvider>
      </FileSearchProvider>
    </div>
  );
}
