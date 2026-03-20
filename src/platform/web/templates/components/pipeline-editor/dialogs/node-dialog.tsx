import { useState, useEffect, useRef } from "zeb";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import NodeForm from "@/components/nodes/node-form";
import type { PipelineNodeData, EditorDataState, NodeCatalogEntry } from "@/components/pipeline-editor/types";
import { extractNodeConfig, ensureUniqueSlug } from "@/components/pipeline-editor/nodes/extract";
import { canonicalNodeKind } from "@/components/pipeline-editor/nodes/catalog";

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

  // Reset form state when nodeData changes
  useEffect(() => {
    if (nodeData) {
      setFormState(initFormState());
    }
  }, [nodeData?.graphNodeId, nodeData?.zfKind]);

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
            <small className="text-xs text-slate-500 mt-1">
              Unique key for this node in pipeline graph edges.
            </small>
          </Field>

          {/* Server-driven fields via NodeForm */}
          {serverFields.length > 0 ? (
            <NodeForm
              fields={serverFields}
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
                className="w-full font-mono text-xs p-2 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-950 resize-y"
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
