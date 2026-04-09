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
import type { EditorApi, EditorDataState, PipelineNodeData, PipelineMeta, GitFile, NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";
import {
  buildNodeCatalog,
  normalizeGraphForEditor,
  normalizeNodePins,
  nodeColor,
  canonicalNodeKind,
  nodeCategories,
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
  onDeleteClick,
  onLockToggle,
}: PipelineEditorProps) {
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

  // ── Dialog state ────────────────────────────────────────────────────────────
  const [dialogNode, setDialogNode] = useState<PipelineNodeData | null>(null);
  const [webRenderNode, setWebRenderNode] = useState<PipelineNodeData | null>(null);

  // ── Git commit dialog state ─────────────────────────────────────────────────
  const [gitDialogOpen, setGitDialogOpen] = useState(false);
  const [gitFiles, setGitFiles] = useState<GitFile[]>([]);
  const [gitRedirectUrl, setGitRedirectUrl] = useState("");


  // ── Invocation log panel state ───────────────────────────────────────────────
  const [logsOpen, setLogsOpen] = useState(false);
  const [invocations, setInvocations] = useState<any[]>([]);
  const [expandedInv, setExpandedInv] = useState<number | null>(null);
  const [expandedNode, setExpandedNode] = useState<string | null>(null);
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
      metadata: {
        ...(rawPipeline.metadata || {}),
        locked: currentLocked,
      },
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
    await requestJson(api.activate, {
      method: "POST",
      body: JSON.stringify({ file_rel_path: currentMeta.file_rel_path }),
    });
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
  async function fetchInvocations() {
    if (!currentMeta?.file_rel_path || !api.invocations) return;
    try {
      const data = await requestJson(
        `${api.invocations}?pipeline=${encodeURIComponent(currentMeta.file_rel_path)}`
      );
      setInvocations(Array.isArray(data?.entries) ? data.entries : []);
    } catch {}
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
        <div className="pipeline-editor-toolbar-main">
          <p className="pipeline-editor-title">
            {currentMeta?.title || currentMeta?.name || "No pipeline selected"}
          </p>
          <p className="pipeline-editor-subtitle">
            {currentMeta
              ? `${currentMeta.virtual_path} | ${currentMeta.trigger_kind} | ${currentMeta.file_rel_path}`
              : "Select a pipeline to edit."}
          </p>
        </div>
        <div className="pipeline-editor-toolbar-actions">
          <span
            className="pipeline-editor-indicator pipeline-editor-indicator-trigger"
            data-trigger-kind={triggerKind.toLowerCase()}
          >
            trigger: {triggerKind}
          </span>
          <span
            className="pipeline-editor-indicator pipeline-editor-indicator-lock"
            data-tone={lockTone}
          >
            {currentLocked ? "locked" : "editable"}
          </span>
          <span
            className="pipeline-editor-indicator pipeline-editor-indicator-draft"
            data-tone={draftTone}
          >
            {draftLabel}
          </span>
          {onLockToggle && currentMeta && (
            <Button
              variant="ghost"
              size="xs"
              onClick={() => onLockToggle(!currentLocked)}
              title={currentLocked ? "Unlock (allow agent access)" : "Lock (block agent access)"}
              className={currentLocked ? "text-dark-accent1" : "text-body hover:text-dark-accent1"}
            >
              {currentLocked ? <LockIcon /> : <LockOpenIcon />}
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
          {onDeleteClick && (
            <Button
              variant="destructive"
              size="xs"
              disabled={!currentMeta}
              onClick={onDeleteClick}
            >
              Delete
            </Button>
          )}
        </div>
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
            onNodeEdit={handleNodeEdit}
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
            ) : invocations.map((inv: any, i: number) => (
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
                    {inv.error && (
                      <div className="px-3 py-2 text-[0.7rem] text-red-400 bg-red-500/5 border-b border-border whitespace-pre-wrap break-all">
                        {inv.error}
                      </div>
                    )}
                    {inv.trace.map((entry: any, j: number) => {
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
                                  <pre className="pipeline-editor-logs-io-pre" style={{ borderColor: 'var(--color-danger, #ef4444)' }}>
                                    {entry.error}
                                  </pre>
                                </div>
                              )}
                              {[["Input", entry.input], ["Output", entry.output]].map(([label, val]) => (
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

    </div>
  );
}
