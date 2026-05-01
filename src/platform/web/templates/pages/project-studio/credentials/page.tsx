import { useState, useEffect, cx } from "zeb";
import ProjectStudioShell from "@/pages/project-studio/components/shell";
import { StudioTabNav, StudioTabLink } from "@/components/ui/studio-tab-nav";
import { StudioTable, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import { Select, SelectOption } from "@/components/ui/select";
import Badge from "@/components/ui/badge";

export const page = {
  html: { lang: "en" },
  body: { className: "font-sans" },
  navigation: "history",
};

export function getPage(input) {
  return {
    head: {
      title: input?.seo?.title ?? "",
      description: input?.seo?.description ?? "",
    },
  };
}

async function requestJson(url, options: any = {}) {
  return fetch(url, {
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
    ...options,
  }).then(async (res) => {
    if (res.status === 401) { window.location.href = "/login"; return null; }
    const data = await res.json().catch(() => null);
    if (!res.ok) throw new Error(data?.error?.message || data?.message || `${res.status}`);
    return data;
  });
}

// ── Secret field schemas ──────────────────────────────────────────────────────

const ALGORITHMS = [
  { value: "HS256", label: "HS256 — HMAC-SHA256 (symmetric)" },
  { value: "HS384", label: "HS384 — HMAC-SHA384 (symmetric)" },
  { value: "HS512", label: "HS512 — HMAC-SHA512 (symmetric)" },
  { value: "RS256", label: "RS256 — RSA-PKCS1v15-SHA256 (asymmetric)" },
  { value: "RS384", label: "RS384 — RSA-PKCS1v15-SHA384 (asymmetric)" },
  { value: "RS512", label: "RS512 — RSA-PKCS1v15-SHA512 (asymmetric)" },
  { value: "ES256", label: "ES256 — ECDSA P-256 (asymmetric)" },
  { value: "ES384", label: "ES384 — ECDSA P-384 (asymmetric)" },
];

const CREDENTIAL_KINDS = [
  "postgres", "mysql", "openai", "http", "github", "gitlab",
  "jwt_signing_key", "browser_browserless", "secure_request", "tts", "custom",
];

const REQUEST_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const TTS_PROVIDERS = [{ value: "piper", label: "Piper" }];

function defaultSecretForKind(kind: string): Record<string, any> {
  if (kind === "tts") {
    return { provider: "piper" };
  }
  return {};
}

function generateHex(bytes: number): string {
  const arr = new Uint8Array(bytes);
  crypto.getRandomValues(arr);
  return Array.from(arr).map((b) => b.toString(16).padStart(2, "0")).join("");
}

// ── TagsInput ─────────────────────────────────────────────────────────────────

function TagsInput({ value, onChange, placeholder }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string }) {
  const [text, setText] = useState("");

  function addTag() {
    const v = text.trim();
    if (!v || value.includes(v)) { setText(""); return; }
    onChange([...value, v]);
    setText("");
  }

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap gap-1 min-h-6">
        {value.map((tag) => (
          <span key={tag} className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-surface-2 border border-border text-body">
            {tag}
            <button
              type="button"
              onClick={() => onChange(value.filter((t) => t !== tag))}
              className="text-body-soft hover:text-danger leading-none cursor-pointer"
              aria-label={`Remove ${tag}`}
            >×</button>
          </span>
        ))}
        {value.length === 0 && <span className="text-xs text-body-soft italic">No roles defined</span>}
      </div>
      <div className="flex gap-1.5">
        <Input
          value={text}
          onChange={(e) => setText(e.target.value)}
          onInput={(e: any) => setText(e.target.value)}
          placeholder={placeholder || "Add role…"}
          className="flex-1"
          onKeyDown={(e: any) => { if (e.key === "Enter") { e.preventDefault(); addTag(); } }}
        />
        <Button type="button" variant="outline" size="sm" onClick={addTag}>+ Add</Button>
      </div>
    </div>
  );
}

function KeyValueEditor({
  value,
  onChange,
  addLabel = "+ Add",
  keyPlaceholder = "key",
  valuePlaceholder = "value",
  secretValue = false,
}: {
  value: Record<string, any>;
  onChange: (next: Record<string, string>) => void;
  addLabel?: string;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  secretValue?: boolean;
}) {
  const entries = Object.entries(value && typeof value === "object" ? value : {}).map(([key, item]) => [key, String(item ?? "")] as [string, string]);

  function commit(nextEntries: [string, string][]) {
    const out: Record<string, string> = {};
    for (const [key, item] of nextEntries) {
      const cleanKey = String(key || "").trim();
      if (!cleanKey) continue;
      out[cleanKey] = item;
    }
    onChange(out);
  }

  function updateAt(index: number, nextKey: string, nextValue: string) {
    const next = [...entries];
    next[index] = [nextKey, nextValue];
    commit(next);
  }

  function removeAt(index: number) {
    commit(entries.filter((_, itemIndex) => itemIndex !== index));
  }

  function addRow() {
    const existing = new Set(entries.map(([key]) => key));
    let candidate = "";
    let index = 0;
    while (!candidate || existing.has(candidate)) {
      index += 1;
      candidate = `KEY_${index}`;
    }
    commit([...entries, [candidate, ""]]);
  }

  return (
    <div className="flex flex-col gap-1.5">
      {entries.map(([key, item], index) => (
        <div key={`${key}-${index}`} className="flex items-center gap-1.5">
          <Input value={key} placeholder={keyPlaceholder} onInput={(e: any) => updateAt(index, e.target.value, item)} />
          <Input type={secretValue ? "password" : "text"} value={item} placeholder={valuePlaceholder} onInput={(e: any) => updateAt(index, key, e.target.value)} />
          <Button type="button" variant="ghost" size="xs" onClick={() => removeAt(index)}>×</Button>
        </div>
      ))}
      <div>
        <Button type="button" variant="outline" size="xs" onClick={addRow}>{addLabel}</Button>
      </div>
    </div>
  );
}

function SecureRequestVariablesEditor({
  value,
  onChange,
}: {
  value: any[];
  onChange: (next: any[]) => void;
}) {
  const items = Array.isArray(value) ? value : [];

  function updateAt(index: number, patch: Record<string, any>) {
    const next = items.map((item, itemIndex) => (itemIndex === index ? { ...item, ...patch } : item));
    onChange(next);
  }

  function removeAt(index: number) {
    onChange(items.filter((_, itemIndex) => itemIndex !== index));
  }

  function addVariable() {
    const existing = new Set(items.map((item) => String(item?.name || "").trim()));
    let counter = items.length + 1;
    let name = `VAR_${counter}`;
    while (existing.has(name)) {
      counter += 1;
      name = `VAR_${counter}`;
    }
    onChange([
      ...items,
      { name, label: "", value_type: "string", required: true, default_expr: "", description: "" },
    ]);
  }

  return (
    <div className="flex flex-col gap-2">
      {items.length === 0 ? (
        <p className="text-xs text-body-soft italic">No runtime variables declared yet.</p>
      ) : null}
      {items.map((item, index) => (
        <div key={`${item?.name || "variable"}-${index}`} className="rounded-md border border-ui-border bg-ui-bg px-3 py-3">
          <div className="grid grid-cols-2 gap-3">
            <Field label="Variable Name">
              <Input value={String(item?.name || "")} onInput={(e: any) => updateAt(index, { name: e.target.value })} placeholder="USER_ID" />
            </Field>
            <Field label="Label">
              <Input value={String(item?.label || "")} onInput={(e: any) => updateAt(index, { label: e.target.value })} placeholder="User ID" />
            </Field>
            <Field label="Type">
              <Input value={String(item?.value_type || "")} onInput={(e: any) => updateAt(index, { value_type: e.target.value })} placeholder="string" />
            </Field>
            <Field label="Default Expr">
              <Input value={String(item?.default_expr || "")} onInput={(e: any) => updateAt(index, { default_expr: e.target.value })} placeholder="ctx.nodes.n3.unit.code" />
            </Field>
            <Field label="Description" className="col-span-2">
              <Input value={String(item?.description || "")} onInput={(e: any) => updateAt(index, { description: e.target.value })} placeholder="Shown in the HTTP request node binding editor" />
            </Field>
            <label className="col-span-2 inline-flex items-center gap-2 text-sm text-body">
              <input
                type="checkbox"
                checked={item?.required !== false}
                onChange={(e: any) => updateAt(index, { required: !!e.target.checked })}
              />
              Required binding
            </label>
          </div>
          <div className="mt-3 flex justify-end">
            <Button type="button" variant="ghost" size="xs" onClick={() => removeAt(index)}>Remove Variable</Button>
          </div>
        </div>
      ))}
      <div>
        <Button type="button" variant="outline" size="xs" onClick={addVariable}>+ Add Variable</Button>
      </div>
    </div>
  );
}

// ── SecretFields ──────────────────────────────────────────────────────────────

function SecretFields({ kind, secret, onChange }: { kind: string; secret: Record<string, any>; onChange: (key: string, val: any) => void }) {
  const s = (key: string, fallback = "") => secret[key] ?? fallback;

  if (kind === "postgres") return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="Host" description="Hostname or IP of PostgreSQL server."><Input value={s("host")} onChange={(e) => onChange("host", e.target.value)} onInput={(e:any)=>onChange("host",e.target.value)} /></Field>
      <Field label="Port" description="TCP port for PostgreSQL."><Input value={s("port")} onChange={(e) => onChange("port", e.target.value)} onInput={(e:any)=>onChange("port",e.target.value)} placeholder="5432" /></Field>
      <Field label="Database" description="Database name."><Input value={s("database")} onChange={(e) => onChange("database", e.target.value)} onInput={(e:any)=>onChange("database",e.target.value)} /></Field>
      <Field label="User" description="Login username."><Input value={s("user")} onChange={(e) => onChange("user", e.target.value)} onInput={(e:any)=>onChange("user",e.target.value)} /></Field>
      <Field label="Password" description="Login password." className="col-span-2"><Input type="password" value={s("password")} onChange={(e) => onChange("password", e.target.value)} onInput={(e:any)=>onChange("password",e.target.value)} /></Field>
      <Field label="SSL Mode" description="disable, prefer, require, verify-ca, verify-full."><Input value={s("sslmode")} onChange={(e) => onChange("sslmode", e.target.value)} onInput={(e:any)=>onChange("sslmode",e.target.value)} placeholder="prefer" /></Field>
    </div>
  );

  if (kind === "mysql") return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="Host"><Input value={s("host")} onChange={(e) => onChange("host", e.target.value)} onInput={(e:any)=>onChange("host",e.target.value)} /></Field>
      <Field label="Port"><Input value={s("port")} onChange={(e) => onChange("port", e.target.value)} onInput={(e:any)=>onChange("port",e.target.value)} placeholder="3306" /></Field>
      <Field label="Database"><Input value={s("database")} onChange={(e) => onChange("database", e.target.value)} onInput={(e:any)=>onChange("database",e.target.value)} /></Field>
      <Field label="User"><Input value={s("user")} onChange={(e) => onChange("user", e.target.value)} onInput={(e:any)=>onChange("user",e.target.value)} /></Field>
      <Field label="Password" className="col-span-2"><Input type="password" value={s("password")} onChange={(e) => onChange("password", e.target.value)} onInput={(e:any)=>onChange("password",e.target.value)} /></Field>
    </div>
  );

  if (kind === "openai") return (
    <div className="flex flex-col gap-3">
      <Field label="API Key" description="Provider API token."><Input type="password" value={s("api_key")} onChange={(e) => onChange("api_key", e.target.value)} onInput={(e:any)=>onChange("api_key",e.target.value)} /></Field>
      <Field label="Base URL" description="Custom endpoint if needed."><Input value={s("base_url")} onChange={(e) => onChange("base_url", e.target.value)} onInput={(e:any)=>onChange("base_url",e.target.value)} placeholder="https://api.openai.com/v1" /></Field>
      <Field label="Default Model"><Input value={s("model")} onChange={(e) => onChange("model", e.target.value)} onInput={(e:any)=>onChange("model",e.target.value)} /></Field>
    </div>
  );

  if (kind === "http") return (
    <div className="flex flex-col gap-3">
      <Field label="Base URL"><Input value={s("base_url")} onChange={(e) => onChange("base_url", e.target.value)} onInput={(e:any)=>onChange("base_url",e.target.value)} /></Field>
      <Field label="Token" description="Bearer token or API key."><Input type="password" value={s("token")} onChange={(e) => onChange("token", e.target.value)} onInput={(e:any)=>onChange("token",e.target.value)} /></Field>
    </div>
  );

  if (kind === "github") return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="GitHub Username"><Input value={s("username")} onChange={(e) => onChange("username", e.target.value)} onInput={(e:any)=>onChange("username",e.target.value)} /></Field>
      <Field label="Git Name" description="Full name for git commits."><Input value={s("git_name")} onChange={(e) => onChange("git_name", e.target.value)} onInput={(e:any)=>onChange("git_name",e.target.value)} /></Field>
      <Field label="Git Email" description="Email for git commits."><Input value={s("git_email")} onChange={(e) => onChange("git_email", e.target.value)} onInput={(e:any)=>onChange("git_email",e.target.value)} /></Field>
      <Field label="Personal Access Token" description="PAT with repo scope." className="col-span-2"><Input type="password" value={s("token")} onChange={(e) => onChange("token", e.target.value)} onInput={(e:any)=>onChange("token",e.target.value)} /></Field>
    </div>
  );

  if (kind === "gitlab") return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="Instance URL" className="col-span-2"><Input value={s("url")} onChange={(e) => onChange("url", e.target.value)} onInput={(e:any)=>onChange("url",e.target.value)} placeholder="https://gitlab.com" /></Field>
      <Field label="GitLab Username"><Input value={s("username")} onChange={(e) => onChange("username", e.target.value)} onInput={(e:any)=>onChange("username",e.target.value)} /></Field>
      <Field label="Personal Access Token" className="col-span-2"><Input type="password" value={s("token")} onChange={(e) => onChange("token", e.target.value)} onInput={(e:any)=>onChange("token",e.target.value)} /></Field>
      <Field label="Git Name"><Input value={s("git_name")} onChange={(e) => onChange("git_name", e.target.value)} onInput={(e:any)=>onChange("git_name",e.target.value)} /></Field>
      <Field label="Git Email"><Input value={s("git_email")} onChange={(e) => onChange("git_email", e.target.value)} onInput={(e:any)=>onChange("git_email",e.target.value)} /></Field>
    </div>
  );

  if (kind === "jwt_signing_key") return (
    <div className="flex flex-col gap-3">
      <Field label="Algorithm" description="HS* uses a shared secret; RS*/ES* use a private key.">
        <Select value={s("algorithm", "HS256")} onChange={(e) => onChange("algorithm", e.target.value)}>
          {ALGORITHMS.map((a) => <SelectOption key={a.value} value={a.value} label={a.label} />)}
        </Select>
      </Field>
      <Field label="HMAC Secret" description="Secret for HS* algorithms.">
        <div className="flex gap-1.5">
          <Input type="password" value={s("secret")} onChange={(e) => onChange("secret", e.target.value)} onInput={(e:any)=>onChange("secret",e.target.value)} className="flex-1" />
          <Button type="button" variant="outline" size="sm" onClick={() => onChange("secret", generateHex(32))}>Generate</Button>
        </div>
      </Field>
      <Field label="Private Key (PEM)" description="PEM private key for RS*/ES* algorithms. Leave blank for HS*.">
        <textarea
          value={s("private_key")}
          onChange={(e) => onChange("private_key", e.target.value)}
          onInput={(e:any) => onChange("private_key", e.target.value)}
          rows={5}
          className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40 disabled:opacity-50"
        />
      </Field>
      <Field label="Unauthenticated Redirect" description="Where to redirect when token is missing/invalid. Leave blank for 401 JSON.">
        <Input value={s("auth_redirect")} onChange={(e) => onChange("auth_redirect", e.target.value)} onInput={(e:any)=>onChange("auth_redirect",e.target.value)} placeholder="/login" />
      </Field>
      <Field label="Forbidden Redirect" description="Where to redirect when token is valid but role is insufficient. Leave blank for 403 JSON.">
        <Input value={s("auth_forbidden_redirect")} onChange={(e) => onChange("auth_forbidden_redirect", e.target.value)} onInput={(e:any)=>onChange("auth_forbidden_redirect",e.target.value)} placeholder="/403" />
      </Field>
      <Field label="Allowed Roles" description="Roles available for this credential. Used by webhook nodes to populate the Required Role checkboxes.">
        <TagsInput
          value={Array.isArray(secret.auth_roles) ? secret.auth_roles : []}
          onChange={(roles) => onChange("auth_roles", roles)}
          placeholder="e.g. admin"
        />
      </Field>
    </div>
  );

  if (kind === "browser_browserless") return (
    <div className="flex flex-col gap-3">
      <Field label="URL" description="Browserless instance root URL."><Input value={s("url")} onChange={(e) => onChange("url", e.target.value)} onInput={(e:any)=>onChange("url",e.target.value)} placeholder="http://localhost:3000" /></Field>
      <Field label="Token" description="Optional API token."><Input type="password" value={s("token")} onChange={(e) => onChange("token", e.target.value)} onInput={(e:any)=>onChange("token",e.target.value)} /></Field>
    </div>
  );

  if (kind === "tts") return (
    <div className="flex flex-col gap-4">
      <div className="rounded-md border border-ui-border bg-surface-1 px-3 py-3">
        <p className="text-sm font-medium text-body">Local TTS Runtime Binding</p>
        <p className="mt-1 text-xs leading-relaxed text-body-soft">
          These paths are relative to the project <code>files/private/</code> root.
          For Piper, point to the ONNX model, its JSON config, and the
          <code>espeak-ng-data</code> directory.
        </p>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Provider">
          <Select value={s("provider", "piper")} onChange={(e) => onChange("provider", e.target.value)}>
            {TTS_PROVIDERS.map((provider) => (
              <SelectOption key={provider.value} value={provider.value} label={provider.label} />
            ))}
          </Select>
        </Field>
        <Field label="Voice Label" description="Optional human label for this voice preset.">
          <Input value={s("voice")} onInput={(e:any)=>onChange("voice", e.target.value)} placeholder="arin" />
        </Field>
        <Field label="Model File" className="col-span-2" description="Private-relative ONNX model path. Example: voices/arin/arin-2449.onnx">
          <Input value={s("model_file")} onInput={(e:any)=>onChange("model_file", e.target.value)} placeholder="voices/arin/arin-2449.onnx" />
        </Field>
        <Field label="Config File" className="col-span-2" description="Private-relative Piper JSON config path. Example: voices/arin/arin-2449.onnx.json">
          <Input value={s("config_file")} onInput={(e:any)=>onChange("config_file", e.target.value)} placeholder="voices/arin/arin-2449.onnx.json" />
        </Field>
        <Field label="Espeak Data Dir" className="col-span-2" description="Private-relative directory path to espeak-ng-data. Example: runtime/espeak-ng-data">
          <Input value={s("espeak_data_dir")} onInput={(e:any)=>onChange("espeak_data_dir", e.target.value)} placeholder="runtime/espeak-ng-data" />
        </Field>
      </div>
    </div>
  );

  if (kind === "secure_request") {
    const request = secret.request && typeof secret.request === "object" ? secret.request : {};
    const variables = Array.isArray(secret.variables) ? secret.variables : [];
    const secrets = secret.secrets && typeof secret.secrets === "object" ? secret.secrets : {};
    const requestMethod = String(request.method || "GET");
    const requestUrl = String(request.url || "");
    const requestBody = String(request.body || "");
    const requestHeaders = request.headers && typeof request.headers === "object" ? request.headers : {};
    const updateRequest = (patch: Record<string, any>) => onChange("__json__", {
      ...secret,
      request: {
        ...request,
        ...patch,
      },
    });
    return (
      <div className="flex flex-col gap-4">
        <div className="rounded-md border border-ui-border bg-surface-1 px-3 py-3">
          <p className="text-sm font-medium text-body">Secure Request Profile</p>
          <p className="mt-1 text-xs leading-relaxed text-body-soft">
            Define an HTTP request template with placeholders like <code>&lt;USER_ID&gt;</code> and
            <code>&lt;PROGRAMME_CODE&gt;</code>. The HTTP request node will ask for those bindings and
            resolve any secret placeholders from this credential.
          </p>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="Request Method">
            <Select value={requestMethod} onChange={(e) => updateRequest({ method: e.target.value })}>
              {REQUEST_METHODS.map((method) => <SelectOption key={method} value={method} label={method} />)}
            </Select>
          </Field>
          <Field label="URL Template" className="col-span-2" description="Use placeholders such as <USER_ID> or <SHARED_SECRET>.">
            <Input value={requestUrl} onInput={(e: any) => updateRequest({ url: e.target.value })} placeholder="https://partner.example.com/login?id=<USER_ID>&secret=<SHARED_SECRET>" />
          </Field>
        </div>

        <Field label="Header Templates" description="Header values can also use placeholders.">
          <KeyValueEditor
            value={requestHeaders}
            onChange={(headers) => updateRequest({ headers })}
            addLabel="+ Add Header"
            keyPlaceholder="Header-Name"
            valuePlaceholder="<PLACEHOLDER> or static value"
          />
        </Field>

        <Field label="Body Template" description="Optional raw request body template. Leave blank for no body.">
          <textarea
            value={requestBody}
            onChange={(e) => updateRequest({ body: e.target.value })}
            onInput={(e: any) => updateRequest({ body: e.target.value })}
            rows={5}
            placeholder='{"user_id":"<USER_ID>","programme":"<PROGRAMME_CODE>"}'
            className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
          />
        </Field>

        <Field label="Secret Placeholders" description="These values stay in the credential and can be referenced as placeholders in the request template.">
          <KeyValueEditor
            value={secrets}
            onChange={(nextSecrets) => onChange("__json__", { ...secret, secrets: nextSecrets })}
            addLabel="+ Add Secret"
            keyPlaceholder="SHARED_SECRET"
            valuePlaceholder="Stored secret value"
            secretValue
          />
        </Field>

        <Field label="Runtime Variables" description="These become bindable fields inside the HTTP request node.">
          <SecureRequestVariablesEditor
            value={variables}
            onChange={(nextVariables) => onChange("__json__", { ...secret, variables: nextVariables })}
          />
        </Field>
      </div>
    );
  }

  // custom
  return (
    <Field label="Secret JSON" description="Stored as raw JSON object.">
      <textarea
        value={typeof secret === "object" ? JSON.stringify(secret, null, 2) : String(secret ?? "")}
        onChange={(e) => { try { onChange("__json__", JSON.parse(e.target.value)); } catch { onChange("__json_raw__", e.target.value); } }}
        onInput={(e: any) => { try { onChange("__json__", JSON.parse(e.target.value)); } catch { onChange("__json_raw__", e.target.value); } }}
        rows={8}
        placeholder={'{\n  "key": "value"\n}'}
        className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
      />
    </Field>
  );
}

// ── CredentialDialog ──────────────────────────────────────────────────────────

function CredentialDialog({ open, onClose, mode, editItem, apiList, apiItemBase, onSaved }) {
  const [credentialId, setCredentialId] = useState("");
  const [title, setTitle] = useState("");
  const [kind, setKind] = useState("postgres");
  const [notes, setNotes] = useState("");
  const [secret, setSecret] = useState<Record<string, any>>({});
  const [status, setStatus] = useState("");
  const [statusTone, setStatusTone] = useState("info");
  const [busy, setBusy] = useState(false);

  // Load existing credential when editing
  useEffect(() => {
    if (!open) return;
    if (mode === "create") {
      setCredentialId(""); setTitle(""); setKind("postgres");
      setNotes(""); setSecret(defaultSecretForKind("postgres")); setStatus("Fill fields and save."); setStatusTone("info");
      return;
    }
    if (!editItem) return;
    setCredentialId(editItem.credential_id || "");
    setTitle(editItem.title || "");
    setKind(editItem.kind || "custom");
    setNotes(editItem.notes || "");
    setSecret({});
    setStatus("Loading…"); setStatusTone("info");
    setBusy(true);
    requestJson(`${apiItemBase}/${encodeURIComponent(editItem.credential_id)}`).then((payload) => {
      const item = payload?.credential || payload?.item || {};
      setSecret(item.secret && typeof item.secret === "object" ? item.secret : {});
      setStatus("Loaded. Update fields and save."); setStatusTone("ok");
    }).catch((err) => {
      setStatus(`Failed to load: ${err?.message || err}`); setStatusTone("error");
    }).finally(() => setBusy(false));
  }, [open, mode, editItem?.credential_id]);

  function setSecretField(key: string, val: any) {
    if (key === "__json__") { setSecret(val); return; }
    if (key === "__json_raw__") return; // invalid JSON, ignore
    setSecret((prev) => ({ ...prev, [key]: val }));
  }

  async function handleSave(e) {
    e.preventDefault();
    if (busy) return;
    const id = mode === "edit" ? credentialId : credentialId.trim().toLowerCase().replace(/[^a-z0-9._-]+/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "");
    if (!id) { setStatus("Credential ID is required."); setStatusTone("error"); return; }
    if (!title.trim()) { setStatus("Title is required."); setStatusTone("error"); return; }

    setBusy(true); setStatus("Saving…"); setStatusTone("info");
    try {
      const payload = { credential_id: id, title: title.trim(), kind, notes: notes.trim(), secret };
      if (mode === "edit") {
        await requestJson(`${apiItemBase}/${encodeURIComponent(id)}`, { method: "PUT", body: JSON.stringify(payload) });
      } else {
        await requestJson(apiList, { method: "POST", body: JSON.stringify(payload) });
      }
      onSaved();
      onClose();
    } catch (err: any) {
      setStatus(`Save failed: ${err?.message || err}`); setStatusTone("error");
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (mode !== "edit" || !credentialId || busy) return;
    if (!confirm(`Delete credential "${credentialId}"?`)) return;
    setBusy(true); setStatus("Deleting…"); setStatusTone("info");
    try {
      await requestJson(`${apiItemBase}/${encodeURIComponent(credentialId)}`, { method: "DELETE" });
      onSaved(); onClose();
    } catch (err: any) {
      setStatus(`Delete failed: ${err?.message || err}`); setStatusTone("error");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>{mode === "edit" ? `Edit — ${credentialId}` : "Create Credential"}</DialogTitle>
          <p className={cx("text-xs mt-0.5", statusTone === "error" ? "text-danger" : statusTone === "ok" ? "text-success" : "text-body-soft")}>{status}</p>
        </DialogHeader>

        <form onSubmit={handleSave} className="flex flex-col gap-4 px-6 py-4">
          {/* Identity */}
          <div className="grid grid-cols-2 gap-3">
            <Field label="Credential ID">
              <Input
                value={credentialId}
                onChange={(e) => setCredentialId(e.target.value)}
                onInput={(e: any) => setCredentialId(e.target.value)}
                placeholder="pg-main"
                disabled={mode === "edit" || busy}
                required
              />
            </Field>
            <Field label="Kind">
              <Select value={kind} onChange={(e) => { const nextKind = e.target.value; setKind(nextKind); setSecret(defaultSecretForKind(nextKind)); }} disabled={busy}>
                {CREDENTIAL_KINDS.map((k) => <SelectOption key={k} value={k} label={k} />)}
              </Select>
            </Field>
            <Field label="Title" className="col-span-2">
              <Input value={title} onChange={(e) => setTitle(e.target.value)} onInput={(e: any) => setTitle(e.target.value)} placeholder="Main Postgres" required disabled={busy} />
            </Field>
          </div>

          {/* Dynamic secret fields */}
          <SecretFields kind={kind} secret={secret} onChange={setSecretField} />

          {/* Notes */}
          <Field label="Notes">
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              onInput={(e: any) => setNotes(e.target.value)}
              rows={2}
              placeholder="Optional operational notes (no secrets here)"
              disabled={busy}
              className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40 disabled:opacity-50"
            />
          </Field>

          <DialogFooter>
            {mode === "edit" && (
              <Button type="button" variant="destructive" size="sm" onClick={handleDelete} disabled={busy}>Delete</Button>
            )}
            <Button type="button" variant="ghost" size="sm" onClick={onClose} disabled={busy}>Cancel</Button>
            <Button type="submit" size="sm" disabled={busy}>{busy ? "Saving…" : "Save"}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export default function Page(input) {
  const apiList = input?.credentials?.api?.list ?? "";
  const apiItemBase = input?.credentials?.api?.item_base ?? "";

  const [items, setItems] = useState<any[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<"create" | "edit">("create");
  const [editItem, setEditItem] = useState<any>(null);

  async function loadList() {
    try {
      const payload = await requestJson(apiList);
      setItems(Array.isArray(payload?.items) ? payload.items : []);
    } catch {}
  }

  useEffect(() => { loadList(); }, []);

  function openCreate() {
    setDialogMode("create"); setEditItem(null); setDialogOpen(true);
  }
  function openEdit(item: any) {
    setDialogMode("edit"); setEditItem(item); setDialogOpen(true);
  }

  function formatTs(ts) {
    if (!Number.isFinite(Number(ts))) return "-";
    return new Date(Number(ts) * 1000).toISOString().slice(0, 19).replace("T", " ");
  }

  return (
    <ProjectStudioShell
      projectHref={input.project_href}
      projectLabel={input.title}
      currentMenu="Credentials"
      owner={input.owner}
      project={input.project}
      nav={input.nav}
    >
      <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
        <StudioTabNav>
          <StudioTabLink active>Credentials</StudioTabLink>
        </StudioTabNav>
        <section className="flex-1 min-h-0 overflow-auto flex flex-col bg-bg">
          <div className="project-content-wrap">
            <section className="project-content-section">
              <div className="project-content-head">
                <div>
                  <p className="project-content-title">Credentials</p>
                  <p className="project-content-copy">Create and manage project credential records used by database and service nodes.</p>
                </div>
                <Button size="sm" onClick={openCreate}>+ New Credential</Button>
              </div>
            </section>

            <section className="project-content-section">
              <div className="project-content-body">
                <StudioTable>
                  <StudioThead>
                    <tr>
                      <StudioTh>ID</StudioTh>
                      <StudioTh>Title</StudioTh>
                      <StudioTh>Kind</StudioTh>
                      <StudioTh>Roles</StudioTh>
                      <StudioTh>Secret</StudioTh>
                      <StudioTh>Updated</StudioTh>
                      <StudioTh>Action</StudioTh>
                    </tr>
                  </StudioThead>
                  <tbody>
                    {items.length === 0 ? (
                      <tr>
                        <td colSpan={7} className="px-3 py-4 text-sm text-body-soft text-center">No credentials yet</td>
                      </tr>
                    ) : items.map((item) => (
                      <tr key={item.credential_id}>
                        <td className="px-3 py-2 text-sm font-mono text-body">{item.credential_id}</td>
                        <td className="px-3 py-2 text-sm text-body">{item.title}</td>
                        <td className="px-3 py-2 text-sm text-body-soft">{item.kind}</td>
                        <td className="px-3 py-2">
                          <div className="flex flex-wrap gap-1">
                            {(item.auth_roles || []).map((r) => (
                              <Badge key={r} variant="outline" className="text-xs">{r}</Badge>
                            ))}
                          </div>
                        </td>
                        <td className="px-3 py-2 text-sm text-body-soft">{item.has_secret ? "yes" : "no"}</td>
                        <td className="px-3 py-2 text-sm text-body-soft">{formatTs(item.updated_at)}</td>
                        <td className="px-3 py-2">
                          <Button size="xs" variant="outline" onClick={() => openEdit(item)}>Edit</Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </StudioTable>
              </div>
            </section>
          </div>
        </section>
      </div>

      <CredentialDialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        mode={dialogMode}
        editItem={editItem}
        apiList={apiList}
        apiItemBase={apiItemBase}
        onSaved={loadList}
      />
    </ProjectStudioShell>
  );
}
