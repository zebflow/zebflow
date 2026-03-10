import { useState, useEffect, cx, Link } from "rwe";
import PlatformSidebar from "@/components/platform-sidebar";
import {
  initProjectShellBehavior,
  subscribeConsole,
  subscribeOverlay,
  consoleLines,
  autoOverlayState,
  navigate,
} from "@/components/behavior/project-shell";
import ConsolePanel from "@/components/ui/console-panel";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Checkbox from "@/components/ui/checkbox";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuTrigger from "@/components/ui/dropdown-menu-trigger";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";

// ── Icons ────────────────────────────────────────────────────────────────────

function HomeIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-3.5 h-3.5">
      <path d="M4 10.5L12 4l8 6.5V20H4z" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M20 15.2A8 8 0 118.8 4 6.5 6.5 0 0020 15.2z" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SunIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <circle cx="12" cy="12" r="4" stroke="currentColor" strokeWidth="1.8" />
      <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

function TerminalIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <rect x="3" y="5" width="18" height="14" rx="2" stroke="currentColor" strokeWidth="1.8" />
      <path d="M7 9l3 3-3 3M13 15h4" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SessionIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M8 6h8M6 10h12M9 14h6M11 18h2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  );
}

// ── ConsoleOutput ────────────────────────────────────────────────────────────

function ConsoleOutput() {
  const [lines, setLines] = useState(consoleLines);

  useEffect(() => {
    subscribeConsole(() => setLines([...consoleLines]));
  }, []);

  return (
    <div className="cli-output-list" data-cli-mount>
      {lines.map((line) =>
        line.isLink ? (
          <div key={line.id} className={cx("cli-line", line.cls)}>
            <a
              href={line.isLink}
              className="cli-link"
              onClick={(e) => { e.preventDefault(); navigate(line.isLink); }}
            >
              {line.text}
            </a>
          </div>
        ) : (
          <div key={line.id} className={cx("cli-line", line.cls)}>{line.text}</div>
        )
      )}
    </div>
  );
}

// ── AutoOverlay ──────────────────────────────────────────────────────────────

function AutoOverlay() {
  const [s, setS] = useState(autoOverlayState);

  useEffect(() => {
    subscribeOverlay(() => setS({ ...autoOverlayState }));
  }, []);

  if (!s.active) return null;

  return (
    <div className="zf-auto-overlay">
      <div
        className={cx("zf-auto-cursor", s.clicking && "is-clicking")}
        style={{ transform: `translate(${s.cursorX}px, ${s.cursorY}px)` }}
      />
      <div className="zf-auto-label">{s.label}</div>
      <div className="zf-auto-loader" />
    </div>
  );
}

// ── Session Panel (inside DropdownMenu) ──────────────────────────────────────

function SessionPanel({ owner, project }) {
  return (
    <DropdownMenu
      className="project-shell-session"
      data-owner={owner}
      data-project={project}
    >
      <DropdownMenuTrigger
        className="inline-flex items-center gap-1.5 h-8 px-2.5 rounded-lg border border-[var(--studio-border)] bg-[var(--studio-panel-2)] text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] hover:bg-[var(--studio-panel-3)] transition-all text-sm"
      >
        <SessionIcon />
        <span className="text-xs">Session</span>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="right" className="w-80 p-4 space-y-4 border-[var(--studio-border)] bg-[var(--studio-panel)] text-[var(--studio-text)]">
        <div>
          <p className="text-sm font-medium">MCP Session</p>
          <p className="text-xs text-[var(--studio-text-soft)] mt-0.5">
            Enable per-project remote control for LLM agents (Cursor, etc.)
          </p>
        </div>

        <div>
          <Checkbox label="Enable MCP session" className="project-shell-session-toggle" />
        </div>

        <div className="space-y-1.5 project-shell-session-operations">
          <p className="text-xs font-medium text-[var(--studio-text-soft)]">Allowed capabilities:</p>
          <Checkbox label="Pipelines Read" value="pipelines.read" defaultChecked />
          <Checkbox label="Pipelines Write" value="pipelines.write" />
          <Checkbox label="Pipelines Execute" value="pipelines.execute" />
          <Checkbox label="Templates Read" value="templates.read" />
          <Checkbox label="Templates Write" value="templates.write" />
          <Checkbox label="Credentials Read" value="credentials.read" />
          <Checkbox label="Tables Read" value="tables.read" />
        </div>

        <div className="space-y-1.5">
          <p className="text-xs font-medium text-[var(--studio-text-soft)]">Token:</p>
          <div className="flex gap-1.5">
            <Input
              readOnly
              placeholder="Enable session to generate token"
              className="project-shell-session-token-input flex-1 text-xs h-7"
            />
            <Button variant="outline" size="sm" className="project-shell-session-copy-button">
              Copy
            </Button>
          </div>
        </div>

        <div className="space-y-1.5">
          <p className="text-xs font-medium text-[var(--studio-text-soft)]">MCP URL:</p>
          <Input
            readOnly
            placeholder="Enable session to get URL"
            className="project-shell-session-url-input text-xs h-7"
          />
          <p className="text-[0.65rem] text-[var(--studio-text-soft)] leading-snug">
            Add in Cursor: URL above + Authorization: Bearer TOKEN
          </p>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// ── Layout Shell ─────────────────────────────────────────────────────────────

export default function ProjectStudioShell(props) {
  const [theme, setTheme] = useState("dark");
  const nav = props?.nav ?? {};
  const owner = props?.owner ?? "";
  const project = props?.project ?? "";

  useEffect(() => {
    initProjectShellBehavior();
  }, []);

  return (
    <div className="project-studio-shell">
      <div className="project-studio-frame" data-theme={theme}>
        <PlatformSidebar nav={nav} />

        <main className="project-shell-main">
          <header className="project-shell-header">
            <div className="flex items-center justify-between px-4 h-10">
              {/* Breadcrumb */}
              <nav className="flex items-center gap-2 text-[0.78rem] leading-none min-w-0">
                <Link
                  href="/home"
                  className="text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors"
                  aria-label="Go to home"
                >
                  <HomeIcon />
                </Link>
                <span className="text-[var(--studio-border)] select-none">/</span>
                <Link
                  href={props?.projectHref ?? "#"}
                  className="text-[var(--studio-text-soft)] hover:text-[var(--studio-text)] transition-colors truncate"
                >
                  {props?.projectLabel ?? "Project"}
                </Link>
                <span className="text-[var(--studio-border)] select-none">/</span>
                <span className="text-[var(--studio-text)] font-medium" data-rwe-breadcrumb>
                  {props?.currentMenu ?? "Workspace"}
                </span>
              </nav>

              {/* Tool buttons */}
              <div className="flex items-center gap-1.5">
                {/* Theme toggle */}
                <Button
                  variant="outline"
                  size="icon"
                  onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
                  title="Toggle theme"
                >
                  <span className="project-shell-theme-dark">
                    <MoonIcon />
                  </span>
                  <span className="project-shell-theme-light">
                    <SunIcon />
                  </span>
                </Button>

                {/* Console trigger */}
                <Button
                  variant="outline"
                  size="icon"
                  title="Console (` to toggle)"
                  data-console-trigger="true"
                  data-owner={owner}
                  data-project={project}
                >
                  <TerminalIcon />
                </Button>

                {/* MCP Session */}
                <SessionPanel owner={owner} project={project} />
              </div>
            </div>
          </header>

          <section className="project-shell-workspace" data-rwe-outlet>
            {props?.children}
          </section>
        </main>
      </div>

      {/* Console — teleported to document.body by behavior on first mount */}
      <ConsolePanel owner={owner} project={project}>
        <ConsoleOutput />
      </ConsolePanel>

      {/* AutoOverlay — activated by InteractionRunner via patchOverlay() */}
      <AutoOverlay />

    </div>
  );
}
