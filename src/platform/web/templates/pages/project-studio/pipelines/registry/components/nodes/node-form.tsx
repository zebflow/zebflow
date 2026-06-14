import NodeField from "@/pages/project-studio/pipelines/registry/components/nodes/node-field";
import NodeLayout from "@/pages/project-studio/pipelines/registry/components/nodes/node-layout";
import { useFileSearchOptional } from "@/pages/project-studio/components/file-search-context";
import type {
  NodeFieldDef,
  SelectOptionDef,
  EditorDataState,
  NodeFieldType,
  LayoutItem,
} from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/types";

// ── Helpers ────────────────────────────────────────────────────────────────────

function normalizeCredentialOption(item: any): SelectOptionDef | null {
  const credentialId = String(item?.credential_id || "").trim();
  if (!credentialId) return null;
  const title = String(item?.title || "").trim();
  const kind = String(item?.kind || "").trim();
  const label = title ? `${title} (${credentialId})` : credentialId;
  return { value: credentialId, label: kind ? `${label} | ${kind}` : label };
}

function buildCredentialOptions(
  credentialList: any[],
  selectedId: string | undefined
): SelectOptionDef[] {
  const options = (Array.isArray(credentialList) ? credentialList : [])
    .map(normalizeCredentialOption)
    .filter(Boolean) as SelectOptionDef[];
  const selected = String(selectedId || "");
  if (selected && !options.some((o) => o.value === selected)) {
    options.unshift({ value: selected, label: `${selected} (not listed)` });
  }
  if (options.length === 0) {
    options.push({ value: "", label: "No credential available" });
  }
  return options;
}

function buildTemplateOptions(
  templateList: any[],
  selectedTemplate: string
): SelectOptionDef[] {
  const options = (Array.isArray(templateList) ? templateList : [])
    .map((item: any) => {
      const relPath = String(item?.rel_path || "").trim();
      if (!relPath) return null;
      const name = String(item?.name || "").trim();
      return { value: relPath, label: name ? `${name} | ${relPath}` : relPath };
    })
    .filter(Boolean) as SelectOptionDef[];
  if (selectedTemplate && !options.some((o) => o.value === selectedTemplate)) {
    options.unshift({ value: selectedTemplate, label: `${selectedTemplate} (not listed)` });
  }
  return options;
}

function webhookPublicUrlFor(dataState: EditorDataState, webhookPath: string): string {
  const owner = String(dataState?.owner || "").trim();
  const project = String(dataState?.project || "").trim();
  if (!owner || !project || typeof window === "undefined") return "";
  const base = `${window.location.origin}/wh/${owner}/${project}`;
  const norm = (String(webhookPath || "/").trim() || "/");
  const normalized = norm.startsWith("/") ? norm : `/${norm}`;
  return normalized === "/" ? base : `${base}${normalized}`;
}

function defaultFor(type: NodeFieldType): unknown {
  if (type === "checkbox") return false;
  if (type === "multi_checkbox") return [];
  if (type === "key_value_pairs") return {};
  if (type === "claims_pairs") return {};
  if (type === "params_builder") return { type: "object", required: [], properties: {} };
  if (type === "match_cases") return { cases: [], default: { pin: "default", label: "Default" } };
  return "";
}

// ── Types ──────────────────────────────────────────────────────────────────────

interface EnrichedFieldDef extends NodeFieldDef {
  value: unknown;
  secureRequestCredential?: any;
  secureRequestVariables?: any[];
}

// ── Grid span logic ────────────────────────────────────────────────────────────

const FULL_WIDTH_TYPES: NodeFieldType[] = [
  "code_editor",
  "textarea",
  "datalist",
  "method_buttons",
  "copy_url",
  "section",
  "multi_checkbox",
  "key_value_pairs",
  "claims_pairs",
  "params_builder",
  "secure_request_bindings",
  "match_cases",
  "source_bindings",
];

function isFullWidth(field: NodeFieldDef): boolean {
  if (field.span === "full") return true;
  if (field.span === "half") return false;
  return FULL_WIDTH_TYPES.includes(field.type as NodeFieldType);
}

// ── enrichFields ───────────────────────────────────────────────────────────────

function enrichFields(
  fields: NodeFieldDef[],
  config: Record<string, unknown>,
  dataState: EditorDataState
): EnrichedFieldDef[] {
  return fields.map((f) => {
    let value: unknown =
      config[f.name] !== undefined
        ? config[f.name]
        : f.default_value !== undefined
        ? f.default_value
        : defaultFor(f.type as NodeFieldType);

    let options = [...(f.options ?? [])];

    if (f.data_source === "credentials_all") {
      options = buildCredentialOptions(dataState.allCredentials, value as string);
    } else if (f.data_source === "credentials_postgres") {
      options = buildCredentialOptions(dataState.pgCredentials, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = "No postgres credential available";
      }
    } else if (f.data_source === "credentials_jwt") {
      options = buildCredentialOptions(dataState.jwtCredentials, value as string);
    } else if (f.data_source === "credentials_browser") {
      options = buildCredentialOptions(dataState.browserCredentials, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = "No browser credential available";
      }
    } else if (f.data_source === "credentials_open_ai") {
      options = buildCredentialOptions(dataState.openaiCredentials, value as string);
      if (options.length === 1 && (options[0] as SelectOptionDef).value === "") {
        (options[0] as SelectOptionDef).label = "No OpenAI credential available";
      }
    } else if (f.data_source === "credentials_secure_request") {
      options = buildCredentialOptions(dataState.secureRequestCredentials, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = "No secure request profile available";
      }
    } else if (f.data_source === "credentials_http_auth") {
      options = buildCredentialOptions(dataState.httpAuthCredentials, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = "No HTTP auth credential available";
      }
    } else if (f.data_source === "credentials_webhook_auth") {
      options = buildCredentialOptions(dataState.webhookAuthCredentials, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = "No webhook auth credential available";
      }
    } else if (f.data_source === "ai_tools") {
      options = (Array.isArray(dataState.aiTools) ? dataState.aiTools : []).map((t: any) => ({
        value: t.tool_name,
        label: t.tool_name,
        description: t.tool_description,
      }));
    } else if (f.data_source === "templates_pages") {
      options = buildTemplateOptions(dataState.pageTemplates, value as string);
    } else if (f.data_source === "function_pipelines") {
      options = (Array.isArray(dataState.functionPipelines) ? dataState.functionPipelines : []).map(
        (m: any) => ({
          value: String(m?.meta?.name || m?.name || ""),
          label: m?.meta?.title || m?.meta?.name || m?.name || "",
        })
      ).filter((o: any) => o.value);
    } else if (f.data_source === "credential_jwt_roles") {
      // Roles come from the JWT credential selected in the sibling auth_credential field.
      const selectedCredId = String(config.auth_credential ?? "");
      const cred = (dataState.jwtCredentials as any[]).find((c: any) => c.credential_id === selectedCredId);
      const roles: string[] = Array.isArray(cred?.auth_roles) ? cred.auth_roles : [];
      options = roles.map((r: string) => ({ value: r, label: r }));
    } else if (f.data_source && (f.data_source as string).startsWith("credentials:")) {
      // Dynamic credential kind filter (composite node packages).
      const kind = (f.data_source as string).slice("credentials:".length);
      const filtered = (dataState.allCredentials as any[]).filter(
        (c: any) => c.kind === kind
      );
      options = buildCredentialOptions(filtered, value as string);
      if (options.length === 1 && options[0].value === "") {
        options[0].label = `No ${kind} credential available`;
      }
    }

    if (f.type === "copy_url") {
      value = webhookPublicUrlFor(dataState, String(config.path ?? "/"));
    }
    if (f.type === "match_cases") {
      const draft = config[f.name];
      value = draft && typeof draft === "object" && !Array.isArray(draft)
        ? draft
        : {
            cases: Array.isArray(config.cases) ? config.cases : [],
            default: config.default !== undefined ? config.default : { pin: "default", label: "Default" },
          };
    }

    const enriched: EnrichedFieldDef = { ...f, value, options };
    if (f.type === "secure_request_bindings") {
      const selectedCredId = String(config.credential_id ?? "");
      const allHttpCreds = [...(dataState.secureRequestCredentials || []), ...(dataState.httpAuthCredentials || [])];
      const cred = allHttpCreds.find(
        (item: any) => String(item?.credential_id || "") === selectedCredId
      );
      const variables = Array.isArray(cred?.secure_request_vars) ? cred.secure_request_vars : [];
      const raw =
        value && typeof value === "object" && !Array.isArray(value)
          ? (value as Record<string, unknown>)
          : {};
      const seeded: Record<string, unknown> = { ...raw };
      for (const item of variables) {
        const key = String(item?.name || "").trim();
        if (!key || seeded[key] !== undefined) continue;
        const fallback = String(item?.default_expr || "").trim();
        if (fallback) seeded[key] = fallback;
      }
      enriched.value = seeded;
      enriched.secureRequestCredential = cred ?? null;
      enriched.secureRequestVariables = variables;
    }

    return enriched;
  });
}

// ── NodeForm ───────────────────────────────────────────────────────────────────

interface Props {
  fields: NodeFieldDef[];
  layout?: LayoutItem[];
  config: Record<string, unknown>;
  dataState: EditorDataState;
  onChange: (name: string, value: unknown) => void;
}

export default function NodeForm({ fields, layout, config, dataState, onChange }: Props) {
  if (!fields || fields.length === 0) return null;

  const fileSearch = useFileSearchOptional();
  const enriched = enrichFields(fields, config, dataState);

  if (layout && layout.length > 0) {
    const fieldMap = new Map(enriched.map((f) => [f.name, f as NodeFieldDef]));
    return <NodeLayout layout={layout} fieldMap={fieldMap} onChange={onChange} />;
  }

  return (
    <div className="pipeline-editor-fields-grid">
      {enriched.map((f) => (
        <div
          key={f.name}
          style={{ gridColumn: isFullWidth(f) ? "1 / -1" : undefined }}
        >
          {f.data_source === "templates_pages" && fileSearch ? (
            <div className="flex items-end gap-1">
              <div className="flex-1 min-w-0">
                <NodeField field={f} value={f.value} onChange={(val) => onChange(f.name, val)} />
              </div>
              <button
                type="button"
                onClick={() =>
                  fileSearch.openFileSearch({
                    scope: "pages",
                    onSelect: (relPath) => onChange(f.name, relPath),
                  })
                }
                title="Browse template files"
                className="mb-0.5 px-2 py-1.5 text-xs rounded border border-dark-border text-dark-text1/60 hover:text-dark-text1 hover:bg-dark-accent3 shrink-0 transition-colors"
              >
                Browse
              </button>
            </div>
          ) : (
            <NodeField
              field={f}
              value={f.value}
              onChange={(val) => onChange(f.name, val)}
            />
          )}
        </div>
      ))}
    </div>
  );
}
