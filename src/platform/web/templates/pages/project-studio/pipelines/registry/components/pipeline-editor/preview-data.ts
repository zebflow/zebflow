import { useMemo } from "zeb/react";
import { declaredPreviewSize } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/preview-size";

/**
 * Node previews: turning the latest invocation into what the canvas draws.
 *
 * A node declares `config.preview = { in?: {as, path?}, out?: {as, path?} }`.
 * This module reads that, finds the node's entry in the newest invocation, and
 * resolves one value into the cell shape the graphui bundle renders. Nothing
 * here is read by the engine — `config.preview` is presentation, like
 * `config.ui`.
 */

export type PreviewKind =
  | "image"
  | "video"
  | "audio"
  | "pdf"
  | "json"
  | "text"
  | "table"
  | "html";

export type PreviewCell = {
  as: PreviewKind;
  status: "ok" | "none" | "error";
  src?: string;
  text?: string;
  rows?: Record<string, unknown>[];
  error?: string;
  /** A small caption on the cell — "temporary — not saved" on a snapshot. */
  note?: string;
  /** Panel size on the canvas, when one is stored; the bundle's defaults otherwise. */
  width?: number;
  height?: number;
};

export type PreviewData = Record<string, { in?: PreviewCell; out?: PreviewCell }>;

/** A payload value big enough to hang the canvas is not a preview. */
const MAX_TEXT_CHARS = 4000;
const MAX_TABLE_ROWS = 8;
/** How deep the no-path search looks for a value of the wanted kind. */
const AUTO_PICK_DEPTH = 3;

const MEDIA_KINDS = ["image", "video", "audio", "pdf", "html"];

/** What a media cell says of a FileRef whose bytes were deleted with the run. */
export const TEMPORARY_FILE_GONE = "temporary file — gone after the run; add fs.save to keep it";
/** The caption on a snapshot: the picture is the record's copy, not a file. */
export const TEMPORARY_SNAPSHOT_NOTE = "temporary — not saved";

/**
 * The record's own small copy of a temporary image, when the engine kept one
 * for this half (`entry.preview_snapshot`, see kinds/invocation-record). The
 * cell draws it from a data URI, captioned, so "run, look, tweak, run again"
 * works without `fs.save`. Absent or skipped: nothing, and the caller says
 * the file is gone.
 */
export function snapshotCell(entry: any, which: "in" | "out", as: PreviewKind): PreviewCell | undefined {
  const snap = entry?.preview_snapshot;
  if (!snap || typeof snap !== "object" || as !== "image") return undefined;
  if (String(snap.slot || "out") !== which) return undefined;
  const mime = String(snap.mime || "");
  const data = typeof snap.data_base64 === "string" ? snap.data_base64 : "";
  if (!mime.startsWith("image/") || !data) return undefined;
  return { as, status: "ok", src: `data:${mime};base64,${data}`, note: TEMPORARY_SNAPSHOT_NOTE };
}

/** Why the record has no snapshot for this half, when it says. */
function snapshotSkipped(entry: any, which: "in" | "out"): string {
  const snap = entry?.preview_snapshot;
  if (!snap || typeof snap !== "object" || String(snap.slot || "out") !== which) return "";
  return typeof snap.snapshot_skipped === "string" ? snap.snapshot_skipped : "";
}

const KIND_MIME_PREFIX: Record<string, string> = {
  image: "image/",
  video: "video/",
  audio: "audio/",
  pdf: "application/pdf",
  html: "text/html",
};

export function isFileRef(value: any): boolean {
  return !!value && typeof value === "object" && value.__zf_type === "file_ref";
}

/**
 * The authenticated URL for one stored object.
 *
 * `files/object?ref=…` is the Studio's own read route: the session cookie
 * (or an MCP bearer) with FilesRead, private and public objects alike, a
 * run's temporary files included while they exist. Never `/fs/…`, a public
 * surface that is off by default and exists for sites, and never `/_files/`,
 * which serves the public folder only. No token rides in the URL.
 */
export function fileRefSrc(owner: string, project: string, ref: any): string {
  const path = String(ref?.ref || "").replace(/^\/+/, "");
  if (!path) return "";
  return `/api/projects/${encodeURIComponent(owner)}/${encodeURIComponent(project)}/files/object?ref=${encodeURIComponent(path)}`;
}

/** Follow a dot path into a payload. Missing at any step → undefined. */
export function followPath(payload: any, path: string): any {
  const trimmed = String(path || "").trim();
  if (!trimmed) return payload;
  let current = payload;
  for (const segment of trimmed.split(".")) {
    if (current === null || current === undefined) return undefined;
    current = current[segment];
  }
  return current;
}

function isUrlString(value: any): boolean {
  return (
    typeof value === "string" &&
    (value.startsWith("/") || /^https?:\/\//i.test(value))
  );
}

function isRowArray(value: any): boolean {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    value.every((row) => !!row && typeof row === "object" && !Array.isArray(row))
  );
}

function fileRefMatchesKind(ref: any, as: string): boolean {
  const mime = String(ref?.mime || "").toLowerCase();
  const kind = String(ref?.kind || "").toLowerCase();
  const prefix = KIND_MIME_PREFIX[as];
  if (!prefix) return false;
  return mime.startsWith(prefix) || kind === as;
}

/** The trigger envelope's own keys: what arrived, not what a node made. */
const ENVELOPE_KEYS = ["body", "files"];

/**
 * No path given: take the first value in the payload that fits the kind.
 * json and text take the whole payload; the rest go looking — a node's own
 * product first (the envelope's `body` / `files` are searched last), and a
 * durable file before a temporary one, which is gone once the run ends.
 */
export function autoPickValue(payload: any, as: string): any {
  if (as === "json" || as === "text") return payload;

  const wanted = (value: any): boolean => {
    if (as === "table") return isRowArray(value);
    if (isFileRef(value)) return fileRefMatchesKind(value, as);
    return false;
  };

  const pick = (skipTemporary: boolean): any => {
    const seen: any[] = [];
    const walk = (value: any, depth: number): any => {
      if (value === null || value === undefined) return undefined;
      if (wanted(value)) {
        if (skipTemporary && isFileRef(value) && value.lifecycle === "temporary") return undefined;
        return value;
      }
      if (depth >= AUTO_PICK_DEPTH || typeof value !== "object") return undefined;
      if (seen.includes(value)) return undefined;
      seen.push(value);
      const children = Array.isArray(value)
        ? value
        : Object.keys(value)
            .sort((a, b) => Number(ENVELOPE_KEYS.includes(a)) - Number(ENVELOPE_KEYS.includes(b)))
            .map((key) => value[key]);
      for (const child of children) {
        const hit = walk(child, depth + 1);
        if (hit !== undefined) return hit;
      }
      return undefined;
    };
    return walk(payload, 0);
  };
  const durable = pick(true);
  return durable !== undefined ? durable : pick(false);
}

function truncate(text: string): string {
  return text.length > MAX_TEXT_CHARS ? `${text.slice(0, MAX_TEXT_CHARS)}…` : text;
}

/** Resolve one already-selected value into the cell the canvas draws. */
export function resolvePreviewValue(
  value: any,
  as: PreviewKind,
  ctx: { owner: string; project: string }
): PreviewCell {
  if (value === undefined || value === null) {
    return { as, status: "none" };
  }
  if (isFileRef(value) && MEDIA_KINDS.includes(as)) {
    // The editor renders from the record, so the run is always over — and
    // a temporary file lived for the run. Say so instead of drawing an
    // <img> that 404s. Nothing is stored to make it renderable: that is a
    // separate decision.
    if (value.lifecycle === "temporary") {
      return { as, status: "error", error: TEMPORARY_FILE_GONE };
    }
    const src = fileRefSrc(ctx.owner, ctx.project, value);
    return src ? { as, status: "ok", src } : { as, status: "none" };
  }
  if (isUrlString(value)) {
    return { as, status: "ok", src: value };
  }
  if (as === "table") {
    return isRowArray(value)
      ? { as, status: "ok", rows: value.slice(0, MAX_TABLE_ROWS) }
      : { as, status: "none" };
  }
  if (MEDIA_KINDS.includes(as)) {
    // An image preview of something that is not a file or a URL has nothing
    // to show — saying so beats drawing a broken image.
    return { as, status: "none" };
  }
  const text =
    as === "json"
      ? JSON.stringify(value, null, 2)
      : typeof value === "string"
      ? value
      : JSON.stringify(value);
  return { as, status: "ok", text: truncate(String(text ?? "")) };
}

/** One declared half of one node's preview against one trace entry. */
export function buildPreviewCell(
  declaration: any,
  entry: any,
  which: "in" | "out",
  ctx: { owner: string; project: string }
): PreviewCell | undefined {
  const as = String(declaration?.as || "").trim().toLowerCase() as PreviewKind;
  if (!as) return undefined;
  const sized = (cell: PreviewCell): PreviewCell => ({
    ...cell,
    ...(declaredPreviewSize(declaration) || {}),
  });
  if (!entry) return sized({ as, status: "none" });
  if (which === "out" && entry.error) {
    return sized({ as, status: "error", error: String(entry.error) });
  }
  const payload = which === "in" ? entry.input : entry.output;
  const path = String(declaration?.path || "").trim();
  const value = path ? followPath(payload, path) : autoPickValue(payload, as);
  if (isFileRef(value) && value.lifecycle === "temporary") {
    const snapshot = snapshotCell(entry, which, as);
    if (snapshot) return sized(snapshot);
    const skipped = snapshotSkipped(entry, which);
    if (skipped) {
      return sized({ as, status: "error", error: `${TEMPORARY_FILE_GONE} (snapshot skipped: ${skipped})` });
    }
  }
  return sized(resolvePreviewValue(value, as, ctx));
}

/**
 * The capture level a run of this pipeline records at: its own setting, else
 * the project's, else the engine default. A declared preview records its
 * payload at `on-error`; only `none` leaves the cell with nothing, ever.
 */
export function effectiveCaptureLevel(pipelineMetadata: any, projectDefault: any): string {
  const own = pipelineMetadata?.settings?.trace_capture?.level;
  const level = String(own || projectDefault?.level || "on-error").trim().toLowerCase();
  return level || "on-error";
}

/** At level `none` an empty cell is not "no run yet" — it will never fill. */
function markCaptureOff(cell: PreviewCell | undefined, captureOff: boolean): PreviewCell | undefined {
  if (!cell || !captureOff || cell.status !== "none") return cell;
  return { as: cell.as, status: "error", error: "capture off" };
}

/** The newest invocation, or null when the pipeline has never run. */
export function latestInvocation(invocations: any[]): any {
  const entries = Array.isArray(invocations) ? invocations : [];
  if (entries.length === 0) return null;
  return entries.reduce((newest, item) =>
    Number(item?.at || 0) > Number(newest?.at || 0) ? item : newest
  );
}

/** Every node that declares a preview, keyed by pipeline node id. */
export function buildPreviewData(
  graph: any,
  invocation: any,
  ctx: { owner: string; project: string; captureOff?: boolean }
): PreviewData {
  const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
  const trace = Array.isArray(invocation?.trace) ? invocation.trace : [];
  const captureOff = !!ctx.captureOff;
  const out: PreviewData = {};
  for (const node of nodes) {
    const preview = node?.config?.preview;
    if (!preview || typeof preview !== "object") continue;
    const nodeId = String(node.id || "");
    if (!nodeId) continue;
    const entry = trace.find((item: any) => String(item?.node_id || "") === nodeId) || null;
    const cells: { in?: PreviewCell; out?: PreviewCell } = {};
    const inCell = markCaptureOff(buildPreviewCell(preview.in, entry, "in", ctx), captureOff);
    if (inCell) cells.in = inCell;
    const outCell = markCaptureOff(buildPreviewCell(preview.out, entry, "out", ctx), captureOff);
    if (outCell) cells.out = outCell;
    if (cells.in || cells.out) out[nodeId] = cells;
  }
  return out;
}

/** The editor's one line of preview wiring. */
export function useNodePreviews(
  graph: any,
  invocations: any[],
  owner: string,
  project: string,
  captureLevel: string
): PreviewData {
  return useMemo(
    () =>
      buildPreviewData(graph, latestInvocation(invocations), {
        owner,
        project,
        captureOff: captureLevel === "none",
      }),
    [graph, invocations, owner, project, captureLevel]
  );
}
