import { useEffect, useRef, useState } from "zeb";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import HelpTooltip from "@/components/ui/help-tooltip";
import type { SidebarSection } from "@/components/pipeline-editor/types";

// ── Codemirror loader (same pattern as project-db-suite-postgresql.ts) ─────────

let _cmRuntime: any = null;
let _cmPromise: Promise<any> | null = null;

async function loadCm() {
  if (_cmRuntime) return _cmRuntime;
  if (_cmPromise) return _cmPromise;
  _cmPromise = (async () => {
    if (typeof window === "undefined") return null;
    const url = new URL(
      "/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs",
      window.location.origin
    );
    const cm = await import(url.href);
    _cmRuntime = cm;
    return cm;
  })();
  return _cmPromise;
}

// ── Sidebar component ──────────────────────────────────────────────────────────

function SidebarPanel({ sections }: { sections: SidebarSection[] }) {
  if (!sections || sections.length === 0) return null;
  return (
    <div className="w-52 shrink-0 border-l border-slate-200 dark:border-slate-700 overflow-y-auto text-xs">
      {sections.map((section, i) => (
        <details key={i} open className="group">
          <summary className="flex items-center gap-1 px-3 py-1.5 cursor-pointer select-none bg-slate-50 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-700 font-semibold text-slate-600 dark:text-slate-300 list-none">
            <span className="opacity-60 group-open:rotate-90 transition-transform inline-block">▶</span>
            {section.title}
          </summary>
          <ul className="py-1">
            {(section.items || []).map((item, j) => (
              <li key={j} className="px-3 py-1 border-b border-slate-100 dark:border-slate-800 last:border-0">
                <span className="font-mono text-blue-600 dark:text-blue-400">{item.label}</span>
                {item.type_hint && (
                  <span className="ml-1 text-slate-400">: {item.type_hint}</span>
                )}
                {item.description && (
                  <p className="text-slate-500 dark:text-slate-400 mt-0.5 text-[10px] leading-snug">
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
  const externalValueRef = useRef<string>(String(value ?? ""));
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const hasSidebar = Array.isArray(field.sidebar) && field.sidebar.length > 0;

  // Mount CodeMirror on first render
  useEffect(() => {
    if (!containerRef.current) return;
    let destroyed = false;

    loadCm().then((cm) => {
      if (destroyed || !containerRef.current || !cm) return;

      const initDoc = String(value ?? "");
      externalValueRef.current = initDoc;

      const view = new cm.EditorView({
        doc: initDoc,
        extensions: [
          cm.basicSetup,
          cm.oneDark,
          cm.EditorView.updateListener.of((update: any) => {
            if (update.docChanged) {
              const newVal = update.state.doc.toString();
              externalValueRef.current = newVal;
              onChange(newVal);
            }
          }),
          ...(field.readonly ? [cm.EditorView.editable.of(false)] : []),
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
  }, []);

  // Sync external value changes (e.g., form reset when different node is opened)
  useEffect(() => {
    const newVal = String(value ?? "");
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
        {hasSidebar && (
          <button
            type="button"
            onClick={() => setSidebarOpen((v) => !v)}
            className="text-[10px] text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 px-2 py-0.5 rounded border border-slate-200 dark:border-slate-700"
          >
            {sidebarOpen ? "Hide sidebar" : "Show sidebar"}
          </button>
        )}
      </div>
      <div
        className={cx(
          "flex rounded-md border border-slate-200 dark:border-slate-700 overflow-hidden",
          "bg-[#282c34]" // oneDark background
        )}
        style={{ minHeight: "160px", maxHeight: "320px" }}
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
