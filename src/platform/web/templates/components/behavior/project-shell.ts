const initializedRoots = new WeakSet();

export function initProjectShellBehavior() {
  if (typeof Deno !== "undefined") return;
  if (typeof window === "undefined" || typeof document === "undefined") return;

  const mount = () => {
    document.querySelectorAll(".project-shell-session").forEach((panel) => {
      if (initializedRoots.has(panel)) return;
      initializedRoots.add(panel);
      initSessionPanel(panel);
    });
    document.querySelectorAll(".project-shell-chat").forEach((panel) => {
      if (initializedRoots.has(panel)) return;
      initializedRoots.add(panel);
      initProjectAssistant(panel);
    });
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
    } catch (_err) {
      // Keep UI usable if prefetch fails.
    }
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
      window.setTimeout(() => {
        copyBtn.textContent = "Copy";
      }, 1500);
    } catch (_err) {
      copyBtn.textContent = "Copy failed";
      window.setTimeout(() => {
        copyBtn.textContent = "Copy";
      }, 1500);
    }
  });

  void loadSession();
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Executes a server-issued interaction sequence in the browser:
 * - Animated fake mouse cursor moves to each target element
 * - Blocking overlay prevents user interaction during automation
 * - Esc key cancels the sequence at any step
 */
class InteractionRunner {
  private cancelled = false;
  private cursor: HTMLElement | null = null;
  private overlay: HTMLElement | null = null;
  private statusBar: HTMLElement | null = null;
  private loader: HTMLElement | null = null;

  constructor(private label: string) {
    this.install();
    document.addEventListener("keydown", this.onKey, { capture: true });
  }

  private onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopImmediatePropagation();
      this.cancelled = true;
    }
  };

  private install() {
    this.overlay = document.createElement("div");
    this.overlay.className = "zf-auto-overlay";
    this.overlay.title = "Press Esc to cancel";
    document.body.appendChild(this.overlay);

    this.loader = document.createElement("div");
    this.loader.className = "zf-auto-loader";
    document.body.appendChild(this.loader);

    this.cursor = document.createElement("div");
    this.cursor.className = "zf-auto-cursor";
    this.cursor.innerHTML =
      '<svg width="20" height="24" viewBox="0 0 20 24" fill="none">' +
      '<path d="M2 2L2 20L7.5 15.5L10.5 22L13 21L10 14.5H18L2 2Z"' +
      ' fill="white" stroke="#1a1a2e" stroke-width="1.5"/></svg>';
    document.body.appendChild(this.cursor);

    this.statusBar = document.createElement("div");
    this.statusBar.className = "zf-auto-status";
    this.statusBar.textContent = this.label;
    document.body.appendChild(this.statusBar);
  }

  private uninstall() {
    this.cursor?.remove();
    this.overlay?.remove();
    this.statusBar?.remove();
    this.loader?.remove();
    document.removeEventListener("keydown", this.onKey, { capture: true });
    this.cursor = null;
    this.overlay = null;
    this.statusBar = null;
    this.loader = null;
  }

  private setStatus(msg: string) {
    if (this.statusBar) this.statusBar.textContent = msg;
  }

  private async moveCursorTo(el: Element) {
    const rect = el.getBoundingClientRect();
    const x = Math.round(rect.left + rect.width / 2);
    const y = Math.round(rect.top + rect.height / 2);
    if (this.cursor) {
      this.cursor.style.left = `${x}px`;
      this.cursor.style.top = `${y}px`;
    }
    // Wait for CSS transition to play
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
        this.setStatus(`Navigating…`);
        if (typeof (window as any).rweNavigate === "function") {
          (window as any).rweNavigate(url);
        } else {
          window.location.href = url;
        }
        // Give the SPA router time to swap the page
        await sleep(900);
        break;
      }

      case "wait_for_selector": {
        const selector = String(step.selector ?? "");
        const timeout = Number(step.timeout_ms ?? 5000);
        this.setStatus(`Waiting for UI…`);
        await this.waitForSelector(selector, timeout);
        break;
      }

      case "set_editor": {
        const selector = String(step.selector ?? "");
        const value = String(step.value ?? "");
        const el = await this.waitForSelector(selector, 3000);
        if (!el) break;
        await this.moveCursorTo(el);
        this.setStatus(`Typing SQL…`);
        const cmView = (el as any)._cmView;
        if (cmView) {
          const docLen: number = cmView.state.doc.length;
          cmView.dispatch({ changes: { from: 0, to: docLen, insert: value } });
        } else {
          // Fallback for plain textarea
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
        const input = el as HTMLInputElement;
        input.value = value;
        input.dispatchEvent(new Event("input", { bubbles: true }));
        break;
      }

      case "click": {
        const selector = String(step.selector ?? "");
        const el = await this.waitForSelector(selector, 3000);
        if (!el) break;
        await this.moveCursorTo(el);
        this.cursor?.classList.add("is-clicking");
        await sleep(120);
        this.cursor?.classList.remove("is-clicking");
        (el as HTMLElement).click();
        break;
      }

      case "sleep": {
        const ms = Number(step.ms ?? 200);
        await sleep(Math.min(ms, 5000));
        break;
      }
    }
  }
}

function initProjectAssistant(panel) {
  if (!(panel instanceof HTMLElement)) return;

  const owner = panel.dataset.owner;
  const project = panel.dataset.project;
  if (!owner || !project) return;

  const thread = panel.querySelector<HTMLElement>("[data-assistant-thread]");
  const form = panel.querySelector<HTMLFormElement>("[data-assistant-form]");
  const input = panel.querySelector<HTMLTextAreaElement>("[data-assistant-input]");
  const sendButton = panel.querySelector<HTMLButtonElement>("[data-assistant-send]");
  const useHighToggle = panel.querySelector<HTMLInputElement>("[data-assistant-use-high]");
  const status = panel.querySelector<HTMLElement>("[data-assistant-status]");
  if (!thread || !form || !input || !sendButton || !status) return;

  const endpoint = `/api/projects/${owner}/${project}/assistant/chat`;
  const history: Array<{ role: string; content: string }> = [];
  let currentPage = window.location.pathname;
  window.addEventListener("rwe:nav", (e: Event) => {
    const detail = (e as CustomEvent).detail;
    if (detail?.url) currentPage = detail.url;
  });

  appendBubble(thread, "assistant", "Assistant ready. Ask anything about this project.");

  // Enter sends, Shift+Enter adds newline
  input.addEventListener("keydown", (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      form.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));
    }
  });

  form.addEventListener("submit", async (event) => {
    event.preventDefault();

    const message = String(input.value || "").trim();
    if (!message) return;

    appendBubble(thread, "user", message);
    history.push({ role: "user", content: message });
    input.value = "";
    setBusy(sendButton, input, true);
    setStatus(status, "Thinking...");

    // Thinking placeholder — replaced by tool bubbles + final answer
    const thinkingPlaceholder = appendThinkingPlaceholder(thread);
    // The final assistant bubble is created lazily when the message event arrives
    let assistantBubble: HTMLElement | null = null;
    try {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "text/event-stream",
        },
        body: JSON.stringify({
          message,
          history: history.slice(-24),
          use_high_model: !!useHighToggle?.checked,
          current_page: currentPage,
          client_time: new Date().toLocaleString(),
        }),
      });

      if (!response.ok) {
        const payload = await tryReadJson(response);
        const detail =
          payload?.error?.message || payload?.message || `request failed (${response.status})`;
        thinkingPlaceholder.remove();
        assistantBubble = appendBubble(thread, "assistant", `Error: ${detail}`);
        setStatus(status, "Error");
        return;
      }

      if (!response.body) {
        thinkingPlaceholder.remove();
        assistantBubble = appendBubble(thread, "assistant", "Error: empty response body");
        setStatus(status, "Error");
        return;
      }

      let finalContent = "";
      await consumeSse(response.body, ({ event, data }) => {
        try {
          if (event === "tool_call") {
            const payload = JSON.parse(data);
            appendToolBubble(thread, payload.tool, payload.args, payload.thought);
          } else if (event === "tool_result") {
            const payload = JSON.parse(data);
            updateLastToolBubble(thread, payload.result_preview);
          } else if (event === "interaction_sequence") {
            const payload = JSON.parse(data);
            if (Array.isArray(payload?.steps)) {
              const runner = new InteractionRunner(payload.label || "Running…");
              runner.run(payload.steps).catch((err) => {
                console.error("interaction runner failed", err);
              });
            }
          } else if (event === "navigate") {
            const payload = JSON.parse(data);
            if (payload?.url) {
              // Use the SPA router if available, fall back to location
              if (typeof (window as any).rweNavigate === "function") {
                (window as any).rweNavigate(payload.url);
              } else {
                window.location.href = payload.url;
              }
              appendBubble(thread, "assistant", `Navigating to ${payload.label || payload.url}…`);
            }
          } else if (event === "fill_input") {
            const payload = JSON.parse(data);
            if (payload?.selector) {
              // Wait briefly for nav to settle before filling
              setTimeout(() => {
                const el = document.querySelector<HTMLInputElement | HTMLTextAreaElement>(payload.selector);
                if (el) {
                  el.value = payload.value ?? "";
                  el.dispatchEvent(new Event("input", { bubbles: true }));
                  if (payload.submit) {
                    const form = el.closest("form");
                    form?.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));
                  }
                }
              }, 600);
            }
          } else if (event === "message") {
            const payload = JSON.parse(data);
            finalContent = stripThinkTags(String(payload?.content || ""));
            thinkingPlaceholder.remove();
            assistantBubble = appendBubble(thread, "assistant", finalContent || "(empty response)", payload?.content_html || null);
          } else if (event === "done") {
            // nothing extra needed
          }
        } catch (_err) {
          if (event === "message") {
            finalContent = stripThinkTags(data);
            thinkingPlaceholder.remove();
            assistantBubble = appendBubble(thread, "assistant", finalContent || "(empty response)", null);
          }
        }
      });

      history.push({
        role: "assistant",
        content: finalContent || assistantBubble?.textContent || "",
      });
      setStatus(status, "Ready");
    } catch (err) {
      const messageText = err instanceof Error ? err.message : String(err);
      thinkingPlaceholder.remove();
      assistantBubble = appendBubble(thread, "assistant", `Error: ${messageText}`);
      setStatus(status, "Error");
    } finally {
      setBusy(sendButton, input, false);
    }
  });
}

function setBusy(sendButton: HTMLButtonElement, input: HTMLTextAreaElement, busy: boolean) {
  sendButton.disabled = busy;
  input.disabled = busy;
  if (!busy) input.focus();
}

function setStatus(el: HTMLElement, text: string) {
  el.textContent = text;
}

function appendBubble(thread: HTMLElement, role: string, text: string, contentHtml?: string | null): HTMLElement {
  const row = document.createElement("div");
  row.className = `project-shell-chat-bubble project-shell-chat-bubble-${role}`;
  if (contentHtml && role === "assistant") {
    const inner = document.createElement("div");
    inner.className = "prose prose-chat";
    inner.innerHTML = contentHtml;
    row.appendChild(inner);
  } else {
    row.textContent = text;
  }
  thread.appendChild(row);
  thread.scrollTop = thread.scrollHeight;
  return row;
}

function appendThinkingPlaceholder(thread: HTMLElement) {
  const row = document.createElement("div");
  row.className = "project-shell-chat-bubble project-shell-chat-bubble-assistant";
  const indicator = document.createElement("span");
  indicator.className = "project-shell-thinking-indicator";
  indicator.textContent = "Thinking…";
  row.appendChild(indicator);
  thread.appendChild(row);
  thread.scrollTop = thread.scrollHeight;
  return row;
}

/** Strip <think>...</think> chain-of-thought blocks from model output. */
function stripThinkTags(text: string): string {
  return text.replace(/<think>[\s\S]*?<\/think>\s*/gi, "").trim();
}

function appendToolBubble(
  thread: HTMLElement,
  tool: string,
  args: unknown,
  thought: string,
) {
  const row = document.createElement("div");
  row.className =
    "project-shell-chat-bubble project-shell-chat-bubble-tool";
  row.dataset.toolBubble = "1";

  const details = document.createElement("details");
  const summary = document.createElement("summary");
  summary.textContent = `\u{1F527} ${tool}`;
  details.appendChild(summary);

  if (thought) {
    const thoughtEl = document.createElement("pre");
    thoughtEl.className = "tool-thought";
    thoughtEl.textContent = thought;
    details.appendChild(thoughtEl);
  }

  if (args && typeof args === "object" && Object.keys(args as object).length > 0) {
    const argsEl = document.createElement("pre");
    argsEl.className = "tool-args";
    argsEl.textContent = JSON.stringify(args, null, 2);
    details.appendChild(argsEl);
  }

  row.appendChild(details);
  thread.appendChild(row);
  thread.scrollTop = thread.scrollHeight;
  return row;
}

function updateLastToolBubble(thread: HTMLElement, resultPreview: string) {
  const bubbles = thread.querySelectorAll("[data-tool-bubble]");
  const last = bubbles[bubbles.length - 1];
  if (!last) return;
  const details = last.querySelector("details");
  if (!details) return;
  const existing = details.querySelector(".tool-result");
  if (existing) {
    existing.textContent = resultPreview;
  } else {
    const resultEl = document.createElement("pre");
    resultEl.className = "tool-result";
    resultEl.textContent = resultPreview;
    details.appendChild(resultEl);
  }
  thread.scrollTop = thread.scrollHeight;
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

async function tryReadJson(response: Response): Promise<any | null> {
  try {
    return await response.json();
  } catch (_err) {
    return null;
  }
}
