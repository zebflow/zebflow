import { useMemo } from "zeb/react";
import {
  fileRefSrc,
  followPath,
  isFileRef,
  latestInvocation,
} from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/preview-data";

/**
 * The Run form, built from the graph.
 *
 * Every `n.input.*` node declares one field of the trigger envelope: `body`
 * (fields) and `files` (FileRefs). This module turns those nodes into the
 * widget specs the canvas draws, checks the values the operator typed, and
 * builds the request — multipart when a file is present, JSON otherwise.
 * Values live in editor state only; nothing here writes the pipeline.
 */

export const INPUT_KIND_PREFIX = "n.input.";
export const FILE_INPUT_KINDS = ["file", "files", "image", "audio", "video"];

export type InputNodeSpec = {
  nodeId: string;
  /** The word after `input.`. */
  kind: string;
  name: string;
  label: string;
  optional: boolean;
  accept: string[];
  default?: unknown;
  min?: unknown;
  max?: unknown;
  /** The widget's dragged size (`config.ui.widget`), when one is stored. */
  width?: number;
  height?: number;
};

export type WidgetSize = { width: number; height: number };

/** A stored widget size: both numbers positive, else nothing. */
export function widgetSizeOf(config: any): WidgetSize | null {
  const raw = config?.ui?.widget;
  const width = Number(raw?.width);
  const height = Number(raw?.height);
  if (!(width > 0) || !(height > 0)) return null;
  return { width: Math.round(width), height: Math.round(height) };
}

export type InputRunResult = {
  text?: string;
  meta?: string;
  src?: string;
  /** The run the value came from — a new run always redraws, even with the same value. */
  runId?: string;
};

export type InputWidgetSpec = InputNodeSpec & {
  value?: unknown;
  invalid?: boolean;
  result?: InputRunResult | null;
};

/** `text` for `n.input.text`; `""` for any other kind. */
export function inputKindOf(kind: string): string {
  const raw = String(kind || "");
  return raw.startsWith(INPUT_KIND_PREFIX) ? raw.slice(INPUT_KIND_PREFIX.length) : "";
}

export function isFileInputKind(kind: string): boolean {
  return FILE_INPUT_KINDS.includes(kind);
}

function acceptList(raw: unknown): string[] {
  if (Array.isArray(raw)) return raw.map((v) => String(v || "").trim()).filter(Boolean);
  return String(raw || "")
    .split(",")
    .map((v) => v.trim())
    .filter(Boolean);
}

/** Every input node in the graph, in graph order. */
export function collectInputNodes(graph: any): InputNodeSpec[] {
  const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
  const out: InputNodeSpec[] = [];
  for (const node of nodes) {
    const kind = inputKindOf(node?.kind);
    if (!kind) continue;
    const config = node?.config && typeof node.config === "object" ? node.config : {};
    const name = String(config.name || "").trim();
    if (!name) continue;
    out.push({
      nodeId: String(node.id || ""),
      kind,
      name,
      label: String(config.label || "").trim() || name,
      optional: config.optional === true,
      accept: acceptList(config.accept),
      default: config.default,
      min: config.min,
      max: config.max,
      ...(widgetSizeOf(config) || {}),
    });
  }
  return out;
}

/**
 * The canvas reported an input widget dragged to a new size. Written where a
 * Save Draft will find it — the live node's `zfConfig.ui.widget`, which
 * `collectPipeline()` keeps beside the coordinates — and into the loaded
 * graph's node in place, the way `applyPreviewSize` does for a panel (a new
 * `pipeline` prop would reload the scene and drop every unsaved drag).
 * `config.ui` is the editor-only key the engine ignores and `graph_to_dsl`
 * never renders, so there is no DSL flag for this: the file carries it, the
 * `describe` line does not. Nothing goes to the server until Save Draft.
 */
export function applyWidgetSize(graph: any, liveNodes: any[], nodeId: string, size: WidgetSize): boolean {
  const width = Math.round(Number(size?.width));
  const height = Math.round(Number(size?.height));
  if (!(width > 0) || !(height > 0)) return false;
  const write = (holder: any): boolean => {
    if (!holder || typeof holder !== "object") return false;
    const ui = holder.ui && typeof holder.ui === "object" ? holder.ui : {};
    ui.widget = { width, height };
    holder.ui = ui;
    return true;
  };
  const loaded = (Array.isArray(graph?.nodes) ? graph.nodes : []).find(
    (node: any) => String(node?.id || "") === nodeId
  );
  const live = (Array.isArray(liveNodes) ? liveNodes : []).find(
    (node: any) => String(node?.zfPipelineNodeId || "") === nodeId
  );
  if (loaded) {
    if (!loaded.config || typeof loaded.config !== "object") loaded.config = {};
    write(loaded.config);
  }
  if (live) {
    if (!live.zfConfig || typeof live.zfConfig !== "object") live.zfConfig = {};
    write(live.zfConfig);
  }
  return !!(loaded || live);
}

/** The value the Run will send for one node: what was typed, else the default. */
export function inputValueFor(spec: InputNodeSpec, values: Record<string, unknown>): unknown {
  const typed = values[spec.nodeId];
  if (typed !== undefined && typed !== null && typed !== "") return typed;
  if (isFileInputKind(spec.kind)) return typed;
  return spec.default;
}

export function isInputValueEmpty(spec: InputNodeSpec, value: unknown): boolean {
  if (value === undefined || value === null) return true;
  if (spec.kind === "boolean") return false;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === "string") return value.trim() === "";
  return false;
}

/** Node ids whose required value is empty — the Run is refused inline for these. */
export function missingRequiredInputs(specs: InputNodeSpec[], values: Record<string, unknown>): string[] {
  return specs
    .filter((spec) => !spec.optional && isInputValueEmpty(spec, inputValueFor(spec, values)))
    .map((spec) => spec.nodeId);
}

function isFileValue(value: unknown): boolean {
  return typeof File !== "undefined" && value instanceof File;
}

function hasFileValue(specs: InputNodeSpec[], values: Record<string, unknown>): boolean {
  return specs.some((spec) => {
    const value = inputValueFor(spec, values);
    return isFileValue(value) || (Array.isArray(value) && value.some(isFileValue));
  });
}

/** A typed value as the JSON form sends it: numbers as numbers, JSON parsed when it parses. */
function jsonValue(spec: InputNodeSpec, value: unknown): unknown {
  if (spec.kind === "number") {
    const n = Number(value);
    return Number.isFinite(n) ? n : value;
  }
  if (spec.kind === "boolean") return value === true || value === "true" || value === "on" || value === "1";
  if (spec.kind === "json" && typeof value === "string") {
    try {
      return JSON.parse(value);
    } catch {
      return value;
    }
  }
  return value;
}

/** A typed value as a multipart text part. */
function textPart(spec: InputNodeSpec, value: unknown): string {
  if (spec.kind === "boolean") return value === true || value === "true" || value === "on" || value === "1" ? "true" : "false";
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}

export type RunRequest = {
  body: FormData | string;
  /** Which spelling the request uses. */
  encoding: "multipart" | "json";
};

/**
 * The execute request. `route` names the trigger the run is validated
 * against: `manual` (the default), or a webhook's path and method so a form
 * route can be tried from the canvas.
 */
export function buildRunRequest(
  fileRelPath: string,
  specs: InputNodeSpec[],
  values: Record<string, unknown>,
  route?: { trigger: "webhook"; path: string; method: string } | null
): RunRequest {
  if (hasFileValue(specs, values)) {
    const form = new FormData();
    form.append("file_rel_path", fileRelPath);
    if (route) {
      form.append("trigger", "webhook");
      form.append("webhook_path", route.path);
      form.append("webhook_method", route.method);
    }
    for (const spec of specs) {
      const value = inputValueFor(spec, values);
      if (isInputValueEmpty(spec, value)) continue;
      if (isFileInputKind(spec.kind)) {
        const files = Array.isArray(value) ? value : [value];
        for (const file of files) if (isFileValue(file)) form.append(spec.name, file as File);
      } else {
        form.append(spec.name, textPart(spec, value));
      }
    }
    return { body: form, encoding: "multipart" };
  }
  const body: Record<string, unknown> = {};
  for (const spec of specs) {
    const value = inputValueFor(spec, values);
    if (isInputValueEmpty(spec, value) || isFileInputKind(spec.kind)) continue;
    body[spec.name] = jsonValue(spec, value);
  }
  const request: Record<string, unknown> = {
    file_rel_path: fileRelPath,
    trigger: route ? "webhook" : "manual",
    input: { body },
  };
  if (route) {
    request.webhook_path = route.path;
    request.webhook_method = route.method;
  }
  return { body: JSON.stringify(request), encoding: "json" };
}

/** The webhook trigger's route, when the graph starts with one. */
export function webhookRouteOf(graph: any): { trigger: "webhook"; path: string; method: string } | null {
  const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
  const trigger = nodes.find((n: any) => String(n?.kind || "") === "n.trigger.webhook");
  if (!trigger) return null;
  const config = trigger.config || {};
  return {
    trigger: "webhook",
    path: String(config.path || "/").trim() || "/",
    method: String(config.method || "POST").trim().toUpperCase() || "POST",
  };
}

function formatBytes(size: unknown): string {
  const n = Number(size);
  if (!Number.isFinite(n) || n < 0) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function firstLine(text: string): string {
  const line = String(text || "").split(/\r?\n/, 1)[0] || "";
  return line.length > 120 ? `${line.slice(0, 120)}…` : line;
}

/** What went in, from one FileRef in the record. */
function fileResult(ref: any, ctx: { owner: string; project: string }): InputRunResult {
  const mime = String(ref?.mime || "").toLowerCase();
  const out: InputRunResult = {
    text: String(ref?.filename || "file"),
    meta: [formatBytes(ref?.size), mime].filter(Boolean).join(" · "),
  };
  // A temporary upload lived for the run and is gone now; only a durable
  // object (a store path named in the JSON form) still has bytes to show.
  if (mime.startsWith("image/") && ref?.lifecycle !== "temporary") {
    out.src = fileRefSrc(ctx.owner, ctx.project, ref);
  }
  return out;
}

/** One node's "what went in" from the latest invocation, or null before any run. */
export function inputRunResult(
  spec: InputNodeSpec,
  invocation: any,
  ctx: { owner: string; project: string }
): InputRunResult | null {
  const trace = Array.isArray(invocation?.trace) ? invocation.trace : [];
  const entry = trace.find((item: any) => String(item?.node_id || "") === spec.nodeId);
  if (!entry || entry.input === null || entry.input === undefined) return null;
  const runId = String(invocation?.run_id || invocation?.at || "");
  const slot = isFileInputKind(spec.kind) ? "files" : "body";
  const value = followPath(entry.input, `${slot}.${spec.name}`);
  const result = ((): InputRunResult | null => {
    if (value === undefined || value === null || value === "") {
      return spec.optional ? { text: "(nothing sent)" } : null;
    }
    if (spec.kind === "files") {
      const list = Array.isArray(value) ? value : [value];
      const total = list.reduce((a: number, f: any) => a + (Number(f?.size) || 0), 0);
      return { text: `${list.length} file${list.length === 1 ? "" : "s"}`, meta: formatBytes(total) };
    }
    if (isFileInputKind(spec.kind)) {
      return isFileRef(value) ? fileResult(value, ctx) : { text: String(value) };
    }
    if (typeof value === "string") return { text: firstLine(value) };
    return { text: firstLine(JSON.stringify(value)) };
  })();
  return result ? { ...result, runId } : null;
}

const objectUrls: WeakMap<File, string> = new WeakMap();

/** A local preview of a file the operator just picked, made once per File. */
function localImageSrc(value: unknown): string {
  if (!isFileValue(value) || typeof URL === "undefined" || !URL.createObjectURL) return "";
  const file = value as File;
  if (!String(file.type || "").startsWith("image/")) return "";
  let url = objectUrls.get(file);
  if (!url) {
    url = URL.createObjectURL(file);
    objectUrls.set(file, url);
  }
  return url;
}

/** Every input node's widget, keyed by pipeline node id — the one prop the canvas needs. */
export function buildInputWidgets(
  specs: InputNodeSpec[],
  invocation: any,
  values: Record<string, unknown>,
  invalid: string[],
  ctx: { owner: string; project: string }
): Record<string, InputWidgetSpec> {
  const out: Record<string, InputWidgetSpec> = {};
  for (const spec of specs) {
    let result = inputRunResult(spec, invocation, ctx);
    // A picked image that the record no longer holds (temporary files are
    // deleted after the run) still has its local bytes to show.
    if (result && isFileInputKind(spec.kind)) {
      const local = localImageSrc(values[spec.nodeId]);
      if (local) result = { ...result, src: local };
    }
    out[spec.nodeId] = {
      ...spec,
      value: values[spec.nodeId],
      invalid: invalid.includes(spec.nodeId),
      result,
    };
  }
  return out;
}

/** The editor's one line of Run-form wiring. */
export function useInputWidgets(
  specs: InputNodeSpec[],
  invocations: any[],
  values: Record<string, unknown>,
  invalid: string[],
  owner: string,
  project: string
): Record<string, InputWidgetSpec> {
  return useMemo(
    () => buildInputWidgets(specs, latestInvocation(invocations), values, invalid, { owner, project }),
    [specs, invocations, values, invalid, owner, project]
  );
}
