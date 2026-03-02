// Project assistant chat launcher (cookie-authenticated internal API + SSE).

(function () {
  function initProjectAssistant() {
    const panel = document.querySelector(".project-shell-chat");
    if (!panel) return;

    const owner = panel.dataset.owner;
    const project = panel.dataset.project;
    if (!owner || !project) return;

    const thread = panel.querySelector("[data-assistant-thread]");
    const form = panel.querySelector("[data-assistant-form]");
    const input = panel.querySelector("[data-assistant-input]");
    const sendButton = panel.querySelector("[data-assistant-send]");
    const useHighToggle = panel.querySelector("[data-assistant-use-high]");
    const status = panel.querySelector("[data-assistant-status]");
    if (!thread || !form || !input || !sendButton || !status) return;

    const endpoint = `/api/projects/${owner}/${project}/assistant/chat`;
    const history = [];

    appendBubble(thread, "assistant", "Assistant ready. Ask anything about this project.");

    form.addEventListener("submit", async (event) => {
      event.preventDefault();

      const message = String(input.value || "").trim();
      if (!message) return;

      appendBubble(thread, "user", message);
      history.push({ role: "user", content: message });
      input.value = "";
      setBusy(sendButton, input, true);
      setStatus(status, "Thinking...");

      const assistantBubble = appendBubble(thread, "assistant", "...");
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
          }),
        });

        if (!response.ok) {
          const payload = await tryReadJson(response);
          const detail =
            payload?.error?.message || payload?.message || `request failed (${response.status})`;
          assistantBubble.textContent = `Error: ${detail}`;
          setStatus(status, "Error");
          return;
        }

        if (!response.body) {
          assistantBubble.textContent = "Error: empty response body";
          setStatus(status, "Error");
          return;
        }

        let finalContent = "";
        await consumeSse(response.body, ({ event, data }) => {
          if (event !== "message") return;
          try {
            const payload = JSON.parse(data);
            finalContent = String(payload?.content || "");
            assistantBubble.textContent = finalContent || "(empty response)";
          } catch (_err) {
            finalContent = data;
            assistantBubble.textContent = finalContent || "(empty response)";
          }
        });

        history.push({
          role: "assistant",
          content: finalContent || assistantBubble.textContent || "",
        });
        setStatus(status, "Ready");
      } catch (err) {
        assistantBubble.textContent = `Error: ${err?.message || String(err)}`;
        setStatus(status, "Error");
      } finally {
        setBusy(sendButton, input, false);
      }
    });
  }

  function setBusy(sendButton, input, busy) {
    sendButton.disabled = busy;
    input.disabled = busy;
    if (!busy) input.focus();
  }

  function setStatus(el, text) {
    el.textContent = text;
  }

  function appendBubble(thread, role, text) {
    const row = document.createElement("div");
    row.className = `project-shell-chat-bubble project-shell-chat-bubble-${role}`;
    row.textContent = text;
    thread.appendChild(row);
    thread.scrollTop = thread.scrollHeight;
    return row;
  }

  async function consumeSse(body, onEvent) {
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

  function parseSseFrame(frame) {
    if (!frame) return null;
    const lines = frame.split("\n");
    let event = "message";
    const data = [];
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

  async function tryReadJson(response) {
    try {
      return await response.json();
    } catch (_err) {
      return null;
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initProjectAssistant);
  } else {
    initProjectAssistant();
  }
})();
