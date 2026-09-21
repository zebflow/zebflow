import { useEffect, useState } from "zeb/react";
import { latestInvocation } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/preview-data";

/**
 * Live node status on the canvas — the n8n-style badge at each node's
 * top-right.
 *
 * Two sources feed it. On page open, the latest invocation record: each
 * trace entry's `status` / `duration_ms` / `error` becomes a settled badge,
 * so a reloaded page shows the last run's outcome. When Run is pressed, the
 * execute route's SSE stream: every node goes `pending`, the engine's
 * `node_start` / `node_ok` / `node_skip` / `node_fail` / `node_retry` /
 * `node_error_routed` signals move them, and the closing `result` event is
 * the JSON answer the route would otherwise have sent. Webhook-triggered
 * runs that happen while the page is open are not watched; the record
 * catches up on the next fetch.
 *
 * A retry is a wait, not a failure: a failure an `:error` edge consumed is
 * `retry` (orange ring with the count) or `error_routed` (orange), and red
 * is for the unrouted `fail` only.
 */

export type NodeRunState = "pending" | "running" | "ok" | "skip" | "fail" | "retry" | "error_routed";

export type NodeRunStatus = {
  state: NodeRunState;
  duration_ms?: number;
  error?: string;
  /** `retry`: which attempt this is, and the budget when the engine knows it. */
  attempt?: number;
  max_attempts?: number;
  /** `error_routed`: the node the `:error` edge reached. */
  to_node?: string;
};

export type NodeStatusMap = Record<string, NodeRunStatus>;

/** One `Signal` as the execute stream carries it. */
export type RunSignal = {
  kind: string;
  message?: string;
  node_id?: string;
  node_kind?: string;
  data?: any;
  at?: string;
};

/**
 * The record's status word as a badge state. `refused` and `failed` are
 * both a red cross; `retry` and `error_routed` (a failure an `:error` edge
 * consumed) are orange; a record written before the word existed reads as
 * `ok` unless it carries an error, which is what it meant.
 */
export function stateOfTraceEntry(entry: any): NodeRunState {
  const word = String(entry?.status || "").trim().toLowerCase();
  if (word === "skip") return "skip";
  if (word === "retry") return "retry";
  if (word === "error_routed") return "error_routed";
  if (word === "refused" || word === "failed") return "fail";
  if (entry?.error) return "fail";
  return "ok";
}

/**
 * Every node the latest record ran, settled: the node's latest entry wins,
 * so a node that retried eight times and then succeeded is green. A `retry`
 * entry's attempt is its ordinal among that node's `retry` entries, which is
 * how the engine counted them.
 */
export function seedFromRecord(invocation: any): NodeStatusMap {
  const trace = Array.isArray(invocation?.trace) ? invocation.trace : [];
  const out: NodeStatusMap = {};
  const retries: Record<string, number> = {};
  for (const entry of trace) {
    const nodeId = String(entry?.node_id || "");
    if (!nodeId) continue;
    const status: NodeRunStatus = { state: stateOfTraceEntry(entry) };
    const ms = Number(entry?.duration_ms);
    if (Number.isFinite(ms)) status.duration_ms = ms;
    if (entry?.error) status.error = String(entry.error);
    if (status.state === "retry") {
      retries[nodeId] = (retries[nodeId] || 0) + 1;
      status.attempt = retries[nodeId];
    }
    out[nodeId] = status;
  }
  return out;
}

/** Every node in the graph, `pending` — what Run shows before the first signal. */
export function pendingFor(graph: any): NodeStatusMap {
  const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
  const out: NodeStatusMap = {};
  for (const node of nodes) {
    const nodeId = String(node?.id || "");
    if (nodeId) out[nodeId] = { state: "pending" };
  }
  return out;
}

const SIGNAL_STATES: Record<string, NodeRunState> = {
  node_start: "running",
  node_ok: "ok",
  node_skip: "skip",
  node_fail: "fail",
  node_retry: "retry",
  node_error_routed: "error_routed",
};

/**
 * One engine lifecycle signal applied; any other signal leaves the map as it
 * is. A node re-entered after a `retry` keeps its count while `running`: the
 * wait itself happens inside the retry node's next run (its `--delay-ms`),
 * so the ring with "2/40" would otherwise show for a few milliseconds and
 * the pulsing dot for the rest.
 */
export function applySignal(map: NodeStatusMap, signal: RunSignal): NodeStatusMap {
  const state = SIGNAL_STATES[String(signal?.kind || "")];
  const nodeId = String(signal?.node_id || "");
  if (!state || !nodeId) return map;
  const status: NodeRunStatus = { state };
  const data = signal?.data || {};
  if (state === "running") {
    const prev = map[nodeId];
    if (prev && (prev.state === "retry" || prev.state === "running") && Number.isFinite(prev.attempt)) {
      status.attempt = prev.attempt;
      if (Number.isFinite(prev.max_attempts)) status.max_attempts = prev.max_attempts;
    }
    return { ...map, [nodeId]: status };
  }
  const ms = Number(data.duration_ms);
  if (state !== "running" && Number.isFinite(ms)) status.duration_ms = ms;
  const error = data.error ?? data.message;
  if ((state === "fail" || state === "retry" || state === "error_routed") && error) status.error = String(error);
  if (state === "retry") {
    const attempt = Number(data.attempt);
    if (Number.isFinite(attempt)) status.attempt = attempt;
    const max = Number(data.max_attempts);
    if (Number.isFinite(max) && max > 0) status.max_attempts = max;
  }
  if (state === "error_routed" && data.to_node) status.to_node = String(data.to_node);
  return { ...map, [nodeId]: status };
}

/**
 * What the canvas status line says while a run streams. A wait is told in
 * the muted style — "v_check: waiting — attempt 3/40 (video still
 * processing)" — and so is a handled error; the red "Run failed:" prefix is
 * for the unrouted `node_fail`, whose run is about to fail. Anything else
 * leaves the line as it is.
 */
export function statusLineForSignal(signal: RunSignal): string | null {
  const kind = String(signal?.kind || "");
  const nodeId = String(signal?.node_id || "");
  const data = signal?.data || {};
  const reason = data.message ?? data.error;
  const why = reason ? ` (${String(reason)})` : "";
  if (kind === "node_retry") {
    const attempt = Number(data.attempt);
    const max = Number(data.max_attempts);
    const count = Number.isFinite(attempt)
      ? Number.isFinite(max) && max > 0
        ? ` — attempt ${attempt}/${max}`
        : ` — attempt ${attempt}`
      : "";
    return `${nodeId}: waiting${count}${why}`;
  }
  if (kind === "node_error_routed") {
    const to = data.to_node ? ` → ${String(data.to_node)}` : "";
    return `${nodeId}: error${to}${why}`;
  }
  if (kind === "node_fail") {
    return `Run failed: node '${nodeId}'${why}`;
  }
  return null;
}

/**
 * The stream has ended. A node still `running` failed with the run (the
 * engine could not announce it — a build error, a lost connection); a node
 * still `pending` never ran and gets no badge, as in the record.
 */
export function settleAfterResult(map: NodeStatusMap, ok: boolean, error?: string): NodeStatusMap {
  const out: NodeStatusMap = {};
  for (const [nodeId, status] of Object.entries(map)) {
    if (status.state === "pending") continue;
    if (status.state === "running") {
      out[nodeId] = ok ? { state: "ok" } : { state: "fail", error: error || "run failed" };
      continue;
    }
    out[nodeId] = status;
  }
  return out;
}

export type SseEvent = { event: string; data: string };

/**
 * The complete events in an SSE buffer and what is left over. An event ends
 * at a blank line; comment lines (keep-alives) are dropped; several `data:`
 * lines join with a newline, as the spec says.
 */
export function parseSseEvents(buffer: string): { events: SseEvent[]; rest: string } {
  const events: SseEvent[] = [];
  const normalized = buffer.replace(/\r\n/g, "\n");
  const blocks = normalized.split("\n\n");
  const rest = blocks.pop() ?? "";
  for (const block of blocks) {
    let event = "message";
    const data: string[] = [];
    for (const line of block.split("\n")) {
      if (!line || line.startsWith(":")) continue;
      const colon = line.indexOf(":");
      const field = colon === -1 ? line : line.slice(0, colon);
      let value = colon === -1 ? "" : line.slice(colon + 1);
      if (value.startsWith(" ")) value = value.slice(1);
      if (field === "event") event = value;
      else if (field === "data") data.push(value);
    }
    if (data.length) events.push({ event, data: data.join("\n") });
  }
  return { events, rest };
}

/**
 * One execute request over the stream. Resolves to the `result` event's
 * payload — the JSON answer the route would have sent — and throws on a
 * failed run or a refused request, the way `requestJson` does, so the
 * caller's error path is unchanged. A refusal before the run starts (a
 * missing pipeline, a trigger mismatch) is a plain JSON answer and is read
 * as one. `fetch` because `EventSource` cannot POST.
 */
export async function streamExecute(
  url: string,
  body: FormData | string,
  onSignal: (signal: RunSignal) => void
): Promise<any> {
  const isForm = typeof FormData !== "undefined" && body instanceof FormData;
  const response = await fetch(url, {
    method: "POST",
    body,
    headers: {
      Accept: "text/event-stream, application/json",
      ...(isForm ? {} : { "Content-Type": "application/json" }),
    },
  });
  const type = response.headers.get("content-type") || "";
  if (!type.startsWith("text/event-stream")) {
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const msg = payload?.error?.message || payload?.message || `${response.status} ${response.statusText}`;
      throw new Error(String(msg));
    }
    return payload;
  }
  const reader = response.body?.getReader();
  if (!reader) throw new Error("the run's stream has no body");
  const decoder = new TextDecoder();
  let buffer = "";
  let result: any;
  let seenResult = false;
  for (;;) {
    const { value, done } = await reader.read();
    buffer += decoder.decode(value || new Uint8Array(), { stream: !done });
    const parsed = parseSseEvents(done ? `${buffer}\n\n` : buffer);
    buffer = done ? "" : parsed.rest;
    for (const item of parsed.events) {
      if (item.event === "signal") {
        try {
          onSignal(JSON.parse(item.data));
        } catch {}
      } else if (item.event === "result") {
        seenResult = true;
        try {
          result = JSON.parse(item.data);
        } catch {
          result = null;
        }
      }
    }
    if (done) break;
  }
  if (!seenResult) throw new Error("the run ended without a result");
  if (!result?.ok) throw new Error(String(result?.error?.message || result?.error || "run failed"));
  return result;
}

/**
 * The editor's one line of status wiring: seeded from the latest record
 * whenever the records change and no run is live; the caller drives it
 * from the stream while one is.
 */
export function useRunStatus(
  invocations: any[],
  busy: boolean
): [NodeStatusMap, (next: NodeStatusMap | ((prev: NodeStatusMap) => NodeStatusMap)) => void] {
  const [status, setStatus] = useState<NodeStatusMap>({});
  useEffect(() => {
    if (busy) return;
    setStatus(seedFromRecord(latestInvocation(invocations)));
  }, [invocations, busy]);
  return [status, setStatus];
}
