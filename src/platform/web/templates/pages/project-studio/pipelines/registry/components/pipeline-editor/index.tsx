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
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import type { EditorApi, EditorDataState, PipelineNodeData, PipelineMeta, GitFile, NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";
import {
  buildNodeCatalog,
  normalizeGraphForEditor,
  normalizeNodePins,
  nodeColor,
  canonicalNodeKind,
  isTriggerNodeKind,
  nodeCategories,
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
  const graphRef = useRef(null);
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
  const [dataState, setDataState] = useState<EditorDataState>({
    pgCredentials: [],
    jwtCredentials: [],
    browserCredentials: [],
    openaiCredentials: [],
    secureRequestCredentials: [],
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
            pgCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "postgres"),
            jwtCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "jwt_signing_key"),
            browserCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase().startsWith("browser_")),
            openaiCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "openai"),
            secureRequestCredentials: items.filter((i: any) => String(i?.kind || "").toLowerCase() === "secure_request"),
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

  // ── Apply node edit from NodeDialog ──────────────────────────────────────
  function handleNodeApply(
    nodeData: PipelineNodeData,
    slug: string,
    config: Record<string, unknown>
  ) {
    // Mutate live graph node via _raw escape hatch
    const rawNode = nodeData._raw;
    if (rawNode) {
      rawNode.zfPipelineNodeId = slug;
      rawNode.zfConfig = config;
      if (config.title) {
        rawNode.title = String(config.title);
        const header = rawNode.el?.querySelector?.(".zgu-node-header");
        if (header) header.textContent = String(config.title);
      }
    }
    // Re-attach edit buttons to refresh slug badge
    const app = graphRef.current?.getApp?.();
    if (app) {
      // PipelineGraph handles this via its internal MutationObserver
      // but we can manually trigger a refresh
      setTimeout(() => {
        const root = app.root;
        if (!root) return;
        root.querySelectorAll(".zf-node-slug").forEach((badge: any) => {
          const el = badge.closest?.(".zgu-node");
          const nodeMap = new Map(app.graph.nodes.map((n: any) => [String(n.id), n]));
          const nd = nodeMap.get(el?.getAttribute?.("data-id") || "");
          if (nd) {
            badge.textContent = nd.zfPipelineNodeId || "node";
          }
        });
      }, 0);
    }
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
        inputs: normalizeNodePins(kind, "input", entry?.input_pins || [], ["in"]),
        outputs: normalizeNodePins(kind, "output", entry?.output_pins || [], ["out"]),
      });
      node.zfKind = kind;
      node.zfConfig = canonicalNodeKind(kind) === "n.trigger.webhook"
        ? { method: "GET" }
        : canonicalNodeKind(kind) === "n.trigger.mapserver"
          ? { mode: "features", source_kind: "geojson_file", bbox_required: true, max_features: 1000 }
          : {};
      node.zfPipelineNodeId =
        anchor?.zfPipelineNodeId ||
        String(kind).replace(/[^a-z0-9]+/gi, "_").replace(/^_+|_+$/g, "").toLowerCase();
      app.addNode(node);
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
    graphRef.current.addNode(kind, {
      title: entry?.title || kind,
      color: nodeColor(kind),
      input_pins: normalizeNodePins(kind, "input", entry?.input_pins || [], ["in"]),
      output_pins: normalizeNodePins(kind, "output", entry?.output_pins || [], ["out"]),
    });
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
      <div className="flex-1 min-h-0 border-b border-border-soft relative">
        {/* Category buttons */}
        <div className="absolute top-3 left-3 z-[35] flex flex-col gap-1.5">
          {Object.keys(nodeCategories).map((cat) => {
            const items = (nodeCategories[cat] || [])
              .map((k) => catalog.get(k))
              .filter(Boolean) as NodeCatalogEntry[];
            return (
              <DropdownMenu
                key={cat}
                trigger={
                  <button
                    type="button"
                    className="w-8 h-8 shrink-0 rounded-md border border-border-soft bg-surface-2 text-body-muted flex items-center justify-center p-0 hover:bg-surface-3 hover:text-body hover:border-border transition-colors disabled:opacity-40 disabled:cursor-default"
                    title={cat.charAt(0).toUpperCase() + cat.slice(1)}
                    disabled={currentLocked || !currentMeta}
                  >
                    {CAT_ICONS[cat]}
                  </button>
                }
              >
                {items.map((item) => (
                  <DropdownMenuItem
                    key={item.kind}
                    label={item.title || item.kind}
                    onClick={() => handleAddNode(item.kind)}
                  />
                ))}
              </DropdownMenu>
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
            id="pipeline-canvas"
            className="w-full h-full"
            onNodeEdit={currentLocked ? undefined : handleNodeEdit}
            onReady={() => {
              // No-op; edit buttons handled by PipelineGraph's MutationObserver
            }}
          />
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
