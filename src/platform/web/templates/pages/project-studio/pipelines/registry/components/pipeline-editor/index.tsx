/**
 * PipelineEditor — full React Way pipeline canvas + dialogs.
 *
 * Replaces the ~2378-line imperative pipeline-editor.ts behavior file.
 *
 * Responsibilities:
 *  - Loads catalog, credentials, templates on mount
 *  - Loads selected pipeline on mount (and on selectedId change)
 *  - Renders PipelineGraph with the loaded pipeline JSON
 *  - Opens NodeDialog / WebRenderDialog when a node's "E" button is clicked
 *  - Opens GitCommitDialog after save
 *  - Exposes addNode, save, activate, deactivate
 */
import { useState, useEffect, useRef, useCallback, useNavigate, cx } from "zeb";
import { notifyStudioRepoChanged } from "@/pages/project-studio/components/studio-chrome-bridge";
import Button from "@/components/ui/button";
import Badge from "@/components/ui/badge";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import type { EditorApi, EditorDataState, PipelineNodeData, PipelineMeta, GitFile, NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";
import {
  buildNodeCatalog,
  buildKindIcons,
  buildKindTitles,
  normalizeGraphForEditor,
  normalizeNodePins,
  nodeColor,
  deriveNodeOutputPins,
  deriveNodeOutputLabels,
  canonicalNodeKind,
  isTriggerNodeKind,
  groupedCatalogEntries,
  triggerKindFromNodeKind,
} from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/catalog";
import { sanitizeSlug, ensureUniqueSlug } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";
import { extractNodeConfig } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";
import NodeDialog from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/dialogs/node-dialog";
import WebRenderDialog from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/dialogs/web-render-dialog";
import GitCommitDialog from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/dialogs/git-commit-dialog";
import { LockIcon, LockOpenIcon } from "@/pages/project-studio/components/icons";

// ── graphui bundle loader (sets globalThis.PipelineGraph) ────────────────────
let _graphuiPromise: Promise<void> | null = null;
async function loadGraphui(src: string): Promise<void> {
  if (!src || typeof document === "undefined") return;
  if (_graphuiPromise) return _graphuiPromise;
  _graphuiPromise = (async () => {
    const url = new URL(src, document.baseURI).href;
    await import(url);
  })();
  return _graphuiPromise;
}

// ── Category button SVGs ──────────────────────────────────────────────────────

const CAT_ICONS: Record<string, any> = {
  trigger: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M5 12h14M12 5l7 7-7 7" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  ),
  data: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <ellipse cx="12" cy="6" rx="7" ry="3" stroke="currentColor" strokeWidth="1.7"/>
      <path d="M5 6v8c0 1.66 3.13 3 7 3s7-1.34 7-3V6" stroke="currentColor" strokeWidth="1.7"/>
    </svg>
  ),
  logic: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <circle cx="7" cy="7" r="2" stroke="currentColor" strokeWidth="1.7"/>
      <circle cx="17" cy="17" r="2" stroke="currentColor" strokeWidth="1.7"/>
      <path d="M9 7h3a4 4 0 014 4v4" stroke="currentColor" strokeWidth="1.7"/>
    </svg>
  ),
  web: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.7"/>
      <path d="M12 3c-2.5 3-4 5.5-4 9s1.5 6 4 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/>
      <path d="M12 3c2.5 3 4 5.5 4 9s-1.5 6-4 9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/>
      <path d="M3 12h18" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/>
    </svg>
  ),
  security: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M12 2l7 4v6c0 4.42-3.13 8.56-7 9.93C8.13 20.56 5 16.42 5 12V6l7-4z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
    </svg>
  ),
  files: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
      <path d="M14 2v6h6" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
    </svg>
  ),
  wasm: (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4">
      <path d="M12 2L3 7v10l9 5 9-5V7l-9-5z" stroke="currentColor" strokeWidth="1.7" strokeLinejoin="round"/>
      <path d="M12 12L3 7M12 12l9-5M12 12v10" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round"/>
    </svg>
  ),
};

function PipelineRunIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true">
      <path d="M8 6.5v11l8.5-5.5-8.5-5.5Z" fill="currentColor" />
    </svg>
  );
}

function PipelineDeleteIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 7h16" />
      <path d="M9 3h6" />
      <path d="M7 7l1 13h8l1-13" />
      <path d="M10 11v5M14 11v5" />
    </svg>
  );
}

function PipelineSettingsIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" className="w-4 h-4" aria-hidden="true" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3.75 13.84 5a1 1 0 0 0 .98.08l2.04-.86 1.43 2.47-1.63 1.5a1 1 0 0 0-.28.95l.42 2.18 2.08.72v2.93l-2.08.72a1 1 0 0 0-.65.75l-.42 2.18a1 1 0 0 0 .28.95l1.63 1.5-1.43 2.47-2.04-.86a1 1 0 0 0-.98.08L12 20.25l-1.84-1.25a1 1 0 0 0-.98-.08l-2.04.86-1.43-2.47 1.63-1.5a1 1 0 0 0 .28-.95l-.42-2.18a1 1 0 0 0-.65-.75L4.5 13.97v-2.93l2.08-.72a1 1 0 0 0 .65-.75l.42-2.18a1 1 0 0 0-.28-.95L5.74 4.69l1.43-2.47 2.04.86a1 1 0 0 0 .98-.08L12 3.75Z" />
      <circle cx="12" cy="12" r="3.25" />
    </svg>
  );
}

function sanitizePipelineMetadata(metadata: any, locked: boolean) {
  const next = { ...(metadata || {}) } as any;
  next.locked = !!locked;
  const retention = next?.settings?.invocation_retention || null;
  const maxInv = Number(retention?.max_invocations ?? 0);
  const maxAge = Number(retention?.max_age_secs ?? 0);
  if (maxInv > 0 || maxAge > 0) {
    next.settings = {
      ...(next.settings || {}),
      invocation_retention: {
        ...(maxInv > 0 ? { max_invocations: maxInv } : {}),
        ...(maxAge > 0 ? { max_age_secs: maxAge } : {}),
      },
    };
  } else if (next.settings?.invocation_retention) {
    const settings = { ...(next.settings || {}) };
    delete settings.invocation_retention;
    if (Object.keys(settings).length > 0) {
      next.settings = settings;
    } else {
      delete next.settings;
    }
  }
  return next;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// ── PipelineEditor ────────────────────────────────────────────────────────────

interface PipelineEditorProps {
  api: EditorApi;
  selectedId: string;
  owner: string;
  project: string;
  scopePath: string;
  graphuiSrc: string;
  snapToGrid?: boolean;
  graphuiPackageLabel?: string;
  projectDefaultMaxInvocations?: number;
  onDeleteClick?: () => void;
  onLockToggle?: (locked: boolean) => void;
}

export default function PipelineEditor({
  api,
  selectedId,
  owner,
  project,
  scopePath,
  graphuiSrc,
  snapToGrid = true,
  graphuiPackageLabel = "Graph UI",
  projectDefaultMaxInvocations = 20,
  onDeleteClick,
  onLockToggle,
}: PipelineEditorProps) {
  type InvocationTraceEntry = {
    node_id: string;
    node_kind: string;
    config?: any;
    duration_ms: number;
    input: any;
    output: any;
    error?: string | null;
  };
  type InvocationEntry = {
    run_id?: string;
    at: number;
    duration_ms: number;
    status: string;
    trigger: string;
    error?: string | null;
    trace: InvocationTraceEntry[];
  };
  type ContinuationRequest = {
    graphNodeId: number;
    zfKind: string;
    zfPipelineNodeId: string;
    title?: string;
    x: number;
    y: number;
    outputSlot: number;
    outputPin: string;
    _raw: any;
  };
  const graphRef = useRef(null);
  const canvasRef = useRef<HTMLDivElement>(null);
  const nav = useNavigate();

  const requestJson = useCallback(async (url: string, options: RequestInit = {}): Promise<any> => {
    const response = await fetch(url, {
      headers: {
        Accept: "application/json",
        ...(options.body ? { "Content-Type": "application/json" } : {}),
      },
      ...options,
    });
    if (response.status === 401) {
      nav("/login");
      return null;
    }
    if (response.status === 204) return null;
    const payload = await response.json().catch(() => null);
    if (!response.ok) {
      const msg = (payload as any)?.error?.message || (payload as any)?.message || `${response.status} ${response.statusText}`;
      throw new Error(msg);
    }
    return payload;
  }, [nav]);

  // ── Async data state ────────────────────────────────────────────────────────
  const [catalog, setCatalog] = useState<Map<string, NodeCatalogEntry>>(new Map());
  const [kindIcons, setKindIcons] = useState<Record<string, string>>({});
  const [kindTitles, setKindTitles] = useState<Record<string, string>>({});
  const [dataState, setDataState] = useState<EditorDataState>({
    allCredentials: [],
    pgCredentials: [],
    jwtCredentials: [],
    browserCredentials: [],
    openaiCredentials: [],
    secureRequestCredentials: [],
    httpAuthCredentials: [],
    webhookAuthCredentials: [],
    aiTools: [],
    pageTemplates: [],
    functionPipelines: [],
    owner,
    project,
  });

  // ── Pipeline state ──────────────────────────────────────────────────────────
  const [currentGraph, setCurrentGraph] = useState<any>(null);
  const [currentMeta, setCurrentMeta] = useState<PipelineMeta | null>(null);
  const [currentLocked, setCurrentLocked] = useState(false);
  const [hits, setHits] = useState<any>(null);
  const [loadError, setLoadError] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [pipelineMetadata, setPipelineMetadata] = useState<any>({});

  // ── Dialog state ────────────────────────────────────────────────────────────
  const [dialogNode, setDialogNode] = useState<PipelineNodeData | null>(null);
  const [webRenderNode, setWebRenderNode] = useState<PipelineNodeData | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [retentionInherit, setRetentionInherit] = useState(true);
  const [retentionMaxInv, setRetentionMaxInv] = useState("");
  const [retentionMaxAgeDays, setRetentionMaxAgeDays] = useState("");
  const [continuationRequest, setContinuationRequest] = useState<ContinuationRequest | null>(null);
  const [nodePickerQuery, setNodePickerQuery] = useState("");
  const [nodePickerCategory, setNodePickerCategory] = useState("all");
  const [nodePickerOpen, setNodePickerOpen] = useState(false);

  // ── Git commit dialog state ─────────────────────────────────────────────────
  const [gitDialogOpen, setGitDialogOpen] = useState(false);
  const [gitFiles, setGitFiles] = useState<GitFile[]>([]);
  const [gitRedirectUrl, setGitRedirectUrl] = useState("");


  // ── Invocation log panel state ───────────────────────────────────────────────
  const [logsOpen, setLogsOpen] = useState(false);
  const [invocations, setInvocations] = useState<InvocationEntry[]>([]);
  const [expandedInv, setExpandedInv] = useState<number | null>(null);
  const [expandedNode, setExpandedNode] = useState<string | null>(null);
  const [runBusy, setRunBusy] = useState(false);
  const [runStatus, setRunStatus] = useState("");
  const pollRef = useRef<any>(null);

  // ── Multi-select & clipboard state ─────────────────────────────────────────
  const [multiSelected, setMultiSelected] = useState<Set<number>>(new Set());
  const [selectionMode, setSelectionMode] = useState<"normal" | "box">("normal");
  const [clipboardData, setClipboardData] = useState<string | null>(null);
  const [selectionToast, setSelectionToast] = useState("");
  const [marquee, setMarquee] = useState<{x1: number, y1: number, x2: number, y2: number} | null>(null);
  const marqueeStartRef = useRef<{x: number, y: number} | null>(null);
  const toastTimerRef = useRef<any>(null);

  // ── graphui bundle ready ─────────────────────────────────────────────────────
  const [graphuiReady, setGraphuiReady] = useState(false);

  // ── Load graphui bundle on mount ─────────────────────────────────────────────
  useEffect(() => {
    loadGraphui(graphuiSrc).then(() => setGraphuiReady(true)).catch(() => {});
  }, []);

  // ── Load catalog + credentials + templates on mount ─────────────────────────
  // Dependency array is intentionally empty — these are one-time initializations
  // for the lifetime of this editor instance. api URLs never change between renders.
  useEffect(() => {
    (async () => {
      if (api.nodes) {
        try {
          const data = await requestJson(api.nodes);
          const catalogMap = buildNodeCatalog(data?.items || []);
          setCatalog(catalogMap);
          setKindIcons(buildKindIcons(catalogMap));
          setKindTitles(buildKindTitles(catalogMap));
          // Derive aiTools list from catalog entries that have ai_tool.registered = true
          const aiTools = (data?.items || [])
            .filter((item: any) => item?.ai_tool?.registered === true)
            .map((item: any) => ({
              kind: item.kind,
              tool_name: item.ai_tool.tool_name,
              tool_description: item.ai_tool.tool_description,
            }));
          setDataState((prev) => ({ ...prev, aiTools }));
        } catch {}
      }
      if (api.credentials) {
        try {
          const data = await requestJson(api.credentials);
          const items = Array.isArray(data?.items) ? data.items : [];
          setDataState((prev) => ({
            ...prev,
            allCredentials: items,
            pgCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "postgres"),
            jwtCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "jwt_signing_key"),
            browserCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase().startsWith("browser_")),
            openaiCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "openai"),
            secureRequestCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "secure_request"),
            httpAuthCredentials: items.filter((i: any) => { const k = String(i?.kind || "").toLowerCase(); return k === "secure_request" || k === "oauth2"; }),
            webhookAuthCredentials: items.filter((i: any) => { const k = String(i?.kind || "").toLowerCase(); return k === "jwt_signing_key" || k === "hmac" || k === "api_key"; }),
          }));
        } catch {}
      }
      if (api.templatesWorkspace) {
        try {
          const data = await requestJson(api.templatesWorkspace);
          const items = Array.isArray(data?.items) ? data.items : [];
          setDataState((prev) => ({
            ...prev,
            pageTemplates: items.filter(
              (i: any) =>
                String(i?.kind || "").toLowerCase() === "file" &&
                String(i?.file_kind || "").toLowerCase() === "page"
            ),
          }));
        } catch {}
      }
      // Load function pipelines for n.function.call datalist + n.ai.agent tool list
      if (owner && project) {
        try {
          const data = await requestJson(
            `/api/projects/${encodeURIComponent(owner)}/${encodeURIComponent(project)}/pipelines?recursive=true`
          );
          const items = Array.isArray(data?.items) ? data.items : [];
          const fnPipelines = items.filter(
            (i: any) => String(i?.meta?.trigger_kind || "").toLowerCase() === "function"
          );
          // Merge function pipeline slugs into aiTools so n.ai.agent multi-checkbox shows them
          const fnPipelineTools = fnPipelines.map((i: any) => ({
            kind: "n.trigger.function",
            tool_name: String(i?.meta?.name || ""),
            tool_description: `Function pipeline: ${i?.meta?.title || i?.meta?.name || ""}`,
          })).filter((t: any) => t.tool_name);
          setDataState((prev) => ({
            ...prev,
            functionPipelines: fnPipelines,
            aiTools: [
              ...(prev.aiTools || []).filter((t: any) => t.kind !== "n.trigger.function"),
              ...fnPipelineTools,
            ],
          }));
        } catch {}
      }
    })();
  }, []);

  // ── Load pipeline when selectedId changes ────────────────────────────────────
  useEffect(() => {
    if (!selectedId || !api.byId) return;
    loadPipeline(selectedId);
  }, [selectedId, api.byId]);

  async function loadPipeline(id: string) {
    setLoaded(false);
    setLoadError("");
    try {
      const payload = await requestJson(`${api.byId}?id=${encodeURIComponent(id)}&include_source=true`);
      const source = payload?.source || "{}";
      let graph: any;
      try { graph = JSON.parse(source); } catch {
        graph = { kind: "zebflow.pipeline", version: "0.1", id, entry_nodes: [], nodes: [], edges: [] };
      }
      graph = normalizeGraphForEditor(graph);
      setCurrentGraph(graph);
      setPipelineMetadata(graph?.metadata || {});
      setCurrentMeta(payload.meta || null);
      setCurrentLocked(!!payload.locked);
      setHits(payload.hits || null);
      setLoaded(true);
    } catch (err: any) {
      setLoadError(err?.message || String(err));
      setLoaded(false);
    }
  }

  // ── onNodeEdit callback from PipelineGraph ────────────────────────────────
  function handleNodeEdit(nodeData: PipelineNodeData) {
    if (currentLocked) return;
    const kind = canonicalNodeKind(nodeData.zfKind);
    if (kind === "n.web.render") {
      setWebRenderNode(nodeData);
    } else {
      setDialogNode(nodeData);
    }
  }

  const handleOutputAdd = useCallback((req: ContinuationRequest) => {
    setContinuationRequest(req);
    setNodePickerQuery("");
    setNodePickerCategory("all");
  }, []);

  useEffect(() => {
    function onRuntimeNodeEdit(event: Event) {
      const detail = (event as CustomEvent<PipelineNodeData>).detail;
      if (!detail) return;
      event.preventDefault();
      handleNodeEdit(detail);
    }
    function onRuntimeOutputAdd(event: Event) {
      const detail = (event as CustomEvent<ContinuationRequest>).detail;
      if (!detail) return;
      event.preventDefault();
      handleOutputAdd(detail);
    }
    window.addEventListener("zebflow:pipeline-node-edit", onRuntimeNodeEdit);
    window.addEventListener("zebflow:pipeline-output-add", onRuntimeOutputAdd);
    return () => {
      window.removeEventListener("zebflow:pipeline-node-edit", onRuntimeNodeEdit);
      window.removeEventListener("zebflow:pipeline-output-add", onRuntimeOutputAdd);
    };
  }, [currentLocked, handleOutputAdd]);

  // ── Apply node edit from NodeDialog ──────────────────────────────────────
  function handleNodeApply(
    nodeData: PipelineNodeData,
    slug: string,
    config: Record<string, unknown>
  ) {
    const app = graphRef.current?.getApp?.();
    if (!app) { setDialogNode(null); setWebRenderNode(null); return; }

    // Resolve live graph node — prefer _raw if still in graph, else find by id.
    const graphNodes: any[] = app.graph?.nodes || [];
    let rawNode = nodeData._raw;
    if (!rawNode || !graphNodes.includes(rawNode)) {
      rawNode = graphNodes.find((n: any) => n.id === nodeData.graphNodeId) || null;
    }
    if (!rawNode) { setDialogNode(null); setWebRenderNode(null); return; }

    rawNode.zfPipelineNodeId = slug;
    rawNode.zfConfig = config;

    // Update canvas label
    const kind = nodeData.zfKind || "";
    const catalogEntry = catalog.get(kind);
    const displayTitle = config.title
      ? String(config.title)
      : (catalogEntry?.title || kind);
    rawNode.title = displayTitle;
    const label = rawNode.el?.querySelector?.(".zgu-node-label");
    const header = rawNode.el?.querySelector?.(".zgu-node-header");
    if (label) label.textContent = displayTitle;
    else if (header) header.textContent = displayTitle;
    const badge = rawNode.el?.querySelector?.(".zf-node-slug");
    if (badge) {
      badge.textContent = slug;
      badge.classList.toggle("long", slug.length > 2);
    }

    // Update output pins
    const nextOutputs = deriveNodeOutputPins(
      kind, config,
      (rawNode.outputs || []).map((pin: any) => pin.name),
      ["out"]
    );
    rawNode.zfOutputLabels = deriveNodeOutputLabels(kind, config, nextOutputs);
    app.updateNodePins(rawNode, {
      inputs: (rawNode.inputs || []).map((pin: any) => pin.name),
      outputs: nextOutputs,
    });

    setDialogNode(null);
    setWebRenderNode(null);
  }

  // ── Save pipeline ─────────────────────────────────────────────────────────
  async function handleSave() {
    if (!currentMeta || !graphRef.current || currentLocked) return;
    setSaveError(null);
    const rawPipeline = graphRef.current.collectPipeline();
    const graph = {
      ...rawPipeline,
      metadata: sanitizePipelineMetadata(
        {
          ...(rawPipeline.metadata || {}),
          ...(pipelineMetadata || {}),
        },
        currentLocked,
      ),
    };
    const source = JSON.stringify(graph, null, 2);
    const payload = {
      file_rel_path: currentMeta.file_rel_path,
      title: currentMeta.title,
      description: (currentMeta as any).description,
      trigger_kind: currentMeta.trigger_kind,
      source,
    };
    let result: any;
    try {
      result = await requestJson(api.definition, {
        method: "POST",
        body: JSON.stringify(payload),
      });
    } catch (err: any) {
      setSaveError(err?.message || "Save failed");
      return;
    }
    const id = result?.meta?.file_rel_path || selectedId;
    const path = result?.meta?.virtual_path || currentMeta.virtual_path || scopePath || "/";
    const redirectUrl = `/projects/${owner}/${project}/pipelines/registry?type=pipeline&path=${encodeURIComponent(path)}&file=${encodeURIComponent(id)}`;

    // Check git status and show commit dialog
    try {
      const gitRes = await fetch(`/api/projects/${owner}/${project}/git/status`, {
        headers: { Accept: "application/json" },
      });
      const files = gitRes.ok ? await gitRes.json().catch(() => []) : [];
      if (Array.isArray(files) && files.length > 0) {
        setGitFiles(files);
        setGitRedirectUrl(redirectUrl);
        setGitDialogOpen(true);
        return;
      }
    } catch {}

    notifyStudioRepoChanged();
    nav(redirectUrl);
  }

  useEffect(() => {
    function handleGlobalSaveShortcut(event: KeyboardEvent) {
      const isSaveKey = (event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "s";
      if (!isSaveKey) return;
      const active = document.activeElement as Element | null;
      if (active?.closest?.(".cm-editor, dialog[open]")) {
        return;
      }
      const activeTag = active?.tagName || "";
      if (activeTag === "INPUT" || activeTag === "TEXTAREA" || activeTag === "SELECT" || active?.hasAttribute?.("contenteditable")) {
        return;
      }
      if (!currentMeta || currentLocked) {
        return;
      }
      event.preventDefault();
      void handleSave();
    }

    window.addEventListener("keydown", handleGlobalSaveShortcut);
    return () => window.removeEventListener("keydown", handleGlobalSaveShortcut);
  }, [currentMeta, currentLocked, selectedId]);

  // ── Activate ──────────────────────────────────────────────────────────────
  async function handleActivate() {
    if (!currentMeta || currentLocked) return;
    setSaveError(null);
    try {
      await requestJson(api.activate, {
        method: "POST",
        body: JSON.stringify({ file_rel_path: currentMeta.file_rel_path }),
      });
    } catch (err: any) {
      setSaveError(err?.message || "Activate failed");
      return;
    }
    if (typeof document !== "undefined") {
      nav(`${document.location.pathname}${document.location.search}`);
    }
  }

  // ── Deactivate ────────────────────────────────────────────────────────────
  async function handleDeactivate() {
    if (!currentMeta || currentLocked) return;
    await requestJson(api.deactivate, {
      method: "POST",
      body: JSON.stringify({ file_rel_path: currentMeta.file_rel_path }),
    });
    if (typeof document !== "undefined") {
      nav(`${document.location.pathname}${document.location.search}`);
    }
  }

  // ── Invocation log fetch + polling ────────────────────────────────────────
  async function fetchInvocations(focusLatest = false) {
    if (!currentMeta?.file_rel_path || !api.invocations) return;
    try {
      const data = await requestJson(
        `${api.invocations}?pipeline=${encodeURIComponent(currentMeta.file_rel_path)}`
      );
      const entries = Array.isArray(data?.entries) ? data.entries as InvocationEntry[] : [];
      setInvocations(entries);
      if (focusLatest) {
        setExpandedInv(entries.length > 0 ? 0 : null);
        setExpandedNode(null);
      }
    } catch {}
  }

  async function refreshPipelineSummary() {
    if (!currentMeta?.file_rel_path || !api.byId) return;
    try {
      const payload = await requestJson(`${api.byId}?id=${encodeURIComponent(currentMeta.file_rel_path)}`);
      setCurrentMeta(payload?.meta || null);
      setCurrentLocked(!!payload?.locked);
      setHits(payload?.hits || null);
    } catch {}
  }

  async function handleRunManual() {
    if (!currentMeta || !api.execute || currentLocked || !currentMeta.active_hash) return;
    setRunBusy(true);
    setRunStatus("Running active pipeline…");
    setLogsOpen(true);
    try {
      const payload = await requestJson(api.execute, {
        method: "POST",
        body: JSON.stringify({
          file_rel_path: currentMeta.file_rel_path,
          trigger: "manual",
          input: {},
        }),
      });
      await Promise.all([refreshPipelineSummary(), fetchInvocations(true)]);
      const runId = typeof payload?.run_id === "string" ? payload.run_id : "";
      setRunStatus(runId ? `Manual run completed. ${runId}` : "Manual run completed.");
    } catch (err: any) {
      await Promise.all([refreshPipelineSummary(), fetchInvocations(true)]);
      setRunStatus(`Run failed: ${err?.message || String(err)}`);
    } finally {
      setRunBusy(false);
    }
  }

  useEffect(() => {
    if (!logsOpen) { clearInterval(pollRef.current); return; }
    fetchInvocations();
    pollRef.current = setInterval(fetchInvocations, 5000);
    return () => clearInterval(pollRef.current);
  }, [logsOpen, currentMeta?.file_rel_path]);

  // ── Add node from category ────────────────────────────────────────────────
  function handleAddNode(kind: string) {
    if (!graphRef.current || currentLocked) return;
    const entry = catalog.get(kind);
    const app = (graphRef.current as any)?.getApp?.();
    if (isTriggerNodeKind(kind) && app?.graph && app?.factory && app?.ui) {
      const existingTriggers = Array.isArray(app.graph.nodes)
        ? app.graph.nodes.filter((node: any) => isTriggerNodeKind(node?.zfKind || node?.kind || ""))
        : [];
      const anchor = existingTriggers[0] || null;
      const outgoingLinks = anchor && Array.isArray(app.graph.links)
        ? app.graph.links
            .filter((link: any) => link?.fromNode === anchor.id)
            .map((link: any) => ({
              fromSlot: Number.isFinite(Number(link.fromSlot)) ? Number(link.fromSlot) : 0,
              toNode: link.toNode,
              toSlot: Number.isFinite(Number(link.toSlot)) ? Number(link.toSlot) : 0,
              options: { animated: !!link.animated },
            }))
        : [];
      if (app.ui.selectedNode && existingTriggers.some((node: any) => node?.id === app.ui.selectedNode?.id)) {
        app.ui.clearSelection?.();
      }
      existingTriggers.forEach((node: any) => app.graph.remove(node));
      app.ui.updateWires?.();

      const x = typeof anchor?.x === "number"
        ? anchor.x
        : (-app.ui.transform.x + app.ui.workspaceEl.clientWidth / 2) / app.ui.transform.k - 90;
      const y = typeof anchor?.y === "number"
        ? anchor.y
        : (-app.ui.transform.y + app.ui.workspaceEl.clientHeight / 2) / app.ui.transform.k - 50;

      const node = app.factory.custom(x, y, {
        title: entry?.title || kind,
        color: nodeColor(kind),
        icon: kindIcons[kind] || "",
        inputs: normalizeNodePins(kind, "input", entry?.input_pins || [], ["in"]),
        outputs: normalizeNodePins(kind, "output", entry?.output_pins || [], ["out"]),
      });
      node.zfKind = kind;
      node.zfConfig = canonicalNodeKind(kind) === "n.trigger.webhook"
        ? { method: "GET" }
        : {};
      node.zfOutputLabels = deriveNodeOutputLabels(
        kind,
        node.zfConfig,
        (node.outputs || []).map((pin: any) => pin.name),
      );
      node.zfPipelineNodeId =
        anchor?.zfPipelineNodeId ||
        String(kind).replace(/[^a-z0-9]+/gi, "_").replace(/^_+|_+$/g, "").toLowerCase();
      app.addNode(node);
      outgoingLinks.forEach((link: any) => {
        const targetExists = app.graph.nodes.some((candidate: any) => candidate?.id === link.toNode);
        if (!targetExists) return;
        const fromSlot = Math.min(Math.max(0, link.fromSlot), Math.max(0, (node.outputs || []).length - 1));
        app.graph.connect(node.id, fromSlot, link.toNode, link.toSlot, link.options || app.ui.options?.defaultManualLinkOptions || {});
      });
      app.ui.updateWires?.();
      setCurrentMeta((prev) =>
        prev
          ? {
              ...prev,
              trigger_kind: triggerKindFromNodeKind(kind) || prev.trigger_kind,
            }
          : prev
      );
      return;
    }
    const nextConfig = defaultConfigForNode(kind);
    const outputPins = deriveNodeOutputPins(kind, nextConfig, entry?.output_pins || [], ["out"]);
    graphRef.current.addNode(kind, {
      title: entry?.title || kind,
      color: nodeColor(kind),
      icon: kindIcons[kind] || "",
      input_pins: normalizeNodePins(kind, "input", entry?.input_pins || [], ["in"]),
      output_pins: outputPins,
    });
  }

  function defaultConfigForNode(kind: string) {
    const canonical = canonicalNodeKind(kind);
    if (canonical === "n.trigger.webhook") {
      return { method: "GET" };
    }
    if (canonical === "n.logic.match") {
      return { cases: [], default: { pin: "default", label: "Default" } };
    }
    return {};
  }

  function baseSlugForKind(kind: string) {
    return String(kind || "node")
      .split(".")
      .filter(Boolean)
      .slice(1)
      .join("-") || "node";
  }

  function handleNodePickerSelect(kind: string) {
    if (continuationRequest) {
      handleContinuationAdd(kind);
    } else {
      handleAddNode(kind);
      setNodePickerOpen(false);
      setNodePickerQuery("");
    }
  }

  function handleContinuationAdd(kind: string) {
    if (!graphRef.current || currentLocked || !continuationRequest) return;
    const app = (graphRef.current as any)?.getApp?.();
    if (!app?.graph || !app?.factory || !app?.ui) return;
    const source = app.graph.nodes.find((node: any) => node.id === continuationRequest.graphNodeId);
    if (!source) {
      setContinuationRequest(null);
      return;
    }

    const entry = catalog.get(kind);
    const inputPins = normalizeNodePins(kind, "input", entry?.input_pins || [], ["in"]);
    const nextConfig = defaultConfigForNode(kind);
    const outputPins = deriveNodeOutputPins(kind, nextConfig, entry?.output_pins || [], ["out"]);
    const x = Number.isFinite(Number(continuationRequest.x))
      ? Number(continuationRequest.x)
      : Number.isFinite(Number(source.x))
        ? Number(source.x) + 330
        : 240;
    const y = Number.isFinite(Number(continuationRequest.y))
      ? Number(continuationRequest.y)
      : Number.isFinite(Number(source.y))
        ? Number(source.y) + Math.max(0, continuationRequest.outputSlot) * 34
        : 160;
    const node = app.factory.custom(x, y, {
      title: entry?.title || kind,
      color: nodeColor(kind),
      icon: kindIcons[kind] || "",
      inputs: inputPins,
      outputs: outputPins,
    });
    node.zfKind = kind;
    node.zfConfig = nextConfig;
    node.zfOutputLabels = deriveNodeOutputLabels(kind, nextConfig, outputPins);
    node.zfPipelineNodeId = ensureUniqueSlug(app.graph.nodes, -1, baseSlugForKind(kind));
    app.addNode(node);
    if (inputPins.length > 0) {
      app.graph.connect(source.id, continuationRequest.outputSlot, node.id, 0, app.ui.options?.defaultManualLinkOptions || {});
      app.ui.updateWires?.();
    }
    setContinuationRequest(null);
    setNodePickerQuery("");
    const nodeData = {
      graphNodeId: node.id,
      zfKind: node.zfKind || "",
      zfPipelineNodeId: node.zfPipelineNodeId || "",
      zfConfig: node.zfConfig || {},
      title: node.title,
      x: node.x,
      y: node.y,
      inputs: node.inputs || [],
      outputs: node.outputs || [],
      _raw: node,
    };
    if (canonicalNodeKind(kind) === "n.web.render") {
      setWebRenderNode(nodeData);
    } else {
      setDialogNode(nodeData);
    }
  }

  // ── Toolbar state indicators ──────────────────────────────────────────────
  const isActive =
    currentMeta?.active_hash && currentMeta.active_hash === currentMeta.hash;
  const hasDraft =
    currentMeta?.active_hash && currentMeta.active_hash !== currentMeta.hash;
  const draftLabel = isActive ? "active" : hasDraft ? "draft changed" : "inactive";
  const draftTone = isActive ? "ok" : hasDraft ? "warning" : "neutral";
  const lockTone = currentLocked ? "danger" : "ok";
  const triggerKind = String(currentMeta?.trigger_kind || "-").toUpperCase();

  const successCount = Number(hits?.success_count || 0);
  const failedCount = Number(hits?.failed_count || 0);
  const latestErr =
    Array.isArray(hits?.latest_errors) && hits.latest_errors.length > 0
      ? `${hits.latest_errors[0].code}: ${hits.latest_errors[0].message}`
      : "-";
  const isManualTrigger = String(currentMeta?.trigger_kind || "").toLowerCase() === "manual";
  const manualRunDisabled = !isManualTrigger || currentLocked || !currentMeta?.active_hash || runBusy;
  const manualRunTitle = !currentMeta?.active_hash
    ? "Activate pipeline before running"
    : hasDraft
      ? "Runs the active version, not the unsaved draft"
      : "Run active manual pipeline";
  const retention = pipelineMetadata?.settings?.invocation_retention || null;
  const retentionSummary = retention?.max_age_secs
    ? `retain for ${Math.max(1, Math.round(Number(retention.max_age_secs) / 86400))} day(s)`
    : retention?.max_invocations
      ? `retain last ${retention.max_invocations} run(s)`
      : `inherit project default (${projectDefaultMaxInvocations})`;
  const catalogGroups = groupedCatalogEntries(catalog);
  const catalogCategoryByKind = new Map<string, string>();
  Object.entries(catalogGroups).forEach(([category, groups]) => {
    groups.forEach((group) => {
      group.entries.forEach((entry) => catalogCategoryByKind.set(entry.kind, category));
    });
  });
  const continuationItems = Array.from(catalog.values())
    .map((entry) => ({ category: catalogCategoryByKind.get(entry.kind) || "other", entry }))
    .filter(({ entry }) => {
      if (nodePickerCategory === "trigger") return isTriggerNodeKind(entry.kind);
      return !isTriggerNodeKind(entry.kind);
    })
    .sort((a, b) => {
      const category = a.category.localeCompare(b.category);
      if (category !== 0) return category;
      return String(a.entry.title || a.entry.kind).localeCompare(String(b.entry.title || b.entry.kind));
    })
    .filter(({ category }) => {
      if (nodePickerCategory === "all" || nodePickerCategory === "trigger") return true;
      return category === nodePickerCategory;
    })
    .filter(({ entry }) => {
      const q = nodePickerQuery.trim().toLowerCase();
      if (!q) return true;
      return [
        entry.kind,
        entry.title,
        entry.description,
      ].some((value) => String(value || "").toLowerCase().includes(q));
    });

  function openSettingsDialog() {
    if (!currentMeta) return;
    const retentionCfg = pipelineMetadata?.settings?.invocation_retention || null;
    setRetentionInherit(!retentionCfg?.max_invocations && !retentionCfg?.max_age_secs);
    setRetentionMaxInv(retentionCfg?.max_invocations ? String(retentionCfg.max_invocations) : "");
    setRetentionMaxAgeDays(
      retentionCfg?.max_age_secs
        ? String(Math.max(1, Math.round(Number(retentionCfg.max_age_secs) / 86400)))
        : ""
    );
    setSettingsOpen(true);
  }

  function closeSettingsDialog() {
    setSettingsOpen(false);
  }

  function applyPipelineSettings(e) {
    e.preventDefault();
    const maxInv = parseInt(retentionMaxInv || "0", 10);
    const maxAgeDays = parseInt(retentionMaxAgeDays || "0", 10);
    const next = { ...(pipelineMetadata || {}) } as any;
    if (retentionInherit) {
      if (next.settings?.invocation_retention) {
        const settings = { ...(next.settings || {}) };
        delete settings.invocation_retention;
        next.settings = Object.keys(settings).length > 0 ? settings : undefined;
      }
    } else {
      next.settings = {
        ...(next.settings || {}),
        invocation_retention: {
          ...(maxInv > 0 ? { max_invocations: maxInv } : {}),
          ...(maxAgeDays > 0 ? { max_age_secs: maxAgeDays * 86400 } : {}),
        },
      };
    }
    setPipelineMetadata(next);
    closeSettingsDialog();
  }


  // ── Multi-select helpers ──────────────────────────────────────────────────
  function showToast(msg: string) {
    setSelectionToast(msg);
    clearTimeout(toastTimerRef.current);
    if (msg) toastTimerRef.current = setTimeout(() => setSelectionToast(""), 2000);
  }

  useEffect(() => {
    const app = graphRef.current?.getApp?.();
    if (!app?.graph?.nodes) return;
    if (multiSelected.size > 1) app.ui?.clearSelection?.();
    for (const node of app.graph.nodes) {
      if (!node?.el) continue;
      if (multiSelected.has(node.id)) {
        node.el.style.outline = "2px solid var(--color-accent, #6d9eff)";
        node.el.style.outlineOffset = "2px";
      } else {
        node.el.style.outline = "";
        node.el.style.outlineOffset = "";
      }
    }
  }, [multiSelected]);

  useEffect(() => {
    const el = canvasRef.current;
    if (!el) return;
    function onClick(e: MouseEvent) {
      if (currentLocked) return;
      const app = graphRef.current?.getApp?.();
      if (!app?.graph?.nodes) return;
      if (selectionMode === "box") return;
      const target = e.target as HTMLElement;
      const clickedNode = app.graph.nodes.find((n: any) => n?.el?.contains?.(target));
      if (clickedNode && (e.ctrlKey || e.metaKey)) {
        setMultiSelected((prev) => {
          const next = new Set(prev);
          if (next.has(clickedNode.id)) next.delete(clickedNode.id);
          else next.add(clickedNode.id);
          return next;
        });
      } else if (!clickedNode && !e.shiftKey) {
        setMultiSelected(new Set());
      } else if (clickedNode && !e.ctrlKey && !e.metaKey) {
        setMultiSelected(new Set());
      }
    }
    el.addEventListener("click", onClick);
    return () => el.removeEventListener("click", onClick);
  }, [currentLocked, selectionMode]);

  function handleSelectAllNodes() {
    if (currentLocked) return;
    const app = graphRef.current?.getApp?.();
    if (!app?.graph?.nodes) return;
    const allIds = new Set<number>(app.graph.nodes.map((n: any) => n.id));
    setMultiSelected(allIds);
    showToast(`${allIds.size} node(s) selected`);
  }

  function handleCopySelected() {
    const app = graphRef.current?.getApp?.();
    if (!app?.graph?.nodes || multiSelected.size === 0) return;
    const selectedNodes = app.graph.nodes.filter((n: any) => multiSelected.has(n.id));
    if (selectedNodes.length === 0) return;
    const selectedIds = new Set(selectedNodes.map((n: any) => n.id));
    const links = (app.graph.links || []).filter(
      (link: any) => selectedIds.has(link.fromNode) && selectedIds.has(link.toNode)
    );
    const data = {
      type: "zebflow-pipeline-nodes",
      nodes: selectedNodes.map((n: any) => ({
        id: n.id,
        x: n.x,
        y: n.y,
        zfKind: n.zfKind || "",
        zfConfig: JSON.parse(JSON.stringify(n.zfConfig || {})),
        zfPipelineNodeId: n.zfPipelineNodeId || "",
        zfOutputLabels: n.zfOutputLabels || undefined,
        title: n.title || "",
        inputs: (n.inputs || []).map((p: any) => p.name),
        outputs: (n.outputs || []).map((p: any) => p.name),
      })),
      links: links.map((link: any) => ({
        fromNode: link.fromNode,
        fromSlot: link.fromSlot ?? 0,
        toNode: link.toNode,
        toSlot: link.toSlot ?? 0,
      })),
    };
    const json = JSON.stringify(data);
    setClipboardData(json);
    navigator.clipboard?.writeText?.(json)?.catch?.(() => {});
    showToast(`Copied ${selectedNodes.length} node(s)`);
  }

  async function handlePasteNodes() {
    const app = graphRef.current?.getApp?.();
    if (!app?.graph || !app?.factory || !app?.ui) return;
    let raw = clipboardData;
    if (!raw) {
      try {
        const text = await navigator.clipboard.readText();
        if (text) {
          let parsed: any;
          try { parsed = JSON.parse(text); } catch {}
          if (parsed?.type === "zebflow-pipeline-nodes") raw = text;
        }
      } catch {}
    }
    if (!raw) return;
    let data: any;
    try { data = JSON.parse(raw); } catch { return; }
    if (data?.type !== "zebflow-pipeline-nodes" || !Array.isArray(data.nodes) || data.nodes.length === 0) return;
    const offset = 60;
    const idMap = new Map<number, any>();
    const newIds = new Set<number>();
    for (const src of data.nodes) {
      const kind = src.zfKind || "";
      const entry = catalog.get(kind);
      const inputPins = normalizeNodePins(kind, "input", src.inputs || [], ["in"]);
      const outputPins = src.outputs || ["out"];
      const node = app.factory.custom(src.x + offset, src.y + offset, {
        title: src.title || entry?.title || kind,
        color: nodeColor(kind),
        icon: kindIcons[kind] || "",
        inputs: inputPins,
        outputs: outputPins,
      });
      node.zfKind = kind;
      node.zfConfig = JSON.parse(JSON.stringify(src.zfConfig || {}));
      node.zfOutputLabels = src.zfOutputLabels || deriveNodeOutputLabels(kind, node.zfConfig, outputPins);
      node.zfPipelineNodeId = ensureUniqueSlug(app.graph.nodes, -1, src.zfPipelineNodeId || baseSlugForKind(kind));
      app.addNode(node);
      idMap.set(src.id, node);
      newIds.add(node.id);
    }
    if (Array.isArray(data.links)) {
      for (const link of data.links) {
        const fromNode = idMap.get(link.fromNode);
        const toNode = idMap.get(link.toNode);
        if (fromNode && toNode) {
          app.graph.connect(fromNode.id, link.fromSlot || 0, toNode.id, link.toSlot || 0, app.ui.options?.defaultManualLinkOptions || {});
        }
      }
      app.ui.updateWires?.();
    }
    setMultiSelected(newIds);
    showToast(`Pasted ${data.nodes.length} node(s)`);
  }

  function handleDeleteSelected() {
    const app = graphRef.current?.getApp?.();
    if (!app?.graph?.nodes || multiSelected.size === 0) return;
    const toRemove = app.graph.nodes.filter((n: any) => multiSelected.has(n.id));
    if (toRemove.length === 0) return;
    if (app.ui.selectedNode && multiSelected.has(app.ui.selectedNode.id)) {
      app.ui.clearSelection?.();
    }
    toRemove.forEach((n: any) => app.graph.remove(n));
    app.ui.updateWires?.();
    setMultiSelected(new Set());
    showToast(`Deleted ${toRemove.length} node(s)`);
  }

  useEffect(() => {
    function handleMultiSelectKeys(e: KeyboardEvent) {
      const active = document.activeElement as Element | null;
      if (active?.closest?.(".cm-editor, dialog[open]")) return;
      const tag = active?.tagName || "";
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || active?.hasAttribute?.("contenteditable")) return;
      if (currentLocked) return;
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === "a" && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        handleSelectAllNodes();
        return;
      }
      if (mod && key === "c" && !e.shiftKey && !e.altKey && multiSelected.size > 0) {
        e.preventDefault();
        handleCopySelected();
        return;
      }
      if (mod && key === "v" && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        handlePasteNodes();
        return;
      }
      if ((key === "delete" || key === "backspace") && multiSelected.size > 0) {
        e.preventDefault();
        handleDeleteSelected();
        return;
      }
      if (key === "escape" && multiSelected.size > 0) {
        e.preventDefault();
        setMultiSelected(new Set());
        return;
      }
    }
    window.addEventListener("keydown", handleMultiSelectKeys);
    return () => window.removeEventListener("keydown", handleMultiSelectKeys);
  }, [currentLocked, multiSelected, clipboardData]);

  function handleMarqueeDown(e: React.PointerEvent) {
    const boxMode = selectionMode === "box";
    if ((!boxMode && !e.shiftKey) || currentLocked) return;
    const app = graphRef.current?.getApp?.();
    const target = e.target as HTMLElement;
    if (target?.closest?.("[data-zgu-nodrag='true']")) return;
    if (!boxMode && app?.graph?.nodes?.some((n: any) => n?.el?.contains?.(target))) return;
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    e.preventDefault();
    e.stopPropagation();
    marqueeStartRef.current = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    setMarquee({ x1: marqueeStartRef.current.x, y1: marqueeStartRef.current.y, x2: marqueeStartRef.current.x, y2: marqueeStartRef.current.y });
  }

  function handleMarqueeMove(e: React.PointerEvent) {
    if (!marqueeStartRef.current) return;
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    e.preventDefault();
    e.stopPropagation();
    setMarquee({
      x1: marqueeStartRef.current.x, y1: marqueeStartRef.current.y,
      x2: e.clientX - rect.left, y2: e.clientY - rect.top,
    });
  }

  function handleMarqueeUp(e?: React.PointerEvent) {
    if (!marqueeStartRef.current || !marquee) { marqueeStartRef.current = null; setMarquee(null); return; }
    e?.preventDefault();
    e?.stopPropagation();
    const left = Math.min(marquee.x1, marquee.x2);
    const top = Math.min(marquee.y1, marquee.y2);
    const right = Math.max(marquee.x1, marquee.x2);
    const bottom = Math.max(marquee.y1, marquee.y2);
    if (right - left > 5 && bottom - top > 5) {
      const app = graphRef.current?.getApp?.();
      const containerRect = canvasRef.current?.getBoundingClientRect();
      if (app?.graph?.nodes && containerRect) {
        const selected = new Set<number>();
        for (const node of app.graph.nodes) {
          if (!node?.el) continue;
          const nodeRect = node.el.getBoundingClientRect();
          const cx = nodeRect.left + nodeRect.width / 2 - containerRect.left;
          const cy = nodeRect.top + nodeRect.height / 2 - containerRect.top;
          if (cx >= left && cx <= right && cy >= top && cy <= bottom) selected.add(node.id);
        }
        if (selected.size > 0) {
          setMultiSelected(selected);
          showToast(`${selected.size} node(s) selected`);
        }
      }
    }
    marqueeStartRef.current = null;
    setMarquee(null);
  }

  // Read PipelineGraph from globalThis after bundle loads
  const PipelineGraph = graphuiReady ? (globalThis as any).PipelineGraph : null;

  return (
    <div className="contents">
      {/* Loading overlay — only when a pipeline is selected and not yet ready */}
      {selectedId && (!loaded || !graphuiReady) && !loadError && (
        <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 bg-bg text-body-muted">
          <svg viewBox="0 0 24 24" fill="none" className="w-10 h-10 opacity-30" aria-hidden="true">
            <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" strokeWidth="1.5"/>
            <path d="M3 9h18" stroke="currentColor" strokeWidth="1.5"/>
            <circle cx="7" cy="6" r="1" fill="currentColor"/>
            <circle cx="10" cy="6" r="1" fill="currentColor"/>
            <circle cx="13" cy="6" r="1" fill="currentColor"/>
            <path d="M8 14h8M8 17h5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
          </svg>
          <p className="text-sm font-medium text-body">Loading pipeline…</p>
        </div>
      )}

      {/* Error state */}
      {loadError && (
        <div className="absolute inset-0 z-10 flex flex-col items-center justify-center gap-3 bg-bg text-body-muted">
          <p className="text-sm font-medium text-red-500">{loadError}</p>
        </div>
      )}

      {/* Toolbar */}
      <div className="pipeline-editor-toolbar">
        <div className="flex items-start justify-between gap-4">
          <div className="pipeline-editor-toolbar-main">
            <p className="pipeline-editor-title">
              {currentMeta?.title || currentMeta?.name || "No pipeline selected"}
            </p>
            <p className="pipeline-editor-subtitle">
              {currentMeta?.file_rel_path || "Select a pipeline to edit."}
            </p>
          </div>
          <div className="flex min-w-0 shrink-0 items-center justify-end gap-2 overflow-x-auto">
            <div className="flex shrink-0 items-center gap-3">
              <span
                className="pipeline-editor-indicator pipeline-editor-indicator-trigger"
                data-trigger-kind={triggerKind.toLowerCase()}
              >
                {triggerKind}
              </span>
              <span
                className="pipeline-editor-indicator pipeline-editor-indicator-draft"
                data-tone={draftTone}
              >
                {draftLabel}
              </span>
              <span
                className="pipeline-editor-indicator pipeline-editor-indicator-lock"
                data-tone={lockTone}
              >
                {currentLocked ? "locked" : "editable"}
              </span>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              {isManualTrigger && (
                <Button
                  size="icon"
                  disabled={manualRunDisabled}
                  onClick={handleRunManual}
                  title={manualRunTitle}
                  aria-label={runBusy ? "Running active pipeline" : "Run active pipeline"}
                >
                  <PipelineRunIcon />
                </Button>
              )}
              <Button
                variant="outline"
                size="xs"
                disabled={currentLocked || !currentMeta}
                onClick={handleSave}
              >
                Save Draft
              </Button>
              <Button
                size="xs"
                disabled={currentLocked || !currentMeta}
                onClick={handleActivate}
              >
                Activate
              </Button>
              <Button
                variant="outline"
                size="xs"
                disabled={currentLocked || !currentMeta}
                onClick={handleDeactivate}
              >
                Deactivate
              </Button>
              {onLockToggle && currentMeta && (
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => onLockToggle(!currentLocked)}
                  title={currentLocked ? "Unlock (allow agent access)" : "Lock (block agent access)"}
                  aria-label={currentLocked ? "Unlock pipeline editor" : "Lock pipeline editor"}
                  className={currentLocked ? "text-dark-accent1" : "text-body hover:text-dark-accent1"}
                >
                  {currentLocked ? <LockIcon /> : <LockOpenIcon />}
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon"
                disabled={!currentMeta}
                onClick={openSettingsDialog}
                aria-label="Pipeline settings"
                title="Pipeline settings"
              >
                <PipelineSettingsIcon />
              </Button>
            </div>
          </div>
        </div>
        {runStatus ? (
          <div className={cx(
            "text-[0.72rem]",
            runStatus.startsWith("Run failed:") ? "text-red-400" : "text-body-soft",
          )}>
            {runStatus}
          </div>
        ) : null}
        {saveError && (
          <div className="pipeline-editor-save-error" role="alert">
            <span>⚠ {saveError}</span>
            <button onClick={() => setSaveError(null)} aria-label="Dismiss">✕</button>
          </div>
        )}
      </div>

      {/* Canvas + category tools */}
      <div
        ref={canvasRef}
        className="flex-1 min-h-0 border-b border-border-soft relative"
        onPointerDownCapture={handleMarqueeDown}
        onPointerMoveCapture={handleMarqueeMove}
        onPointerUpCapture={handleMarqueeUp}
      >
        {/* Category buttons — open node picker dialog filtered by category */}
        <div className="absolute top-3 left-3 z-[35] flex flex-col gap-1.5">
          {Object.entries(catalogGroups).map(([cat, groups]) => {
            const flatItems = groups.flatMap((g) => g.entries);
            if (!flatItems.length) return null;
            return (
              <button
                key={cat}
                type="button"
                className="w-8 h-8 shrink-0 rounded-md border border-border-soft bg-surface-2 text-body-muted flex items-center justify-center p-0 hover:bg-surface-3 hover:text-body hover:border-border transition-colors disabled:opacity-40 disabled:cursor-default"
                title={cat.charAt(0).toUpperCase() + cat.slice(1)}
                disabled={currentLocked || !currentMeta}
                onClick={() => {
                  setNodePickerCategory(cat);
                  setNodePickerQuery("");
                  setNodePickerOpen(true);
                }}
              >
                {CAT_ICONS[cat]}
              </button>
            );
          })}
        </div>

        {/* PipelineGraph canvas */}
        {PipelineGraph && (
          <PipelineGraph
            ref={graphRef}
            pipeline={currentGraph}
            readOnly={currentLocked}
            snapToGrid={snapToGrid}
            gridSize={30}
            kindIcons={kindIcons}
            kindTitles={kindTitles}
            id="pipeline-canvas"
            className="w-full h-full"
            onNodeEdit={currentLocked ? undefined : handleNodeEdit}
            onOutputAdd={currentLocked ? undefined : handleOutputAdd}
            selectionMode={selectionMode}
            onSelectionModeChange={(mode: "normal" | "box") => {
              setSelectionMode(mode);
              if (mode === "normal") setMarquee(null);
            }}
            onSelectAll={handleSelectAllNodes}
            onReady={() => {
              // No-op; edit buttons handled by PipelineGraph's MutationObserver
            }}
          />
        )}

        {/* Marquee selection overlay */}
        {marquee && (
          <div
            className="absolute border border-accent/60 bg-accent/10 pointer-events-none z-[40]"
            style={{
              left: Math.min(marquee.x1, marquee.x2),
              top: Math.min(marquee.y1, marquee.y2),
              width: Math.abs(marquee.x2 - marquee.x1),
              height: Math.abs(marquee.y2 - marquee.y1),
            }}
          />
        )}

        {/* Selection toast */}
        {selectionToast && (
          <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-50 px-3 py-1.5 rounded-md bg-surface-2 border border-border text-xs text-body font-medium shadow-md">
            {selectionToast}
          </div>
        )}
      </div>

      {/* Log panel */}
      {logsOpen && currentGraph && (
        <div className="pipeline-editor-logs-panel border-t border-border bg-bg flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-3 py-1.5 bg-surface border-b border-border flex-shrink-0">
            <span className="text-xs font-semibold text-body">
              Invocations — {currentMeta?.name || currentMeta?.title || "pipeline"}
            </span>
            <div className="flex items-center gap-2">
              <span className="text-[0.65rem] text-body-muted">auto-refresh 5s</span>
              <Button size="sm" variant="ghost" onClick={() => setLogsOpen(false)}>✕</Button>
            </div>
          </div>
          {/* Body */}
          <div className="pipeline-editor-logs-body text-xs">
            {invocations.length === 0 ? (
              <div className="p-4 text-center text-body-muted">No invocations recorded yet.</div>
            ) : invocations.map((inv: InvocationEntry, i: number) => (
              <div key={i} className="border-b border-border">
                {/* Invocation row */}
                <div
                  className={cx(
                    "flex items-center gap-2 px-3 py-1.5 cursor-pointer select-none hover:bg-surface-2",
                    inv.status === "error" && "bg-red-500/5"
                  )}
                  onClick={() => setExpandedInv(expandedInv === i ? null : i)}
                >
                  <span className="text-body-muted whitespace-nowrap shrink-0">
                    {new Date(inv.at * 1000).toLocaleString()}
                  </span>
                  {inv.run_id ? (
                    <code className="max-w-[15rem] truncate text-[0.62rem] text-body-muted">
                      {inv.run_id}
                    </code>
                  ) : null}
                  <Badge variant={inv.status === "ok" ? "default" : "destructive"}>{inv.status}</Badge>
                  <Badge variant="secondary">{inv.trigger}</Badge>
                  <span className="text-body-muted shrink-0">{inv.duration_ms}ms</span>
                  {inv.error && (
                    <span className="text-red-400 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">{inv.error}</span>
                  )}
                  <span className="text-[0.6rem] text-body-muted ml-auto shrink-0">
                    {expandedInv === i ? "▲" : "▼"}
                  </span>
                </div>
                {/* Per-node trace */}
                {expandedInv === i && Array.isArray(inv.trace) && inv.trace.length > 0 && (
                  <div className="bg-surface border-t border-border">
                    {inv.run_id ? (
                      <div className="px-3 py-2 text-[0.68rem] text-body-soft border-b border-border">
                        <span className="mr-2 uppercase tracking-[0.12em] text-body-muted">Run ID</span>
                        <code className="text-body">{inv.run_id}</code>
                      </div>
                    ) : null}
                    {inv.error && (
                      <div className="px-3 py-2 text-[0.7rem] text-red-400 bg-red-500/5 border-b border-border whitespace-pre-wrap break-all">
                        {inv.error}
                      </div>
                    )}
                    {inv.trace.map((entry: InvocationTraceEntry, j: number) => {
                      const nodeKey = `${i}-${j}`;
                      const nodeExpanded = expandedNode === nodeKey;
                      return (
                        <div key={j} className="border-b border-border/60">
                          <div
                            className={cx(
                              "flex items-center gap-2 px-5 py-1 cursor-pointer hover:bg-surface-2",
                              entry.error && "text-red-400"
                            )}
                            onClick={() => {
                              setExpandedNode(nodeExpanded ? null : nodeKey);
                              const app = (graphRef.current as any)?.getApp?.();
                              if (app?.ui && Array.isArray(app?.graph?.nodes)) {
                                app.ui.clearSelection?.();
                                const target = app.graph.nodes.find((n: any) => n.zfPipelineNodeId === entry.node_id);
                                if (target?.el) {
                                  app.ui.selectedNode = target;
                                  target.el.classList.add("selected");
                                }
                              }
                            }}
                          >
                            <code className="font-mono text-[0.7rem]">{entry.node_id}</code>
                            <span className="text-body-muted text-[0.65rem]">{entry.node_kind}</span>
                            <span className="text-body-muted text-[0.65rem] ml-auto shrink-0">{entry.duration_ms}ms</span>
                            {entry.error && <span className="text-red-400 text-[0.65rem]">{entry.error}</span>}
                            <span className="text-[0.6rem] text-body-muted shrink-0">{nodeExpanded ? "▲" : "▼"}</span>
                          </div>
                          {nodeExpanded && (
                            <div className="pipeline-editor-logs-io-grid">
                              {entry.error && (
                                <div>
                                  <span className="text-[0.6rem] font-semibold uppercase text-red-400">Error</span>
                                  <pre className="pipeline-editor-logs-io-pre border-red-500/40 text-red-100">
                                    {entry.error}
                                  </pre>
                                </div>
                              )}
                              {[
                                ["Config", entry.config],
                                ["Input", entry.input],
                                ["Output", entry.output],
                              ].filter(([, val]) => val !== undefined && val !== null).map(([label, val]) => (
                                <div key={label as string}>
                                  <span className="text-[0.6rem] font-semibold uppercase text-body-muted">{label}</span>
                                  <pre className="pipeline-editor-logs-io-pre">{JSON.stringify(val, null, 2)}</pre>
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="pipeline-editor-foot">
        <span className="pipeline-editor-foot-item">{graphuiPackageLabel}</span>
        <span className="pipeline-editor-foot-item">Success: {successCount}</span>
        <span className="pipeline-editor-foot-item">Failed: {failedCount}</span>
        <span className="pipeline-editor-foot-item">Retention: {retentionSummary}</span>
        <span className="pipeline-editor-foot-item" title={latestErr}>
          Latest error: {latestErr}
        </span>
        {multiSelected.size > 0 && (
          <span className="pipeline-editor-foot-item text-accent font-medium">
            {multiSelected.size} selected
          </span>
        )}
        {currentGraph && (
          <Button
            size="sm"
            variant={logsOpen ? "outline" : "ghost"}
            onClick={() => setLogsOpen((o) => !o)}
          >
            Logs
          </Button>
        )}
      </div>

      {/* NodeDialog */}
      <NodeDialog
        nodeData={dialogNode}
        catalog={catalog}
        dataState={dataState}
        graphRef={graphRef}
        webhookBaseUrl={
          owner && project && typeof document !== "undefined"
            ? new URL(`/wh/${owner}/${project}`, document.baseURI).href
            : ""
        }
        onApply={handleNodeApply}
        onClose={() => setDialogNode(null)}
      />

      {/* WebRenderDialog */}
      <WebRenderDialog
        nodeData={webRenderNode}
        templates={dataState.pageTemplates.map((t: any) => ({
          rel_path: String(t.rel_path || ""),
          name: String(t.name || ""),
        }))}
        api={{
          templateFile: api.templateFile,
          templateSave: api.templateSave,
          templateOutline: api.templateOutline,
          templatesWorkspace: api.templatesWorkspace,
        }}
        allGraphNodes={currentGraph?.nodes || []}
        onApply={handleNodeApply}
        onClose={() => setWebRenderNode(null)}
      />

      {/* GitCommitDialog */}
      <GitCommitDialog
        open={gitDialogOpen}
        files={gitFiles}
        gitCommitUrl={`/api/projects/${owner}/${project}/git/commit`}
        redirectUrl={gitRedirectUrl}
        onClose={() => setGitDialogOpen(false)}
      />

      <Dialog open={!!continuationRequest || nodePickerOpen} onOpenChange={(v: boolean) => {
        if (!v) {
          setContinuationRequest(null);
          setNodePickerOpen(false);
          setNodePickerQuery("");
          setNodePickerCategory("all");
        }
      }}>
        <DialogContent className="max-w-3xl border-border bg-[var(--color-surface,#161616)] text-body" onKeyDown={(e) => e.stopPropagation()}>
          <div className="flex items-start justify-between gap-4 border-b border-border px-6 py-4">
            <div>
              <DialogHeader>
                <DialogTitle>{continuationRequest ? "Add Next Node" : nodePickerCategory === "trigger" ? "Change Trigger" : "Add Node"}</DialogTitle>
              </DialogHeader>
              {continuationRequest ? (
                <p className="mt-1 text-xs text-body-muted">
                  After {continuationRequest.zfPipelineNodeId || continuationRequest.title || "node"} / {continuationRequest.outputPin || "out"}
                </p>
              ) : null}
            </div>
          </div>
          <div className="px-6 pt-3 pb-1 flex flex-col gap-2">
            {nodePickerCategory !== "trigger" && (
            <div className="flex items-center gap-1 flex-wrap">
              <button
                type="button"
                className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors ${nodePickerCategory === "all" ? "bg-accent/20 text-accent border border-accent/40" : "bg-surface-2 text-body-muted border border-border-soft hover:bg-surface-3 hover:text-body"}`}
                onClick={() => setNodePickerCategory("all")}
              >All</button>
              {Object.entries(catalogGroups).map(([cat, groups]) => {
                const count = groups.flatMap((g) => g.entries).filter((e) => !isTriggerNodeKind(e.kind)).length;
                if (!count) return null;
                return (
                  <button
                    key={cat}
                    type="button"
                    className={`inline-flex items-center gap-1 px-2 py-1 rounded text-[11px] font-medium transition-colors ${nodePickerCategory === cat ? "bg-accent/20 text-accent border border-accent/40" : "bg-surface-2 text-body-muted border border-border-soft hover:bg-surface-3 hover:text-body"}`}
                    onClick={() => setNodePickerCategory(cat)}
                  >
                    <span className="w-3.5 h-3.5 flex items-center justify-center">{CAT_ICONS[cat]}</span>
                    <span>{cat.charAt(0).toUpperCase() + cat.slice(1)}</span>
                  </button>
                );
              })}
            </div>
            )}
            <input
              className="zf-input"
              value={nodePickerQuery}
              placeholder="Search nodes..."
              autoFocus
              onInput={(e) => setNodePickerQuery((e.target as HTMLInputElement).value)}
            />
          </div>
          <div className="pipeline-editor-continuation-grid">
            {continuationItems.map(({ category, entry }) => (
              <button
                key={entry.kind}
                type="button"
                className="pipeline-editor-continuation-item"
                onClick={() => handleNodePickerSelect(entry.kind)}
              >
                <span className="text-sm font-semibold text-body">{entry.title || entry.kind}</span>
                <span className="pipeline-editor-continuation-kind">{entry.kind}</span>
                {entry.description ? (
                  <span className="line-clamp-2 text-xs text-body-soft">{entry.description}</span>
                ) : (
                  <span className="text-xs uppercase tracking-[0.12em] text-body-soft">{category}</span>
                )}
              </button>
            ))}
            {continuationItems.length === 0 ? (
              <div className="col-span-full rounded-md border border-dashed border-border px-4 py-8 text-center text-sm text-body-muted">
                No matching nodes.
              </div>
            ) : null}
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={settingsOpen} onOpenChange={(v: boolean) => !v && closeSettingsDialog()}>
        <DialogContent className="max-w-xl border-border bg-surface text-body">
          <form className="flex flex-col gap-4" onSubmit={applyPipelineSettings}>
            <DialogHeader className="px-6 pt-6">
              <DialogTitle>Pipeline Settings</DialogTitle>
              <p className="text-xs text-body-muted">
                Pipeline-specific execution log retention and destructive actions.
              </p>
            </DialogHeader>
            <div className="grid gap-3 px-6">
              <label className="pipeline-editor-field">
                <span className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={retentionInherit}
                    onInput={(e) => setRetentionInherit((e.target as HTMLInputElement).checked)}
                  />
                  <span>Use project default retention</span>
                </span>
                <small className="pipeline-editor-field-help">
                  Current inherited count limit: {projectDefaultMaxInvocations} invocation(s) per pipeline.
                </small>
              </label>
              <label className="pipeline-editor-field">
                <span>Max invocation count override</span>
                <input
                  className="zf-input"
                  type="number"
                  min="1"
                  placeholder={`${projectDefaultMaxInvocations}`}
                  value={retentionMaxInv}
                  disabled={retentionInherit}
                  onInput={(e) => setRetentionMaxInv((e.target as HTMLInputElement).value)}
                />
                <small className="pipeline-editor-field-help">
                  Optional hard cap on retained runs for this pipeline.
                </small>
              </label>
              <label className="pipeline-editor-field">
                <span>Max age override (days)</span>
                <input
                  className="zf-input"
                  type="number"
                  min="1"
                  placeholder="1"
                  value={retentionMaxAgeDays}
                  disabled={retentionInherit}
                  onInput={(e) => setRetentionMaxAgeDays((e.target as HTMLInputElement).value)}
                />
                <small className="pipeline-editor-field-help">
                  Optional time-based retention. Example: set <code>1</code> for a login pipeline that should keep only one day of runs.
                </small>
              </label>
            </div>
            <div className="mx-6 flex items-center justify-between gap-3 border-t border-border-soft pt-4">
              <div>
                <div className="text-xs font-semibold uppercase tracking-[0.12em] text-body-muted">Danger Zone</div>
                <div className="text-xs text-body-muted">Delete this pipeline from the project.</div>
              </div>
              {currentLocked ? (
                <span className="inline-flex items-center justify-center text-dark-accent1" title="Locked — cannot delete" aria-label="Locked pipeline">
                  <LockIcon />
                </span>
              ) : onDeleteClick ? (
                <Button variant="destructive" size="xs" type="button" onClick={() => { closeSettingsDialog(); onDeleteClick(); }}>
                  <PipelineDeleteIcon /> Delete Pipeline
                </Button>
              ) : null}
            </div>
            <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
              <Button variant="outline" size="xs" type="button" onClick={closeSettingsDialog}>Cancel</Button>
              <Button variant="primary" size="xs" type="submit">Apply to Draft</Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>

    </div>
  );
}
