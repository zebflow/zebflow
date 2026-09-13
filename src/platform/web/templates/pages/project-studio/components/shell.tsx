/**
 * Project studio chrome only — not a global layout. Import from here:
 * `@/pages/project-studio/components/shell`. Shared UI for all apps stays under `@/components/`.
 *
 * Layout uses Tailwind utilities (RWE `data-rwe-tw`); colours are theme tokens (`.dark` on the root — docs/contracts/kinds/ui-theme).
 */
import { useEffect, useState, Link, cx } from "zeb/react";
import PlatformSidebar from "@/pages/project-studio/components/platform-sidebar";
import Button from "@/components/ui/button";
import { HelpIcon, HomeIcon, MoonIcon, PreferencesIcon, SunIcon, TerminalIcon, UserIcon } from "@/pages/project-studio/components/icons";
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

function TerminalToggleButton() {
  const { toggleConsole, setActivePanel } = useStudioChrome();
  return (
    <button
      type="button"
      className={cx(
        "bg-[#e9904e] text-white hover:bg-[#f6863c]",
        "flex h-9 items-center justify-center gap-1.5 rounded-none px-2.5 font-mono text-[0.68rem] font-semibold tracking-widest",
      )}
      onClick={() => {
        setActivePanel(null);
        toggleConsole();
      }}
      aria-label="Toggle console"
      title="Console"
    >
      <TerminalIcon className="h-4 w-4" />
      <span>CMD</span>
    </button>
  );
}

function HelpDialog({ owner, project, open, onClose }) {
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
    <Dialog open={open} onOpenChange={(value) => { if (!value) onClose(); }}>
      <DialogContent
        size="full"
        className={cx(
          "border shadow-2xl",
          "border-border bg-background",
        )}
      >
        <div className="flex min-h-0 w-full">
          <aside
            className={cx(
              "flex w-60 shrink-0 flex-col border-r",
              "border-border bg-popover/60",
            )}
          >
            <div className="border-b border-inherit px-3.5 py-2.5">
              <p className={cx("text-[0.82rem] font-semibold", "text-muted-foreground")}>Help</p>
            </div>
            <div className="flex-1 overflow-auto px-2 py-2">
              {sections.map((section) => (
                <HelpTreeNode
                  key={section?.id}
                  node={section}
                  level={0}
                  activeId={active?.id}
                 
                  onSelect={setActiveSection}
                />
              ))}
            </div>
          </aside>

          <div className="flex min-w-0 flex-1 flex-col">
            <div
              className={cx(
                "flex items-center justify-between border-b px-5 py-3",
                "border-border bg-background",
              )}
            >
              <div>
                <p className={cx("text-base font-semibold", "text-muted-foreground")}>
                  {active?.title || "Help"}
                </p>
              </div>
              <Button type="button" variant="ghost" size="sm" onClick={onClose}>Close</Button>
            </div>

            <div className="min-h-0 flex-1 overflow-auto px-5 py-4">
              {loading ? (
                <p className={cx("text-sm", "text-muted-foreground")}>Loading help…</p>
              ) : error ? (
                <p className="text-sm text-red-400">{error}</p>
              ) : active ? (
                <div
                  className={cx(
                    "max-w-none prose prose-sm",
                    "prose-invert",
                  )}
                  dangerouslySetInnerHTML={{ __html: renderMarkdown(active?.content || "") }}
                />
              ) : (
                <p className={cx("text-sm", "text-muted-foreground")}>No help content available.</p>
              )}
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
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

function HelpTreeNode({ node, level, activeId, onSelect }) {
  const active = activeId === node?.id;
  const hasChildren = Array.isArray(node?.children) && node.children.length > 0;
  return (
    <div className="mb-1">
      <button
        type="button"
        onClick={() => onSelect(node?.id)}
        className={cx(
              "flex w-full items-center rounded-md px-3 py-1.5 text-left text-[0.78rem] transition-colors",
          level === 0 ? "font-semibold" : level === 1 ? "font-medium" : "",
          active
            ? "bg-primary/15 text-primary"
            : "text-muted-foreground hover:bg-border",
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
             
              onSelect={onSelect}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export default function ProjectStudioShell(props) {
  const [helpOpen, setHelpOpen] = useState(false);
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [editorPreferences, setEditorPreferences] = useState(getDefaultEditorPreferences());
  const nav = props?.nav ?? {};
  const owner = props?.owner ?? "";
  const project = props?.project ?? "";

  useEffect(() => {
    setEditorPreferences(readEditorPreferences());
    ensureStudioClipboard();
    return subscribeEditorPreferences((prefs) => setEditorPreferences(prefs));
  }, []);

  return (
    <div
      data-studio-theme="dark"
      className={cx(
        "flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground",
        "dark",
      )}
    >
      <FileSearchProvider owner={owner} project={project}>
      <StudioChromeProvider>
        <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          <PlatformSidebar nav={nav} />

          <main
            className={cx(
              "ml-16 flex h-screen min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
              "bg-background",
            )}
          >
            <header
              className={cx(
                "relative z-10 flex h-10 shrink-0 items-center border-b px-4 backdrop-blur-md",
                "border-border bg-background",
              )}
            >
              <div className="flex h-full w-full items-center justify-between">
                <nav className="flex min-w-0 items-center gap-2 text-[0.78rem] leading-none">
                  <Link
                    href="/home"
                    className={cx(
                      "transition-colors",
                      "text-muted-foreground hover:text-foreground",
                    )}
                    aria-label="Go to home"
                  >
                    <HomeIcon />
                  </Link>
                  <span
                    className={cx("select-none", "text-muted-foreground")}
                  >
                    /
                  </span>
                  <Link
                    href={props?.projectHref ?? "#"}
                    className={cx(
                      "truncate transition-colors",
                      "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {props?.projectLabel ?? "Project"}
                  </Link>
                  <span
                    className={cx("select-none", "text-muted-foreground")}
                  >
                    /
                  </span>
                  <span
                    className={cx("font-medium", "text-muted-foreground")}
                    data-rwe-breadcrumb
                  >
                    {props?.currentMenu ?? "Workspace"}
                  </span>
                </nav>

                <div className="flex items-center gap-0.5">
                  <SessionPanel owner={owner} project={project} />
                  <GitRepoPanel owner={owner} project={project} />
                  <TerminalToggleButton />
                  <button
                    type="button"
                    className="flex h-9 w-9 items-center justify-center rounded-none bg-foreground/5 text-foreground transition-colors hover:bg-foreground/10"
                    onClick={() => setHelpOpen(true)}
                    aria-label="Open help"
                    title="Help"
                  >
                    <HelpIcon className="w-5 h-5" />
                  </button>
                  <button
                    type="button"
                    className="flex h-9 w-9 items-center justify-center rounded-none bg-foreground/10 text-foreground transition-colors hover:bg-foreground/15"
                    onClick={() => setPreferencesOpen(true)}
                    aria-label="Open preferences"
                    title="Preferences"
                  >
                    <PreferencesIcon className="h-5 w-5" />
                  </button>
                  <a
                    href="/profile"
                    className="flex h-9 w-9 items-center justify-center rounded-none bg-foreground/15 text-foreground transition-colors hover:bg-foreground/20 hover:no-underline"
                    aria-label="Profile"
                    title="Profile"
                  >
                    <UserIcon className="h-[18px] w-[18px]" />
                  </a>
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
              <label className="flex items-start gap-3 rounded-lg border border-border bg-muted px-4 py-3">
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
                  <span className="block text-sm font-medium text-foreground">Enable Vim mode</span>
                  <span className="block text-xs text-muted-foreground">
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
         
          open={helpOpen}
          onClose={() => setHelpOpen(false)}
        />
      </StudioChromeProvider>
      </FileSearchProvider>
    </div>
  );
}
