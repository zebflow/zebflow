import { useState, useEffect, useRef } from "zeb";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import NodeForm from "@/pages/project-studio/pipelines/registry/components/nodes/node-form";
import type { PipelineNodeData, EditorDataState, NodeCatalogEntry } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";
import { extractNodeConfig, ensureUniqueSlug } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";
import { canonicalNodeKind } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/catalog";

interface NodeDialogProps {
  nodeData: PipelineNodeData | null;   // null = closed
  catalog: Map<string, NodeCatalogEntry>;
  dataState: EditorDataState;
  webhookBaseUrl: string;
  onApply: (nodeData: PipelineNodeData, slug: string, config: Record<string, unknown>) => void;
  onClose: () => void;
}

// ── NodeDialog ────────────────────────────────────────────────────────────────

export default function NodeDialog({
  nodeData,
  catalog,
  dataState,
  webhookBaseUrl,
  onApply,
  onClose,
}: NodeDialogProps) {
  const dialogRef = useRef(null);
  const kind = nodeData ? canonicalNodeKind(nodeData.zfKind) : "";
  const config = nodeData?.zfConfig || {};

  // Get server-defined fields from catalog
  const catalogEntry = catalog.get(kind);
  const serverFields = catalogEntry?.fields ?? [];

  // Initialize form state from config + field defaults
  const initFormState = () => {
    const s: Record<string, unknown> = {
      __node_slug: nodeData?.zfPipelineNodeId || "",
    };
    serverFields.forEach((f) => {
      s[f.name] = config[f.name] !== undefined
        ? config[f.name]
        : f.default_value !== undefined
        ? f.default_value
        : "";
    });
    return s;
  };

  const [formState, setFormState] = useState<Record<string, unknown>>({});

  // Dynamic function params (for n.function.call)
  const [functionParams, setFunctionParams] = useState<Record<string, any> | null>(null);
  const [functionParamsLoading, setFunctionParamsLoading] = useState(false);

  // Reset form state when nodeData changes
  useEffect(() => {
    if (nodeData) {
      setFormState(initFormState());
      setFunctionParams(null);
    }
  }, [nodeData?.graphNodeId, nodeData?.zfKind]);

  // Load function params when the selected function changes (n.function.call only)
  useEffect(() => {
    if (kind !== "n.function.call") return;
    const slug = String(formState.function || "").trim();
    if (!slug) { setFunctionParams(null); return; }

    const owner = String(dataState.owner || "");
    const project = String(dataState.project || "");
    if (!owner || !project) return;

    // Find the matching function pipeline to get its file_rel_path
    const match = (Array.isArray(dataState.functionPipelines) ? dataState.functionPipelines : [])
      .find((fp: any) => (fp?.meta?.name || fp?.name || "") === slug);
    const fileRelPath = match?.meta?.file_rel_path;
    if (!fileRelPath) { setFunctionParams(null); return; }

    setFunctionParamsLoading(true);
    fetch(`/api/projects/${encodeURIComponent(owner)}/${encodeURIComponent(project)}/pipelines/by-id?id=${encodeURIComponent(fileRelPath)}&include_source=true`, {
      headers: { Accept: "application/json" },
    })
      .then((r) => r.ok ? r.json() : null)
      .then((data) => {
        if (!data) { setFunctionParams(null); return; }
        let graph: any;
        try { graph = JSON.parse(data.source || "{}"); } catch { graph = {}; }
        const triggerNode = (graph?.nodes || []).find((n: any) => n.kind === "n.trigger.function");
        let params: Record<string, any> | null = null;
        if (triggerNode?.config?.params) {
          const raw = triggerNode.config.params;
          // params may be a string (code editor) or already parsed object
          if (typeof raw === "string") {
            try { params = JSON.parse(raw); } catch { params = null; }
          } else if (typeof raw === "object" && raw !== null) {
            params = raw;
          }
        }
        setFunctionParams(params && Object.keys(params).length > 0 ? params : null);

        // Auto-populate input with defaults if currently empty
        if (params && Object.keys(params).length > 0) {
          setFormState((prev) => {
            const existing = String(prev.input || "").trim();
            if (existing) return prev;
            const template: Record<string, any> = {};
            for (const [key, def] of Object.entries(params)) {
              const dflt = (def as any)?.default;
              template[key] = dflt !== undefined ? dflt : "";
            }
            return { ...prev, input: JSON.stringify(template, null, 2) };
          });
        }
      })
      .catch(() => setFunctionParams(null))
      .finally(() => setFunctionParamsLoading(false));
  }, [kind, formState.function]);

  // Sync webhook URL field when path changes
  useEffect(() => {
    if (kind !== "n.trigger.webhook") return;
    const path = String(formState.path || "/");
    const base = webhookBaseUrl || (typeof window !== "undefined" ? window.location.origin : "");
    const norm = path.startsWith("/") ? path : `/${path}`;
    const url = norm === "/" ? base : `${base}${norm}`;
    setFormState((prev) => ({ ...prev, __webhook_public_url: url }));
  }, [formState.path, kind]);

  // Sync <dialog> open/closed
  useEffect(() => {
    const el = dialogRef.current as HTMLDialogElement | null;
    if (!el) return;
    if (nodeData && !el.open) el.showModal();
    if (!nodeData && el.open) el.close();
  }, [nodeData]);

  function handleChange(name: string, value: unknown) {
    setFormState((prev) => ({ ...prev, [name]: value }));
  }

  function handleSubmit(e) {
    e.preventDefault();
    if (!nodeData) return;
    const slug = ensureUniqueSlug(
      nodeData._raw ? [nodeData._raw].concat([]) : [],
      nodeData.graphNodeId,
      String(formState.__node_slug || "")
    );
    const finalConfig = extractNodeConfig(kind, formState);
    onApply(nodeData, slug, finalConfig);
  }

  function handleCancel() {
    const el = dialogRef.current as HTMLDialogElement | null;
    if (el?.open) el.close();
    onClose();
  }

  // Derive per-param display values from formState.input JSON
  const parsedParamInput: Record<string, string> = (() => {
    if (!functionParams) return {};
    const raw = String(formState.input || "").trim();
    if (!raw) return {};
    try {
      const obj = JSON.parse(raw);
      const result: Record<string, string> = {};
      for (const key of Object.keys(functionParams)) {
        const val = obj[key];
        result[key] = val !== undefined && val !== null
          ? (typeof val === "object" ? JSON.stringify(val) : String(val))
          : "";
      }
      return result;
    } catch { return {}; }
  })();

  function handleParamInputChange(key: string, val: string) {
    const raw = String(formState.input || "").trim();
    let current: Record<string, any> = {};
    if (raw) { try { current = JSON.parse(raw); } catch { current = {}; } }
    handleChange("input", JSON.stringify({ ...current, [key]: val }, null, 2));
  }

  const title = `Edit Node | ${kind || "node"}`;
  const subtitle = catalogEntry?.description || "Configure node fields based on node contract.";

  return (
    <dialog
      ref={dialogRef}
      className="pipeline-editor-dialog"
      onClose={onClose}
    >
      <form className="pipeline-editor-dialog-form" onSubmit={handleSubmit}>
        <h3 className="pipeline-editor-dialog-title">{title}</h3>
        <p className="pipeline-editor-subtitle">{subtitle}</p>

        <div className="pipeline-editor-node-fields">
          {/* Node slug — always shown at top */}
          <Field>
            <Label>Node Slug</Label>
            <Input
              type="text"
              value={String(formState.__node_slug || "")}
              onInput={(e) => handleChange("__node_slug", e.currentTarget.value)}
            />
            <small className="text-xs text-gray-500 mt-1">
              Unique key for this node in pipeline graph edges.
            </small>
          </Field>

          {/* Server-driven fields via NodeForm.
              For n.function.call with params loaded: hide input_path (replaced by param inputs below). */}
          {serverFields.length > 0 ? (
            <NodeForm
              fields={
                kind === "n.function.call" && functionParams !== null
                  ? serverFields.filter((f) => f.name !== "input_path" && f.name !== "input")
                  : serverFields
              }
              layout={catalogEntry?.layout}
              config={formState}
              dataState={dataState}
              onChange={handleChange}
            />
          ) : (
            // Fallback for unknown nodes — show raw JSON editor
            <Field>
              <Label>Config JSON</Label>
              <textarea
                className="w-full font-mono text-xs p-2 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-950 resize-y"
                rows={10}
                value={JSON.stringify(config || {}, null, 2)}
                onInput={(e) => {
                  try {
                    const parsed = JSON.parse(e.currentTarget.value);
                    Object.entries(parsed).forEach(([k, v]) => handleChange(k, v));
                  } catch {
                    // Invalid JSON — ignore
                  }
                }}
              />
            </Field>
          )}

          {/* Dynamic function input panel (n.function.call only) */}
          {kind === "n.function.call" && formState.function && (
            <div className="mt-1 rounded border border-dark-border overflow-hidden">
              <div className="flex items-center gap-2 px-3 py-2 bg-dark-accent3/40 border-b border-dark-border">
                <span className="text-[0.7rem] font-semibold uppercase tracking-wide text-body-soft">
                  Function Input
                </span>
                {functionParamsLoading && (
                  <span className="text-[0.7rem] text-body-muted">Loading…</span>
                )}
                {!functionParamsLoading && !functionParams && (
                  <span className="text-[0.7rem] text-body-muted">
                    No params defined — passes full payload through.
                  </span>
                )}
              </div>
              {functionParams && Object.keys(functionParams).length > 0 && (
                <div className="px-3 py-3 flex flex-col gap-3">
                  {Object.entries(functionParams).map(([name, def]: [string, any]) => (
                    <Field key={name}>
                      <Label>
                        {name}
                        {def?.type && (
                          <span className="ml-1.5 text-[0.68rem] font-normal text-body-muted">
                            {def.type}
                          </span>
                        )}
                      </Label>
                      <Input
                        type="text"
                        placeholder={def?.description || `Enter ${name}…`}
                        value={parsedParamInput[name] ?? ""}
                        onInput={(e) => handleParamInputChange(name, e.currentTarget.value)}
                      />
                      {def?.description && (
                        <small className="text-xs text-body-muted">{def.description}</small>
                      )}
                    </Field>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        <div className="pipeline-editor-dialog-actions">
          <Button variant="outline" size="xs" type="button" onClick={handleCancel}>
            Cancel
          </Button>
          <Button size="xs" type="submit">
            Apply
          </Button>
        </div>
      </form>
    </dialog>
  );
}
