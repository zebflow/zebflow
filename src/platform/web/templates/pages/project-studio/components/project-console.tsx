import { useEffect, useMemo, useRef, useState, cx } from "zeb";
import Button from "@/components/ui/button";
import Kbd from "@/components/ui/kbd";
import Checkbox from "@/components/ui/checkbox";
import { useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";
import { navigate, patchOverlay } from "@/pages/project-studio/components/studio-shell-behavior";

type ConsoleLine = {
  id: number;
  text: string;
  cls?: string;
  isLink?: string;
};

const LINE_STYLES: Record<string, string> = {
  "cli-echo": "text-gray-400",
  "cli-info": "text-gray-500 italic",
  "cli-error": "text-red-400",
  "cli-success": "text-green-400",
  "cli-warning": "text-amber-300",
  "cli-muted": "text-gray-500",
  "cli-blank": "block h-[0.6em]",
  "cli-ai": "text-sky-300 whitespace-pre-wrap break-words",
  "cli-tool": "text-indigo-400 italic",
  "cli-thinking": "text-gray-600 italic",
  "cli-nav": "",
};

const DSL_VERBS = new Set([
  "get", "describe", "register", "activate", "deactivate",
  "execute", "run", "patch", "git", "node", "clear", "help",
]);

const ASSISTANT_CONFIG_CODES = new Set([
  "ASSISTANT_NOT_CONFIGURED",
  "ASSISTANT_DISABLED",
  "ASSISTANT_NO_LLM",
  "ASSISTANT_CREDENTIAL_MISSING",
  "ASSISTANT_CREDENTIAL_INVALID",
]);

function lineClass(cls?: string) {
  const base = "text-gray-400 whitespace-pre break-all";
  if (!cls) return base;
  const extra = cls.split(/\s+/).map((c) => LINE_STYLES[c] ?? "").join(" ");
  return cx(base, extra);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function stripThinkTags(text: string) {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/gi, "").trim();
}

async function consumeSse(
  stream: ReadableStream<Uint8Array>,
  onEvent: (event: { event: string; data: string }) => void,
) {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let split = buffer.indexOf("\n\n");
    while (split >= 0) {
      const raw = buffer.slice(0, split);
      buffer = buffer.slice(split + 2);
      split = buffer.indexOf("\n\n");
      let event = "message";
      const dataLines: string[] = [];
      for (const line of raw.split("\n")) {
        if (line.startsWith("event:")) event = line.slice(6).trim();
        if (line.startsWith("data:")) dataLines.push(line.slice(5).trimStart());
      }
      onEvent({ event, data: dataLines.join("\n") });
    }
  }
}

class InteractionRunner {
  private cancelled = false;

  constructor(private label: string) {
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
        const url = String(step?.url ?? "");
        if (!url) break;
        navigate(url);
        await sleep(900);
        break;
      }
      case "wait_selector": {
        await this.waitForSelector(String(step?.selector ?? ""), Number(step?.timeout_ms ?? 5000));
        break;
      }
      case "click": {
        const selector = String(step?.selector ?? "");
        if (!selector) break;
        const el = await this.waitForSelector(selector, Number(step?.timeout_ms ?? 5000));
        if (!el) break;
        await this.moveCursorTo(el);
        patchOverlay({ clicking: true });
        await sleep(100);
        (el as HTMLElement).click();
        patchOverlay({ clicking: false });
        await sleep(250);
        break;
      }
      default:
        await sleep(120);
        break;
    }
  }
}

export default function ProjectConsole({ owner, project }) {
  const { consoleOpen, toggleConsole, setActivePanel } = useStudioChrome();
  const [lines, setLines] = useState<ConsoleLine[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [pendingLines, setPendingLines] = useState<string[]>([]);
  const [cmdHistory, setCmdHistory] = useState<string[]>([]);
  const [histIdx, setHistIdx] = useState(-1);
  const [useHighModel, setUseHighModel] = useState(false);
  const [autoNavigate, setAutoNavigate] = useState(true);
  const [chatHistory, setChatHistory] = useState<Array<{ role: string; content: string }>>([]);
  const [acItems, setAcItems] = useState<string[]>([]);
  const [acIndex, setAcIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const lineIdRef = useRef(0);
  const currentPageRef = useRef("");

  const prompt = busy ? "···" : pendingLines.length > 0 ? "···>" : "zf>";
  const chatKey = useMemo(() => `zf-chat-${owner}-${project}`, [owner, project]);
  const dslApi = useMemo(() => `/api/projects/${owner}/${project}/pipelines/dsl`, [owner, project]);
  const chatApi = useMemo(() => `/api/projects/${owner}/${project}/assistant/chat`, [owner, project]);

  function pushLine(text: string, cls?: string, isLink?: string) {
    setLines((prev) => [...prev, { id: lineIdRef.current++, text, cls, isLink }]);
  }

  function dropLine(id: number) {
    setLines((prev) => prev.filter((line) => line.id !== id));
  }

  function clearLines() {
    setLines([]);
  }

  function emitLink(url: string) {
    pushLine(url, "cli-nav", url);
  }

  function printHelp() {
    [
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
    ].forEach((line) => pushLine(line));
  }

  useEffect(() => {
    currentPageRef.current = window.location.pathname;
    const onNav = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.url) currentPageRef.current = detail.url;
    };
    window.addEventListener("rwe:nav", onNav);
    return () => window.removeEventListener("rwe:nav", onNav);
  }, []);

  useEffect(() => {
    try {
      const saved = localStorage.getItem(chatKey);
      const parsed = saved ? JSON.parse(saved) : [];
      const next = Array.isArray(parsed) ? parsed : [];
      setChatHistory(next);
      if (next.length > 0) {
        const recent = next.slice(-20);
        const bootLines: ConsoleLine[] = [];
        let nextId = 0;
        for (const msg of recent) {
          if (msg.role === "user") {
            bootLines.push({ id: nextId++, text: `you> ${msg.content}`, cls: "cli-echo" });
          } else if (msg.role === "assistant") {
            for (const line of String(msg.content || "").split("\n")) {
              bootLines.push({ id: nextId++, text: line, cls: "cli-info" });
            }
          }
        }
        if (bootLines.length) {
          bootLines.push({ id: nextId++, text: "─── above: previous session ───", cls: "cli-muted" });
        }
        bootLines.push({ id: nextId++, text: "Zebflow Console  ·  type commands or ask questions", cls: "cli-muted" });
        bootLines.push({ id: nextId++, text: "  type 'help' for DSL reference  ·  ` to toggle", cls: "cli-muted" });
        lineIdRef.current = nextId;
        setLines(bootLines);
      } else {
        setLines([
          { id: 0, text: "Zebflow Console  ·  type commands or ask questions", cls: "cli-muted" },
          { id: 1, text: "  type 'help' for DSL reference  ·  ` to toggle", cls: "cli-muted" },
        ]);
        lineIdRef.current = 2;
      }
    } catch {
      setChatHistory([]);
      setLines([
        { id: 0, text: "Zebflow Console  ·  type commands or ask questions", cls: "cli-muted" },
        { id: 1, text: "  type 'help' for DSL reference  ·  ` to toggle", cls: "cli-muted" },
      ]);
      lineIdRef.current = 2;
    }
  }, [chatKey]);

  useEffect(() => {
    if (!consoleOpen) return;
    setTimeout(() => inputRef.current?.focus(), 40);
  }, [consoleOpen]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "instant" as any });
  }, [lines, consoleOpen]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "`") return;
      const active = document.activeElement;
      const inInput =
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        (active instanceof HTMLElement && active.isContentEditable);
      const inConsole = active instanceof HTMLElement && !!active.closest?.("[data-console-panel]");
      if (inInput && !inConsole) return;
      e.preventDefault();
      toggleConsole();
    }
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true } as any);
  }, [toggleConsole]);

  function getMatches(value: string) {
    const trimmed = value.trimStart();
    if (!trimmed) return [];
    const lower = trimmed.toLowerCase();
    const fromHistory = [...cmdHistory]
      .reverse()
      .filter((cmd) => cmd.toLowerCase().startsWith(lower) && cmd !== trimmed);
    const fromStatic = [
      "get pipelines", "get nodes", "get connections", "get credentials",
      "get templates", "get docs", "describe pipeline ", "describe connection ",
      "describe node ", "register ", "activate pipeline ", "deactivate pipeline ",
      "execute pipeline ", "run ", "run --dry-run", "patch pipeline ",
      "git status", "git log", "git diff", "git add .", "git commit -m ",
      "clear", "help",
    ].filter((cmd) => cmd.toLowerCase().startsWith(lower) && cmd !== trimmed);
    return [...new Set([...fromHistory, ...fromStatic])].slice(0, 8);
  }

  useEffect(() => {
    setAcIndex(-1);
    setAcItems(getMatches(inputValue));
  }, [inputValue, cmdHistory]);

  async function runDsl(cmd: string) {
    const verb = cmd.split(/\s+/)[0].toLowerCase();
    if (verb === "clear") {
      clearLines();
      return;
    }
    if (verb === "help" || cmd.includes("--help") || /\s-h(\s|$)/.test(cmd)) {
      printHelp();
      return;
    }
    try {
      const resp = await fetch(dslApi, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ dsl: cmd }),
      });
      if (resp.status === 401) {
        navigate("/login");
        return;
      }
      const data = await resp.json().catch(() => null);
      const incoming = Array.isArray(data?.lines) ? data.lines : [];
      if (!incoming.length) {
        pushLine("(no output)", "cli-muted");
      } else {
        for (const line of incoming) {
          const text = typeof line === "string" ? line : String(line?.text ?? "");
          const cls = typeof line === "string" ? undefined : line?.cls;
          pushLine(text, cls);
        }
      }
      if (data?.navigate) {
        emitLink(data.navigate);
        if (autoNavigate) navigate(data.navigate);
      }
    } catch (err: any) {
      pushLine(`Error: ${err?.message || String(err)}`, "cli-error");
    }
  }

  async function runAssistant(message: string) {
    const nextHistory = [...chatHistory, { role: "user", content: message }];
    setChatHistory(nextHistory);
    const thinkingId = lineIdRef.current;
    pushLine("thinking…", "cli-thinking");

    try {
      const response = await fetch(chatApi, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "text/event-stream" },
        body: JSON.stringify({
          message,
          history: nextHistory.slice(-24),
          use_high_model: useHighModel,
          current_page: currentPageRef.current,
          client_time: new Date().toLocaleString(),
        }),
      });

      if (response.status === 401) {
        navigate("/login");
        return;
      }

      if (!response.ok) {
        dropLine(thinkingId);
        const payload = await response.json().catch(() => null);
        const code = String(payload?.error?.code || "");
        const messageText = String(payload?.error?.message || `request failed (${response.status})`);
        if (ASSISTANT_CONFIG_CODES.has(code)) {
          pushLine("AI assistant is not ready for this project.", "cli-warning");
          pushLine(`${messageText}. Add an OpenAI credential and assistant config in Settings.`, "cli-warning");
        } else {
          pushLine(`Error: ${messageText}`, "cli-error");
        }
        setChatHistory((prev) => prev.slice(0, -1));
        return;
      }

      if (!response.body) {
        dropLine(thinkingId);
        pushLine("Error: assistant stream missing response body", "cli-error");
        setChatHistory((prev) => prev.slice(0, -1));
        return;
      }

      let finalContent = "";
      await consumeSse(response.body, ({ event, data }) => {
        try {
          if (event === "tool_call") {
            const p = JSON.parse(data);
            pushLine(`  [${p.tool}]`, "cli-tool");
          } else if (event === "tool_result") {
            const p = JSON.parse(data);
            const preview = String(p.result_preview || "").split("\n")[0].slice(0, 100);
            if (preview) pushLine(`  · ${preview}`, "cli-muted");
          } else if (event === "interaction_sequence") {
            const p = JSON.parse(data);
            if (Array.isArray(p?.steps)) {
              const runner = new InteractionRunner(p.label || "Running…");
              runner.run(p.steps).catch(() => {});
            }
          } else if (event === "navigate") {
            const p = JSON.parse(data);
            if (p?.url) {
              emitLink(p.url);
              if (autoNavigate) navigate(p.url);
            }
          } else if (event === "message") {
            finalContent = stripThinkTags(String(JSON.parse(data)?.content || ""));
            dropLine(thinkingId);
            if (finalContent) {
              for (const line of finalContent.split("\n")) pushLine(line, "cli-ai");
            }
          }
        } catch {
          if (event === "message") {
            finalContent = stripThinkTags(data);
            dropLine(thinkingId);
            if (finalContent) pushLine(finalContent, "cli-ai");
          }
        }
      });

      const completed = [...nextHistory, { role: "assistant", content: finalContent }];
      setChatHistory(completed);
      try {
        localStorage.setItem(chatKey, JSON.stringify(completed.slice(-50)));
      } catch {}
    } catch (err: any) {
      dropLine(thinkingId);
      pushLine(`Error: ${err?.message || String(err)}`, "cli-error");
      setChatHistory((prev) => prev.slice(0, -1));
    }
  }

  async function submitCurrent() {
    const raw = inputValue;
    if (!raw || busy) return;
    setAcItems([]);
    setAcIndex(-1);
    setInputValue("");
    setHistIdx(-1);

    if (raw.trimEnd().endsWith("\\")) {
      const next = [...pendingLines, raw.trimEnd().slice(0, -1)];
      setPendingLines(next);
      pushLine(`${next.length === 1 ? "zf>" : "···"}  ${raw}`, "cli-echo");
      return;
    }

    const cmd = [...pendingLines, raw.trim()].join(" ").trim();
    setPendingLines([]);
    if (!cmd) return;
    setCmdHistory((prev) => [...prev, cmd]);
    pushLine(`zf> ${cmd}`, "cli-echo");
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
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }

  function onInputKeyDown(e: KeyboardEvent) {
    const acVisible = acItems.length > 0;
    if (e.key === "Enter") {
      e.preventDefault();
      void submitCurrent();
      return;
    }
    if (e.key === "Tab") {
      e.preventDefault();
      if (!acVisible) return;
      if (e.shiftKey) {
        setAcIndex((prev) => Math.max(prev - 1, 0));
      } else if (acIndex < 0) {
        setAcIndex(0);
      } else {
        const choice = acItems[acIndex];
        if (choice) {
          setInputValue(choice);
          setAcItems([]);
          setAcIndex(-1);
        }
      }
      return;
    }
    if (acVisible) {
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setAcIndex((prev) => Math.max(prev - 1, 0));
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setAcIndex((prev) => Math.min(prev + 1, acItems.length - 1));
        return;
      }
      if (e.key === "Escape") {
        e.stopPropagation();
        setAcItems([]);
        setAcIndex(-1);
        return;
      }
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (!cmdHistory.length) return;
      const nextIdx = Math.min(histIdx + 1, cmdHistory.length - 1);
      setHistIdx(nextIdx);
      setInputValue(cmdHistory[cmdHistory.length - 1 - nextIdx] || "");
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const nextIdx = Math.max(histIdx - 1, -1);
      setHistIdx(nextIdx);
      setInputValue(nextIdx < 0 ? "" : (cmdHistory[cmdHistory.length - 1 - nextIdx] || ""));
      return;
    }
    if (e.key === "Escape") {
      setPendingLines([]);
      setInputValue("");
    }
  }

  return (
    <div
      className={cx(
        "fixed bottom-0 left-0 right-0 z-[1000] flex flex-col border-t border-white/10 bg-[#080b10] transition",
        consoleOpen ? "max-h-[40vh]" : "max-h-0 overflow-hidden",
      )}
      data-console-panel
      data-owner={owner}
      data-project={project}
      aria-hidden={consoleOpen ? "false" : "true"}
    >
      <div className="flex min-h-[2rem] select-none items-center gap-2 border-b border-white/[0.06] px-4 py-1.5">
        <span className="font-mono text-xs font-bold text-gray-500">Console</span>
        <span className="inline-flex items-center gap-1 text-[0.65rem] font-mono text-gray-700">
          <Kbd>`</Kbd>
          <span>toggle</span>
        </span>
        <div className="ml-auto flex items-center gap-2.5">
          <Checkbox label="High" checked={useHighModel} onChange={(e) => setUseHighModel(!!e.currentTarget.checked)} />
          <Checkbox label="Auto nav" checked={autoNavigate} onChange={(e) => setAutoNavigate(!!e.currentTarget.checked)} />
        </div>
        <Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Close console"
          onClick={() => {
            setActivePanel(null);
            if (consoleOpen) toggleConsole();
          }}
          className="ml-1 size-6 text-[0.9rem] text-gray-700 hover:text-gray-400"
        >
          ✕
        </Button>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto">
        <div className="px-4 py-1.5 font-mono text-[0.78rem] leading-[1.65]">
          {lines.map((line) =>
            line.isLink ? (
              <div key={line.id} className={lineClass(line.cls)}>
                <a
                  href={line.isLink}
                  className="text-sky-400 no-underline hover:text-sky-300 hover:underline"
                  onClick={(e) => {
                    e.preventDefault();
                    navigate(line.isLink!);
                  }}
                >
                  {line.text}
                </a>
              </div>
            ) : (
              <div key={line.id} className={lineClass(line.cls)}>{line.text}</div>
            ),
          )}
          <div ref={bottomRef} />
        </div>
      </div>

      {acItems.length > 0 ? (
        <div className="overflow-hidden border-t border-white/[0.04] bg-[#090d14]">
          {acItems.map((item, i) => {
            const matchLen = inputValue.trimStart().length;
            const historyMatch = cmdHistory.includes(item);
            return (
              <button
                key={`${item}-${i}`}
                type="button"
                className={cx(
                  "flex w-full items-center gap-1 px-4 py-[0.22rem] font-mono text-[0.78rem]",
                  i === acIndex ? "bg-white/[0.06] text-gray-200" : "text-gray-400",
                )}
                onClick={() => {
                  setInputValue(item);
                  setAcItems([]);
                  setAcIndex(-1);
                  setTimeout(() => inputRef.current?.focus(), 0);
                }}
              >
                <span className="text-sky-300">{item.slice(0, matchLen)}</span>
                <span>{item.slice(matchLen)}</span>
                {historyMatch ? (
                  <span className="ml-auto shrink-0 text-[0.68rem] text-[#1e3a52]">hist</span>
                ) : null}
              </button>
            );
          })}
        </div>
      ) : null}

      <form
        className="flex items-center gap-1.5 border-t border-white/[0.06] bg-[#080b10] px-4 pb-2 pt-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          void submitCurrent();
        }}
      >
        <span className="shrink-0 select-none font-mono text-[0.8rem] text-green-500">{prompt}</span>
        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onInput={(e: any) => setInputValue(e.target.value)}
          onKeyDown={(e: any) => onInputKeyDown(e)}
          className="min-w-0 flex-1 border-none bg-transparent font-mono text-[0.82rem] text-green-300 caret-green-400 outline-none placeholder:text-gray-600"
          placeholder="ask or type commands"
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
        />
      </form>
    </div>
  );
}
