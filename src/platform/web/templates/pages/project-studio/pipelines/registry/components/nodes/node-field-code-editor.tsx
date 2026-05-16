import { cx, useEffect, useRef, useState } from "zeb";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import HelpTooltip from "@/components/ui/help-tooltip";
import type { SidebarSection } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";
import { prepareCodeMirrorRuntime, subscribeEditorPreferences } from "@/pages/project-studio/components/editor-preferences";

// ── Codemirror loader (same dynamic-import pattern as db suite / template runtimes) ─────────

let _cmRuntime: any = null;
let _cmPromise: Promise<any> | null = null;

async function loadCm() {
  if (_cmRuntime) {
    await prepareCodeMirrorRuntime(_cmRuntime);
    return _cmRuntime;
  }
  if (_cmPromise) return _cmPromise;
  _cmPromise = (async () => {
    if (typeof window === "undefined") return null;
    const url = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs",
      window.location.origin
    );
    const cm = await import(url.href);
    await prepareCodeMirrorRuntime(cm);
    _cmRuntime = cm;
    return cm;
  })();
  return _cmPromise;
}

// ── Sidebar component ──────────────────────────────────────────────────────────

function SidebarPanel({ sections }: { sections: SidebarSection[] }) {
  if (!sections || sections.length === 0) return null;
  return (
    <div className="w-52 shrink-0 border-l border-gray-200 dark:border-gray-700 overflow-y-auto text-xs">
      {sections.map((section, i) => (
        <details key={i} open className="group">
          <summary className="flex items-center gap-1 px-3 py-1.5 cursor-pointer select-none bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700 font-semibold text-gray-600 dark:text-gray-300 list-none">
            <span className="opacity-60 group-open:rotate-90 transition-transform inline-block">▶</span>
            {section.title}
          </summary>
          <ul className="py-1">
            {(section.items || []).map((item, j) => (
              <li key={j} className="px-3 py-1 border-b border-gray-100 dark:border-gray-800 last:border-0">
                <span className="font-mono text-blue-600 dark:text-blue-400">{item.label}</span>
                {item.type_hint && (
                  <span className="ml-1 text-gray-400">: {item.type_hint}</span>
                )}
                {item.description && (
                  <p className="text-gray-500 dark:text-gray-400 mt-0.5 text-[10px] leading-snug">
                    {item.description}
                  </p>
                )}
              </li>
            ))}
          </ul>
        </details>
      ))}
    </div>
  );
}

// ── Main component ─────────────────────────────────────────────────────────────

interface Props {
  field: {
    name: string;
    label: string;
    language?: string;
    help?: string;
    sidebar?: SidebarSection[];
    readonly?: boolean;
    default_value?: unknown;
  };
  value: unknown;
  onChange: (val: string) => void;
}

export default function NodeFieldCodeEditor({ field, value, onChange }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<any>(null);
  const latestValueRef = useRef<string>(String(value ?? ""));
  const externalValueRef = useRef<string>(String(value ?? ""));
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [wrapLines, setWrapLines] = useState(true);
  const [editorPrefsVersion, setEditorPrefsVersion] = useState(0);
  const hasSidebar = Array.isArray(field.sidebar) && field.sidebar.length > 0;
  latestValueRef.current = String(value ?? "");

  useEffect(() => {
    return subscribeEditorPreferences(() => {
      setEditorPrefsVersion((version) => version + 1);
    });
  }, []);

  // Mount CodeMirror on first render
  useEffect(() => {
    if (!containerRef.current) return;
    let destroyed = false;
    const initDoc = latestValueRef.current;
    externalValueRef.current = initDoc;

    if (viewRef.current) {
      viewRef.current.destroy();
      viewRef.current = null;
    }

    containerRef.current.innerHTML = "";

    loadCm().then((cm) => {
      if (destroyed || !containerRef.current || !cm) return;

      const doc = latestValueRef.current;
      externalValueRef.current = doc;

      const view = new cm.EditorView({
        doc,
        extensions: [
          cm.presets.zebflow({
            kind: field.language || "text",
            autocomplete: true,
            clipboardSource: "node-field-code-editor",
            readonly: !!field.readonly,
            minHeight: "160px",
            maxHeight: "320px",
            onDocumentChange: (update: any) => {
              const newVal = update.state.doc.toString();
              latestValueRef.current = newVal;
              externalValueRef.current = newVal;
              onChange(newVal);
            },
          }),
          wrapLines ? cm.EditorView.lineWrapping : [],
        ],
        parent: containerRef.current,
      });

      viewRef.current = view;
    });

    return () => {
      destroyed = true;
      if (viewRef.current) {
        viewRef.current.destroy();
        viewRef.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editorPrefsVersion, wrapLines, field.name, field.language, field.readonly]);

  // Sync external value changes (e.g., form reset when different node is opened)
  useEffect(() => {
    const newVal = String(value ?? "");
    latestValueRef.current = newVal;
    if (!viewRef.current) return;
    if (externalValueRef.current === newVal) return;
    // External update — replace editor content
    const currentContent = viewRef.current.state.doc.toString();
    if (currentContent === newVal) return;
    externalValueRef.current = newVal;
    viewRef.current.dispatch({
      changes: { from: 0, to: currentContent.length, insert: newVal },
    });
  }, [value]);

  return (
    <Field>
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-1.5">
          <Label>{field.label}</Label>
          {field.help && <HelpTooltip text={field.help} />}
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => setWrapLines((v) => !v)}
            aria-pressed={wrapLines}
            title={wrapLines ? "Disable line wrap" : "Enable line wrap"}
            className={cx(
              "text-[10px] px-2 py-0.5 rounded border transition-colors",
              wrapLines
                ? "border-blue-300 text-blue-600 bg-blue-50 dark:border-blue-500/50 dark:text-blue-300 dark:bg-blue-500/10"
                : "border-gray-200 text-gray-400 hover:text-gray-600 dark:border-gray-700 dark:hover:text-gray-300"
            )}
          >
            Wrap
          </button>
          {hasSidebar && (
            <button
              type="button"
              onClick={() => setSidebarOpen((v) => !v)}
              className="text-[10px] text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 px-2 py-0.5 rounded border border-gray-200 dark:border-gray-700"
            >
              {sidebarOpen ? "Hide sidebar" : "Show sidebar"}
            </button>
          )}
        </div>
      </div>
      <div
        className={cx(
          "flex rounded-md border border-gray-200 dark:border-gray-700 overflow-hidden",
          "bg-[#282c34]" // oneDark background
        )}
      >
        <div
          ref={containerRef}
          className="flex-1 overflow-auto text-sm"
          style={{ minWidth: 0 }}
        />
        {hasSidebar && sidebarOpen && (
          <SidebarPanel sections={field.sidebar!} />
        )}
      </div>
    </Field>
  );
}
