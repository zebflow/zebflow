import { registerShortcut, initKeyboardShortcuts } from "@/components/behavior/keyboard-shortcuts";

// ── Console singleton ─────────────────────────────────────────────────────────
// ConsolePanel is rendered by the shell template (so Tailwind classes are compiled)
// and teleported to document.body on first mount so it survives SPA navigations.

let consolePanel: HTMLElement | null = null;
const linkedTriggers = new WeakSet<Element>();

/**
 * Teleport the console panel from inside #__rwe_root to document.body.
 * On repeat calls (after SPA nav), discard the new in-root duplicate and
 * return the already-teleported body panel.
 */
function teleportConsolePanel(): HTMLElement | null {
  if (consolePanel && document.body.contains(consolePanel)) return consolePanel;

  const allPanels = Array.from(document.querySelectorAll<HTMLElement>("[data-console-panel]"));
  const bodyPanel = allPanels.find((el) => el.parentElement === document.body);
  const inRootPanel = allPanels.find((el) => el.parentElement !== document.body);

  if (bodyPanel) {
    inRootPanel?.remove();
    consolePanel = bodyPanel;
    return bodyPanel;
  }

  if (inRootPanel) {
    document.body.appendChild(inRootPanel);
    consolePanel = inRootPanel;
    return inRootPanel;
  }

  return null;
}

function openConsole() {
  const panel = consolePanel;
  if (!panel) return;
  panel.classList.add("is-open");
  panel.setAttribute("aria-hidden", "false");
  document.querySelectorAll(".project-shell-cmd[data-console-trigger]").forEach((t) => {
    (t as HTMLDetailsElement).open = true;
  });
  const input = panel.querySelector<HTMLInputElement>("[data-cli-input]");
  setTimeout(() => input?.focus(), 40);
}

function closeConsole() {
  const panel = consolePanel;
  if (!panel) return;
  panel.classList.remove("is-open");
  panel.setAttribute("aria-hidden", "true");
  document.querySelectorAll(".project-shell-cmd[data-console-trigger]").forEach((t) => {
    (t as HTMLDetailsElement).open = false;
  });
}

function toggleConsole() {
  teleportConsolePanel();
  if (consolePanel?.classList.contains("is-open")) {
    closeConsole();
  } else {
    openConsole();
  }
}

if (typeof window !== "undefined") {
  (window as any).zfToggleConsole = toggleConsole;
}

// ── Console lines store ───────────────────────────────────────────────────────
// Module-level state so async functions (runDsl, runAssistant) can push lines
// without needing a React dispatch reference passed everywhere.
// ConsoleOutput (in project-studio-shell.tsx layout) subscribes via subscribeConsole().

export type ConsoleLine = {
  id: number;
  cls?: string;
  text: string;
  isLink?: string;
};

let lineCounter = 0;
export let consoleLines: ConsoleLine[] = [];
let notifyConsole: (() => void) | null = null;

export function subscribeConsole(fn: () => void) {
  notifyConsole = fn;
}

export function pushLine(line: Omit<ConsoleLine, "id">): number {
  const id = lineCounter++;
  consoleLines = [...consoleLines, { ...line, id }];
  notifyConsole?.();
  return id;
}

export function dropLine(id: number) {
  consoleLines = consoleLines.filter((l) => l.id !== id);
  notifyConsole?.();
}

export function clearConsole() {
  consoleLines = [];
  notifyConsole?.();
}

// ── Automation overlay store ──────────────────────────────────────────────────
// AutoOverlay component lives in project-studio-shell.tsx (layout entry page).
// InteractionRunner drives it via patchOverlay(); overlay subscribes via subscribeOverlay().

export type AutoOverlayState = {
  active: boolean;
  label: string;
  cursorX: number;
  cursorY: number;
  clicking: boolean;
};

export let autoOverlayState: AutoOverlayState = {
  active: false,
  label: "",
  cursorX: 0,
  cursorY: 0,
  clicking: false,
};
let notifyOverlay: (() => void) | null = null;

export function subscribeOverlay(fn: () => void) {
  notifyOverlay = fn;
}

export function patchOverlay(patch: Partial<AutoOverlayState>) {
  autoOverlayState = { ...autoOverlayState, ...patch };
  notifyOverlay?.();
}

// ── Navigate helper ───────────────────────────────────────────────────────────

export function navigate(url: string) {
  if (typeof (window as any).rweNavigate === "function") {
    (window as any).rweNavigate(url);
  } else {
    window.location.href = url;
  }
}

// ── Stale overlay cleanup ─────────────────────────────────────────────────────

function cleanStaleOverlays() {
  patchOverlay({ active: false });
}

if (typeof window !== "undefined" && typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", cleanStaleOverlays, { once: true });
  } else {
    cleanStaleOverlays();
  }
  window.addEventListener("rwe:nav", cleanStaleOverlays);
}

// ── Init entrypoint ───────────────────────────────────────────────────────────

export function initProjectShellBehavior() {
  if (typeof (globalThis as any).Deno !== "undefined") return;
  if (typeof window === "undefined" || typeof document === "undefined") return;

  registerShortcut({
    key: "`",
    description: "Toggle console",
    action: toggleConsole,
  });
  initKeyboardShortcuts();

  const mount = () => {
    document.querySelectorAll(".project-shell-session").forEach((panel) => {
      if (linkedTriggers.has(panel)) return;
      linkedTriggers.add(panel);
      initSessionPanel(panel);
    });

    document.querySelectorAll(".project-shell-cmd[data-console-trigger]").forEach((trigger) => {
      if (linkedTriggers.has(trigger)) return;
      linkedTriggers.add(trigger);
      linkConsoleTrigger(trigger as HTMLDetailsElement);
    });

    const firstTrigger = document.querySelector<HTMLElement>(
      ".project-shell-cmd[data-console-trigger]",
    );
    if (firstTrigger && !consolePanel) {
      const owner = firstTrigger.dataset.owner ?? "";
      const project = firstTrigger.dataset.project ?? "";
      if (owner && project) {
        teleportConsolePanel();
        initConsoleBehavior(owner, project);
      }
    }
  };

  const scheduleMount = () => {
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(mount);
    } else {
      setTimeout(mount, 0);
    }
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", scheduleMount, { once: true });
    return;
  }
  scheduleMount();
}

function linkConsoleTrigger(trigger: HTMLDetailsElement) {
  if (consolePanel?.classList.contains("is-open")) {
    trigger.open = true;
  }
  trigger.addEventListener("toggle", () => {
    if (trigger.open) {
      openConsole();
    } else {
      closeConsole();
    }
  });
}

// ── Console behavior ──────────────────────────────────────────────────────────
// Runs once; after that the panel is persistent.

let consoleBehaviorBooted = false;

function initConsoleBehavior(owner: string, project: string) {
  if (consoleBehaviorBooted) return;
  consoleBehaviorBooted = true;

  const panel = teleportConsolePanel()!;
  const input = panel.querySelector<HTMLInputElement>("[data-cli-input]")!;
  const form = panel.querySelector<HTMLFormElement>("[data-cli-form]")!;
  const promptEl = panel.querySelector<HTMLElement>("[data-cli-prompt]")!;
  const useHighToggle = panel.querySelector<HTMLInputElement>("[data-assistant-use-high]");
  const autoNavToggle = panel.querySelector<HTMLInputElement>("[data-auto-navigate]");
  const closeBtn = panel.querySelector<HTMLButtonElement>("[data-console-close]");

  const dslApi = `/api/projects/${owner}/${project}/pipelines/dsl`;
  const chatApi = `/api/projects/${owner}/${project}/assistant/chat`;
  const chatKey = `zf-chat-${owner}-${project}`;

  const cmdHistory: string[] = [];
  let histIdx = -1;
  let busy = false;
  let pendingLines: string[] = [];

  let chatHistory: Array<{ role: string; content: string }> = [];
  try {
    const saved = localStorage.getItem(chatKey);
    if (saved) chatHistory = JSON.parse(saved);
  } catch (_) { chatHistory = []; }

  let currentPage = window.location.pathname;
  window.addEventListener("rwe:nav", (e: Event) => {
    const detail = (e as CustomEvent).detail;
    if (detail?.url) currentPage = detail.url;
    if (panel.classList.contains("is-open")) {
      setTimeout(() => input.focus(), 80);
    }
  });

  // Replay saved chat history into the console.
  if (chatHistory.length > 0) {
    const recent = chatHistory.slice(-20);
    for (const msg of recent) {
      if (msg.role === "user") {
        emit(`you> ${msg.content}`, "cli-echo");
      } else if (msg.role === "assistant") {
        for (const line of msg.content.split("\n")) {
          emit(line, "cli-info");
        }
      }
    }
    emit("─── above: previous session ───", "cli-muted");
  }

  emit("Zebflow Console  ·  type commands or ask questions", "cli-muted");
  emit("  type 'help' for DSL reference  ·  ` to toggle", "cli-muted");

  closeBtn?.addEventListener("click", closeConsole);

  input.addEventListener("keydown", (e: KeyboardEvent) => {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!cmdHistory.length) return;
      histIdx = Math.min(histIdx + 1, cmdHistory.length - 1);
      input.value = cmdHistory[cmdHistory.length - 1 - histIdx];
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      histIdx = Math.max(histIdx - 1, -1);
      input.value = histIdx < 0 ? "" : cmdHistory[cmdHistory.length - 1 - histIdx];
    } else if (e.key === "Escape") {
      pendingLines = [];
      promptEl.textContent = "zf>";
      input.value = "";
    }
  });

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const raw = input.value;   // don't trim — trailing \ matters
    if (!raw || busy) return;
    input.value = "";
    histIdx = -1;

    if (raw.trimEnd().endsWith("\\")) {
      pendingLines.push(raw.trimEnd().slice(0, -1)); // strip trailing \
      emit(`${pendingLines.length === 1 ? "zf>" : "···"}  ${raw}`, "cli-echo");
      promptEl.textContent = "···>";
      return;
    }

    const fullParts = [...pendingLines, raw.trim()];
    pendingLines = [];
    promptEl.textContent = "zf>";
    const cmd = fullParts.join(" ").trim();
    if (!cmd) return;
    cmdHistory.push(cmd);
    emit(`zf> ${cmd}`, "cli-echo");
    setBusy(true);
    try {
      const verb = cmd.split(/\s+/)[0].toLowerCase();
      if (DSL_VERBS.has(verb)) {
        await runDsl(cmd);
      } else {
        await runAssistant(cmd);
      }
    } finally {
      setBusy(false);
    }
  });

  // ── DSL execution ───────────────────────────────────────────────────────────

  async function runDsl(cmd: string) {
    const verb = cmd.split(/\s+/)[0].toLowerCase();
    if (verb === "clear") { clearConsole(); return; }
    if (verb === "help") { printHelp(); return; }
    if (cmd.includes("--help") || cmd.match(/\s-h(\s|$)/)) { printHelp(); return; }

    try {
      const resp = await fetch(dslApi, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ dsl: cmd }),
      });
      const data = await resp.json();
      const incoming = Array.isArray(data?.lines) ? data.lines : [];
      if (!incoming.length) {
        emit("(no output)", "cli-muted");
      } else {
        for (const line of incoming) {
          const text = typeof line === "string" ? line : (line.text ?? "");
          const cls = typeof line === "string" ? undefined : line.cls;
          emit(text, cls);
        }
      }
      if (data?.navigate) {
        emitLink(data.navigate);
        if (autoNavToggle?.checked) navigate(data.navigate);
      }
    } catch (err) {
      emit(`Error: ${err instanceof Error ? err.message : String(err)}`, "cli-error");
    }
  }

  // ── Assistant chat ──────────────────────────────────────────────────────────

  async function runAssistant(message: string) {
    chatHistory.push({ role: "user", content: message });
    const thinkingId = emit("thinking…", "cli-thinking");

    try {
      const response = await fetch(chatApi, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "text/event-stream" },
        body: JSON.stringify({
          message,
          history: chatHistory.slice(-24),
          use_high_model: !!useHighToggle?.checked,
          current_page: currentPage,
          client_time: new Date().toLocaleString(),
        }),
      });

      if (!response.ok || !response.body) {
        dropLine(thinkingId);
        emit(`Error: request failed (${response.status})`, "cli-error");
        chatHistory.pop();
        return;
      }

      let finalContent = "";

      await consumeSse(response.body, ({ event, data }) => {
        try {
          if (event === "tool_call") {
            const p = JSON.parse(data);
            emit(`  [${p.tool}]`, "cli-tool");
          } else if (event === "tool_result") {
            const p = JSON.parse(data);
            const preview = String(p.result_preview || "").split("\n")[0].slice(0, 100);
            if (preview) emit(`  · ${preview}`, "cli-muted");
          } else if (event === "interaction_sequence") {
            const p = JSON.parse(data);
            if (Array.isArray(p?.steps)) {
              const runner = new InteractionRunner(p.label || "Running…");
              runner.run(p.steps).catch((err) => console.error("interaction runner", err));
            }
          } else if (event === "navigate") {
            const p = JSON.parse(data);
            if (p?.url) {
              emitLink(p.url);
              if (autoNavToggle?.checked) navigate(p.url);
            }
          } else if (event === "message") {
            const p = JSON.parse(data);
            finalContent = stripThinkTags(String(p?.content || ""));
            dropLine(thinkingId);
            if (finalContent) {
              for (const line of finalContent.split("\n")) {
                emit(line, "cli-ai");
              }
            }
          }
        } catch (_) {
          if (event === "message") {
            finalContent = stripThinkTags(data);
            dropLine(thinkingId);
            emit(finalContent, "cli-ai");
          }
        }
      });

      chatHistory.push({ role: "assistant", content: finalContent });
      try { localStorage.setItem(chatKey, JSON.stringify(chatHistory.slice(-50))); } catch (_) {}
    } catch (err) {
      dropLine(thinkingId);
      emit(`Error: ${err instanceof Error ? err.message : String(err)}`, "cli-error");
      chatHistory.pop();
    }
  }

  // ── Helpers ─────────────────────────────────────────────────────────────────

  function emit(text: string, cls?: string): number {
    return pushLine({ text, cls });
  }

  function emitLink(url: string) {
    pushLine({ text: url, cls: "cli-nav", isLink: url });
  }

  function setBusy(state: boolean) {
    busy = state;
    input.disabled = state;
    promptEl.textContent = state ? "···" : "zf>";
    if (!state) input.focus();
  }

  function printHelp() {
    const lines = [
      "DSL Commands:",
      "  get pipelines              List all pipelines",
      "  get nodes                  List all available node kinds",
      "  get connections            List DB connections",
      "  get credentials            List credential keys",
      "  get templates              List template files",
      "  get docs                   List project docs",
      "  describe pipeline <name>   Show pipeline details + nodes + edges",
      "  describe connection <slug> Show connection details",
      "  describe node <kind>       Show node kind details",
      "  register <name> [--path /] | <node-chain>",
      "  patch pipeline <name> node <id> [--flag val...]",
      "  activate pipeline <name>   Make a pipeline live",
      "  deactivate pipeline <name> Take a pipeline offline",
      "  execute pipeline <name>    Execute an active pipeline",
      "  run [--dry-run] | <nodes>  Execute an ephemeral one-shot pipeline",
      "  git status|log|diff|add|commit [args]",
      "  clear                      Clear output",
      "  help                       Show this help",
      "",
      "Everything else is forwarded to the project assistant.",
      "Chain commands with &&.  Continue long commands with \\.",
      "  Shortcuts: ` to toggle console",
    ];
    for (const line of lines) emit(line);
  }
}

// DSL verbs — input starting with these goes to the DSL executor, not the assistant.
const DSL_VERBS = new Set([
  "get", "describe", "register", "activate", "deactivate",
  "execute", "run", "patch", "git", "node", "clear", "help",
]);

// ── Session panel ─────────────────────────────────────────────────────────────

function initSessionPanel(panel) {
  if (!(panel instanceof HTMLElement)) return;

  const owner = panel.dataset.owner;
  const project = panel.dataset.project;
  if (!owner || !project) return;

  const toggle = panel.querySelector<HTMLInputElement>(".project-shell-session-toggle");
  const tokenInput = panel.querySelector<HTMLInputElement>(".project-shell-session-token-input");
  const urlInput = panel.querySelector<HTMLInputElement>(".project-shell-session-url-input");
  const copyBtn = panel.querySelector<HTMLButtonElement>(".project-shell-session-copy-button");
  const operationChecks = panel.querySelectorAll<HTMLInputElement>(
    '.project-shell-session-operations input[type="checkbox"]',
  );
  if (!toggle || !tokenInput || !urlInput || !copyBtn) return;

  const apiBase = `/api/projects/${owner}/${project}/mcp/session`;

  async function loadSession() {
    try {
      const resp = await fetch(apiBase);
      if (!resp.ok) return;
      const data = await resp.json();
      if (data?.ok && data?.session) {
        toggle.checked = true;
      }
    } catch (_err) {}
  }

  async function createSession() {
    const capabilities = Array.from(operationChecks)
      .filter((entry) => entry.checked)
      .map((entry) => entry.value);
    const resp = await fetch(apiBase, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ capabilities }),
    });
    const data = await resp.json();
    if (!data?.ok || !data?.session) {
      throw new Error(data?.error?.message || "failed to create session");
    }
    tokenInput.value = String(data.session.token || "");
    urlInput.value = String(data.session.mcp_url || "");
  }

  async function revokeSession() {
    await fetch(apiBase, { method: "DELETE" });
    tokenInput.value = "";
    urlInput.value = "";
  }

  toggle.addEventListener("change", async () => {
    try {
      if (toggle.checked) {
        await createSession();
      } else {
        await revokeSession();
      }
    } catch (_err) {
      toggle.checked = false;
    }
  });

  copyBtn.addEventListener("click", async () => {
    if (!tokenInput.value) return;
    const text = tokenInput.value;
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        tokenInput.focus();
        tokenInput.select();
        document.execCommand("copy");
      }
      copyBtn.textContent = "Copied!";
      window.setTimeout(() => { copyBtn.textContent = "Copy"; }, 1500);
    } catch (_err) {
      copyBtn.textContent = "Copy failed";
      window.setTimeout(() => { copyBtn.textContent = "Copy"; }, 1500);
    }
  });

  void loadSession();
}

// ── Utilities ─────────────────────────────────────────────────────────────────

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function stripThinkTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/gi, "").trim();
}

/**
 * Runs a server-issued interaction sequence: animated cursor, blocking overlay, Esc to cancel.
 * All visible elements are rendered by AutoOverlay (Preact component in project-studio-shell.tsx).
 */
class InteractionRunner {
  private cancelled = false;

  constructor(private label: string) {
    // AutoOverlay is always mounted in the layout — just activate it.
    patchOverlay({ active: true, label: this.label, clicking: false });
    document.addEventListener("keydown", this.onKey, { capture: true });
  }

  private onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      this.cancelled = true;
    }
  };

  private uninstall() {
    patchOverlay({ active: false, label: "" });
    document.removeEventListener("keydown", this.onKey, { capture: true });
  }

  private setStatus(msg: string) {
    patchOverlay({ label: msg });
  }

  private async moveCursorTo(el: Element) {
    const rect = el.getBoundingClientRect();
    patchOverlay({
      cursorX: Math.round(rect.left + rect.width / 2),
      cursorY: Math.round(rect.top + rect.height / 2),
    });
    await sleep(280);
  }

  private async waitForSelector(selector: string, timeoutMs: number): Promise<Element | null> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      if (this.cancelled) return null;
      const el = document.querySelector(selector);
      if (el) return el;
      await sleep(80);
    }
    return null;
  }

  async run(steps: any[]): Promise<void> {
    try {
      for (const step of steps) {
        if (this.cancelled) break;
        await this.executeStep(step);
      }
    } finally {
      this.uninstall();
    }
  }

  private async executeStep(step: any) {
    const action = String(step?.action ?? "");
    this.setStatus(`${this.label} — ${action}…`);

    switch (action) {
      case "navigate": {
        const url = String(step.url ?? "");
        if (!url) break;
        this.setStatus("Navigating…");
        if (typeof (window as any).rweNavigate === "function") {
          (window as any).rweNavigate(url);
        } else {
          window.location.href = url;
        }
        await sleep(900);
        break;
      }

      case "wait_for_selector": {
        const selector = String(step.selector ?? "");
        const timeout = Number(step.timeout_ms ?? 5000);
        this.setStatus("Waiting for UI…");
        await this.waitForSelector(selector, timeout);
        break;
      }

      case "set_editor": {
        const selector = String(step.selector ?? "");
        const value = String(step.value ?? "");
        const el = await this.waitForSelector(selector, 3000);
        if (!el) break;
        await this.moveCursorTo(el);
        this.setStatus("Typing SQL…");
        const cmView = (el as any)._cmView;
        if (cmView) {
          const docLen: number = cmView.state.doc.length;
          cmView.dispatch({ changes: { from: 0, to: docLen, insert: value } });
        } else {
          const ta = el as HTMLTextAreaElement;
          if (ta.value !== undefined) {
            ta.value = value;
            ta.dispatchEvent(new Event("input", { bubbles: true }));
          }
        }
        break;
      }

      case "fill": {
        const selector = String(step.selector ?? "");
        const value = String(step.value ?? "");
        const el = await this.waitForSelector(selector, 3000);
        if (!el) break;
        await this.moveCursorTo(el);
        const inp = el as HTMLInputElement;
        inp.value = value;
        inp.dispatchEvent(new Event("input", { bubbles: true }));
        break;
      }

      case "click": {
        const selector = String(step.selector ?? "");
        const el = await this.waitForSelector(selector, 3000);
        if (!el) break;
        await this.moveCursorTo(el);
        patchOverlay({ clicking: true });
        await sleep(120);
        patchOverlay({ clicking: false });
        (el as HTMLElement).click();
        break;
      }

      case "sleep": {
        await sleep(Math.min(Number(step.ms ?? 200), 5000));
        break;
      }
    }
  }
}

async function consumeSse(
  body: ReadableStream<Uint8Array>,
  onEvent: (event: { event: string; data: string }) => void,
) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let boundary = buffer.indexOf("\n\n");
    while (boundary >= 0) {
      const frame = buffer.slice(0, boundary);
      buffer = buffer.slice(boundary + 2);
      const parsed = parseSseFrame(frame);
      if (parsed) onEvent(parsed);
      boundary = buffer.indexOf("\n\n");
    }
  }
}

function parseSseFrame(frame: string) {
  if (!frame) return null;
  const lines = frame.split("\n");
  let event = "message";
  const data: string[] = [];
  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    if (!line || line.startsWith(":")) continue;
    if (line.startsWith("event:")) {
      event = line.slice(6).trim() || "message";
    } else if (line.startsWith("data:")) {
      data.push(line.slice(5).trimStart());
    }
  }
  if (!data.length) return null;
  return { event, data: data.join("\n") };
}
