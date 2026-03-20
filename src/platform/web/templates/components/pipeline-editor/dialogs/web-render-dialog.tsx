import { useState, useEffect, useRef } from "zeb";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { loadEditorRuntime } from "@/components/behavior/template-editor-runtime";
import type { PipelineNodeData } from "@/components/pipeline-editor/types";
import { ensureUniqueSlug } from "@/components/pipeline-editor/nodes/extract";

interface WebRenderDialogProps {
  nodeData: PipelineNodeData | null;  // null = closed
  templates: { rel_path: string; name?: string }[];
  api: { templateFile: string; templateSave: string };
  allGraphNodes: any[];
  onApply: (nodeData: PipelineNodeData, slug: string, config: Record<string, unknown>) => void;
  onClose: () => void;
}

export default function WebRenderDialog({
  nodeData,
  templates,
  api,
  allGraphNodes,
  onApply,
  onClose,
}: WebRenderDialogProps) {
  const dialogRef = useRef(null);
  const editorHostRef = useRef(null);
  const editorViewRef = useRef<any>(null);
  const runtimeRef = useRef<any>(null);

  const config = nodeData?.zfConfig || {};
  const [slugValue, setSlugValue] = useState(nodeData?.zfPipelineNodeId || "");
  const [titleValue, setTitleValue] = useState(String(config?.title || ""));
  const [currentPath, setCurrentPath] = useState(
    String(config?.template_path || config?.template_rel_path || "")
  );
  const [isDirty, setIsDirty] = useState(false);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("Idle");

  // Reset state when nodeData changes
  useEffect(() => {
    if (!nodeData) return;
    const cfg = nodeData.zfConfig || {};
    setSlugValue(nodeData.zfPipelineNodeId || "");
    setTitleValue(String(cfg.title || ""));
    setCurrentPath(String(cfg.template_path || cfg.template_rel_path || ""));
    setIsDirty(false);
    setQuery("");
    setStatus("Idle");
  }, [nodeData?.graphNodeId]);

  // Mount/unmount CodeMirror
  useEffect(() => {
    if (!nodeData || !editorHostRef.current) return;

    let cancelled = false;
    (async () => {
      setStatus("Loading editor…");
      try {
        let rt = runtimeRef.current;
        if (!rt) {
          rt = await loadEditorRuntime();
          runtimeRef.current = rt;
        }
        if (cancelled) return;
        const {
          EditorView,
          basicSetup,
          javascript,
          oneDark,
        } = rt.cm;
        const view = new EditorView({
          doc: "",
          parent: editorHostRef.current,
          extensions: [
            basicSetup,
            oneDark,
            javascript({ jsx: true, typescript: true }),
            EditorView.updateListener.of((update) => {
              if (!update.docChanged) return;
              setIsDirty(true);
              setStatus("Unsaved");
            }),
          ],
        });
        editorViewRef.current = view;
        setStatus("Choose template");

        // Auto-load if template was pre-selected
        const preSelected = String(
          (nodeData.zfConfig?.template_path || nodeData.zfConfig?.template_rel_path || "") as string
        ).trim();
        if (preSelected) {
          await doLoadTemplate(preSelected, view);
        }
      } catch (err: any) {
        if (!cancelled) setStatus(`Error: ${err?.message || err}`);
      }
    })();

    return () => {
      cancelled = true;
      if (editorViewRef.current) {
        try { editorViewRef.current.destroy(); } catch {}
        editorViewRef.current = null;
      }
    };
  }, [nodeData?.graphNodeId]);

  // Sync <dialog> open/closed
  useEffect(() => {
    const el = dialogRef.current as HTMLDialogElement | null;
    if (!el) return;
    if (nodeData && !el.open) el.showModal();
    if (!nodeData && el.open) el.close();
  }, [nodeData]);

  async function doLoadTemplate(relPath: string, view?: any) {
    const path = String(relPath || "").trim();
    const edView = view || editorViewRef.current;
    if (!path) {
      setCurrentPath("");
      setIsDirty(false);
      if (edView) edView.dispatch({ changes: { from: 0, to: edView.state.doc.length, insert: "" } });
      setStatus("Idle");
      return;
    }
    if (!api.templateFile) {
      setStatus("Template API missing");
      return;
    }
    setStatus("Loading…");
    try {
      const res = await fetch(`${api.templateFile}?path=${encodeURIComponent(path)}`, {
        headers: { Accept: "application/json" },
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(payload?.message || `HTTP ${res.status}`);
      const content = String(payload?.content || "");
      setCurrentPath(path);
      setIsDirty(false);
      if (edView) {
        edView.dispatch({ changes: { from: 0, to: edView.state.doc.length, insert: content } });
      }
      setStatus("Loaded");
    } catch (err: any) {
      setStatus(`Error: ${err?.message || err}`);
    }
  }

  async function doSaveTemplate() {
    const path = currentPath.trim();
    if (!path || !api.templateSave) return;
    setStatus("Saving…");
    try {
      const content = editorViewRef.current?.state.doc.toString() || "";
      const res = await fetch(api.templateSave, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ rel_path: path, content }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setIsDirty(false);
      setStatus("Saved");
    } catch (err: any) {
      setStatus(`Error: ${err?.message || err}`);
    }
  }

  async function handleSelectTemplate(relPath: string) {
    if (isDirty) {
      await doSaveTemplate().catch(() => {});
    }
    await doLoadTemplate(relPath);
  }

  function handleSubmit(e) {
    e.preventDefault();
    if (!nodeData) return;

    const slug = ensureUniqueSlug(
      allGraphNodes,
      nodeData.graphNodeId,
      slugValue
    );

    const nextConfig: Record<string, unknown> = { ...(nodeData.zfConfig || {}) };
    if (!nextConfig.route) nextConfig.route = "/";
    const title = titleValue.trim();
    if (title) {
      nextConfig.title = title;
    } else {
      delete nextConfig.title;
    }
    if (currentPath.trim()) {
      nextConfig.template_path = currentPath.trim();
      nextConfig.template_rel_path = currentPath.trim();
      nextConfig.template_id = currentPath.trim()
        .replace(/^pages\//, "")
        .replace(/\.(tsx|jsx|ts|js)$/i, "")
        .replace(/[\\/]+/g, ".")
        .replace(/[^a-zA-Z0-9._-]+/g, "_");
    }

    // Kick off a save before applying
    if (isDirty) {
      doSaveTemplate().catch(() => {});
    }

    onApply(nodeData, slug, nextConfig);
  }

  const visibleTemplates = templates.filter((t) => {
    if (!query.trim()) return true;
    return String(t.rel_path || "").toLowerCase().includes(query.toLowerCase());
  });

  return (
    <dialog
      ref={dialogRef}
      className="pipeline-editor-dialog is-fullscreen"
      onClose={onClose}
    >
      <form className="pipeline-editor-dialog-form" onSubmit={handleSubmit}>
        <h3 className="pipeline-editor-dialog-title">Edit Node | n.web.render</h3>
        <p className="pipeline-editor-subtitle">Set slug/title/template, then edit the selected template directly.</p>

        <div className="pipeline-editor-node-fields is-web-render">
          {/* Top row: slug + title */}
          <div className="pipeline-render-top">
            <label className="pipeline-editor-field">
              <span>Node Slug</span>
              <Input
                value={slugValue}
                onInput={(e) => setSlugValue(e.currentTarget.value)}
              />
            </label>
            <label className="pipeline-editor-field">
              <span>Title</span>
              <Input
                value={titleValue}
                onInput={(e) => setTitleValue(e.currentTarget.value)}
              />
            </label>
          </div>

          {/* Workspace: sidebar + editor */}
          <div className="pipeline-render-workspace">
            <aside className="pipeline-render-sidebar">
              <div className="pipeline-render-sidebar-head">
                <div className="pipeline-render-sidebar-title">Select template</div>
                <Input
                  placeholder="Search template path..."
                  value={query}
                  onInput={(e) => setQuery(e.currentTarget.value)}
                />
              </div>
              <div className="pipeline-render-sidebar-list">
                {visibleTemplates.length === 0 ? (
                  <div className="pipeline-render-template-empty">No template matched.</div>
                ) : (
                  visibleTemplates.map((t) => (
                    <button
                      key={t.rel_path}
                      type="button"
                      className={`pipeline-render-template-item${t.rel_path === currentPath ? " is-selected" : ""}`}
                      onClick={() => handleSelectTemplate(t.rel_path)}
                    >
                      {t.rel_path}
                    </button>
                  ))
                )}
              </div>
            </aside>

            <section className="pipeline-render-editor-shell">
              <div className="pipeline-render-editor-head">
                <span>{currentPath || "No template selected"}</span>
                <div className="pipeline-render-editor-actions">
                  <span className="pipeline-render-status">{status}</span>
                  <button
                    type="button"
                    className="project-inline-chip"
                    onClick={() => doSaveTemplate().catch(() => {})}
                  >
                    Save Template
                  </button>
                </div>
              </div>
              <div className="pipeline-render-editor-host" ref={editorHostRef} />
            </section>
          </div>
        </div>

        <div className="pipeline-editor-dialog-actions">
          <Button variant="outline" size="xs" type="button" onClick={onClose}>
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
