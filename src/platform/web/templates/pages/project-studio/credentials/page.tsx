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

// ── Constants ─────────────────────────────────────────────────────────────────

const FALLBACK_KINDS = [
  "postgres", "mysql", "openai", "http", "github", "gitlab",
  "jwt_signing_key", "browser_browserless", "oauth2", "hmac", "api_key", "tts", "secure_request", "custom",
];

const REQUEST_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];

function listToText(value: any): string {
  return Array.isArray(value) ? value.map((item) => String(item ?? "")).filter(Boolean).join("\n") : "";
}

function textToList(value: string): string[] {
  return String(value || "").split(/[\n,]+/).map((item) => item.trim()).filter(Boolean);
}

function defaultSecretForKind(kind: string, types: any[]): Record<string, any> {
  // Complex types with interactive defaults
  if (kind === "oauth2") {
    return {
      provider: "", client_id: "", client_secret: "", authorize_url: "",
      token_url: "", scopes: "", redirect_uri: "", refresh_token: "",
      access_token: "", expires_at: 0, token_type: "Bearer",
    };
  }
  if (kind === "hmac") {
    return {
      provider: "generic", secret: "", signature_header: "X-Hub-Signature-256",
      signature_encoding: "hex", signature_prefix: "sha256=", algorithm: "sha256",
      replay_tolerance: 0,
    };
  }
  // Derive defaults from API field definitions
  const typeDef = types.find((t) => t.kind === kind);
  if (!typeDef) return {};
  const defaults: Record<string, any> = {};
  for (const field of typeDef.fields || []) {
    if (field.default != null) defaults[field.key] = field.default;
  }
  return defaults;
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

// ── Dynamic field renderer (driven by credential-types API) ───────────────────

function DynamicField({ field, secret, onChange }: { field: any; secret: Record<string, any>; onChange: (key: string, val: any) => void }) {
  const value = secret[field.key] ?? field.default ?? "";
  const className = field.fullWidth ? "col-span-2" : "";
  const ft = field.type || "text";

  if (ft === "select") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <Select value={value} onChange={(e) => onChange(field.key, e.target.value)}>
          {(field.options || []).map((o) => <SelectOption key={o.value} value={o.value} label={o.label} />)}
        </Select>
      </Field>
    );
  }

  if (ft === "textarea") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <textarea
          value={value}
          onChange={(e) => onChange(field.key, e.target.value)}
          onInput={(e: any) => onChange(field.key, e.target.value)}
          rows={field.rows || 4}
          placeholder={field.placeholder || ""}
          className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40 disabled:opacity-50"
        />
      </Field>
    );
  }

  if (ft === "tags") {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <TagsInput
          value={Array.isArray(secret[field.key]) ? secret[field.key] : []}
          onChange={(val) => onChange(field.key, val)}
          placeholder={field.placeholder || "Add tag…"}
        />
      </Field>
    );
  }

  // text, password, number
  const inputType = ft === "password" ? "password" : ft === "number" ? "number" : "text";

  if (field.generate) {
    return (
      <Field label={field.label} description={field.help} className={className}>
        <div className="flex gap-1.5">
          <Input
            type={inputType}
            value={value}
            onChange={(e) => onChange(field.key, e.target.value)}
            onInput={(e: any) => onChange(field.key, e.target.value)}
            placeholder={field.placeholder || ""}
            className="flex-1"
          />
          <Button type="button" variant="outline" size="sm" onClick={() => onChange(field.key, generateHex(32))}>Generate</Button>
        </div>
      </Field>
    );
  }

  return (
    <Field label={field.label} description={field.help} className={className}>
      <Input
        type={inputType}
        value={value}
        onChange={(e) => onChange(field.key, e.target.value)}
        onInput={(e: any) => onChange(field.key, e.target.value)}
        placeholder={field.placeholder || ""}
      />
    </Field>
  );
}

// ── SecretFields ──────────────────────────────────────────────────────────────

function SecretFields({ kind, secret, onChange, credentialTypes }: { kind: string; secret: Record<string, any>; onChange: (key: string, val: any) => void; credentialTypes: any[] }) {
  const s = (key: string, fallback = "") => secret[key] ?? fallback;

  // ─── OAuth2: custom UI with status indicator + authorize flow ───
  if (kind === "oauth2") {
    const statusLabel = s("refresh_token") ? (Number(s("expires_at", "0")) * 1000 > Date.now() ? "Authorized" : "Token Expired") : "Not Configured";
    const statusColor = s("refresh_token") ? (Number(s("expires_at", "0")) * 1000 > Date.now() ? "bg-green-500" : "bg-amber-500") : "bg-zinc-400";
    return (
      <div className="flex flex-col gap-4">
        <div className="rounded-md border border-ui-border bg-surface-1 px-3 py-3">
          <p className="text-sm font-medium text-body">OAuth2 Authorization Code Grant</p>
          <p className="mt-1 text-xs leading-relaxed text-body-soft">
            Configure the OAuth2 provider's client credentials and endpoints.
            After saving, use the <strong>Authorize</strong> button to complete the consent flow.
          </p>
        </div>

        <div className="flex items-center gap-2 px-1">
          <span className={cx("inline-block w-2 h-2 rounded-full", statusColor)} />
          <span className="text-xs text-body-soft">{statusLabel}</span>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="Provider" description="Label for this provider (e.g. google, microsoft, slack).">
            <Input value={s("provider")} onInput={(e: any) => onChange("provider", e.target.value)} placeholder="google" />
          </Field>
          <Field label="Token Type">
            <Input value={s("token_type", "Bearer")} onInput={(e: any) => onChange("token_type", e.target.value)} placeholder="Bearer" />
          </Field>
          <Field label="Client ID" className="col-span-2">
            <Input value={s("client_id")} onInput={(e: any) => onChange("client_id", e.target.value)} placeholder="xxx.apps.googleusercontent.com" />
          </Field>
          <Field label="Client Secret" className="col-span-2">
            <Input type="password" value={s("client_secret")} onInput={(e: any) => onChange("client_secret", e.target.value)} />
          </Field>
          <Field label="Authorize URL" className="col-span-2" description="Provider's authorization endpoint.">
            <Input value={s("authorize_url")} onInput={(e: any) => onChange("authorize_url", e.target.value)} placeholder="https://accounts.google.com/o/oauth2/v2/auth" />
          </Field>
          <Field label="Token URL" className="col-span-2" description="Provider's token exchange endpoint.">
            <Input value={s("token_url")} onInput={(e: any) => onChange("token_url", e.target.value)} placeholder="https://oauth2.googleapis.com/token" />
          </Field>
          <Field label="Scopes" className="col-span-2" description="Space-separated scopes to request.">
            <Input value={s("scopes")} onInput={(e: any) => onChange("scopes", e.target.value)} placeholder="openid email profile" />
          </Field>
          <Field label="Callback URL" className="col-span-2" description="Register this URL with your OAuth2 provider. Must match exactly what you configured in their dashboard.">
            <div className="flex gap-1.5">
              <Input value={s("redirect_uri") || `${window.location.origin}/oauth/callback`} onInput={(e: any) => onChange("redirect_uri", e.target.value)} placeholder={`${window.location.origin}/oauth/callback`} className="flex-1" />
              <Button type="button" variant="outline" size="sm" onClick={() => { try { navigator.clipboard.writeText(s("redirect_uri") || `${window.location.origin}/oauth/callback`); } catch {} }}>Copy</Button>
            </div>
          </Field>
        </div>
      </div>
    );
  }

  // ─── HMAC: custom UI with provider preset auto-fill ───
  if (kind === "hmac") {
    const HMAC_PROVIDERS = [
      { value: "generic", label: "Generic" },
      { value: "github", label: "GitHub" },
      { value: "stripe", label: "Stripe" },
      { value: "shopify", label: "Shopify" },
      { value: "slack", label: "Slack" },
    ];
    const HMAC_PRESETS: Record<string, Record<string, any>> = {
      generic:  { signature_header: "X-Signature", signature_encoding: "hex", signature_prefix: "", algorithm: "sha256", replay_tolerance: 0 },
      github:   { signature_header: "X-Hub-Signature-256", signature_encoding: "hex", signature_prefix: "sha256=", algorithm: "sha256", replay_tolerance: 0 },
      stripe:   { signature_header: "Stripe-Signature", signature_encoding: "hex", signature_prefix: "", algorithm: "sha256", replay_tolerance: 300 },
      shopify:  { signature_header: "X-Shopify-Hmac-SHA256", signature_encoding: "base64", signature_prefix: "", algorithm: "sha256", replay_tolerance: 0 },
      slack:    { signature_header: "X-Slack-Signature", signature_encoding: "hex", signature_prefix: "v0=", algorithm: "sha256", replay_tolerance: 300 },
    };
    const HMAC_ENCODINGS = [
      { value: "hex", label: "Hex" },
      { value: "base64", label: "Base64" },
    ];
    const HMAC_ALGORITHMS = [
      { value: "sha256", label: "SHA-256" },
      { value: "sha1", label: "SHA-1 (legacy)" },
    ];

    function applyPreset(provider: string) {
      const preset = HMAC_PRESETS[provider] || HMAC_PRESETS.generic;
      onChange("__json__", { ...secret, provider, ...preset });
    }

    return (
      <div className="flex flex-col gap-4">
        <div className="rounded-md border border-ui-border bg-surface-1 px-3 py-3">
          <p className="text-sm font-medium text-body">Webhook HMAC Verification</p>
          <p className="mt-1 text-xs leading-relaxed text-body-soft">
            Verify inbound webhook signatures from third-party services.
            Select a provider to auto-fill the verification settings, or use Generic and configure manually.
          </p>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="Provider" description="Auto-fills verification settings.">
            <Select value={s("provider", "generic")} onChange={(e) => applyPreset(e.target.value)}>
              {HMAC_PROVIDERS.map((p) => <SelectOption key={p.value} value={p.value} label={p.label} />)}
            </Select>
          </Field>
          <Field label="Algorithm">
            <Select value={s("algorithm", "sha256")} onChange={(e) => onChange("algorithm", e.target.value)}>
              {HMAC_ALGORITHMS.map((a) => <SelectOption key={a.value} value={a.value} label={a.label} />)}
            </Select>
          </Field>
          <Field label="Signing Secret" className="col-span-2" description="The shared secret from the provider's webhook settings.">
            <Input type="password" value={s("secret")} onInput={(e: any) => onChange("secret", e.target.value)} placeholder="whsec_..." />
          </Field>
          <Field label="Signature Header" description="HTTP header containing the signature.">
            <Input value={s("signature_header")} onInput={(e: any) => onChange("signature_header", e.target.value)} placeholder="X-Hub-Signature-256" />
          </Field>
          <Field label="Encoding">
            <Select value={s("signature_encoding", "hex")} onChange={(e) => onChange("signature_encoding", e.target.value)}>
              {HMAC_ENCODINGS.map((enc) => <SelectOption key={enc.value} value={enc.value} label={enc.label} />)}
            </Select>
          </Field>
          <Field label="Signature Prefix" description="Strip this prefix from the header value before comparing (e.g. sha256=).">
            <Input value={s("signature_prefix")} onInput={(e: any) => onChange("signature_prefix", e.target.value)} placeholder="sha256=" />
          </Field>
          <Field label="Replay Tolerance (seconds)" description="Reject requests older than this. 0 = disabled. Stripe/Slack use 300.">
            <Input type="number" value={s("replay_tolerance", "0")} onInput={(e: any) => onChange("replay_tolerance", Number(e.target.value) || 0)} placeholder="0" />
          </Field>
        </div>
      </div>
    );
  }

  // ─── Secure Request: custom UI with KV editors + variables ───
  if (kind === "secure_request") {
    const request = secret.request && typeof secret.request === "object" ? secret.request : {};
    const variables = Array.isArray(secret.variables) ? secret.variables : [];
    const secrets = secret.secrets && typeof secret.secrets === "object" ? secret.secrets : {};
    const egress = secret.egress && typeof secret.egress === "object" ? secret.egress : {};
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
    const updateEgress = (patch: Record<string, any>) => onChange("__json__", {
      ...secret,
      egress: {
        ...egress,
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

        <div className="rounded-md border border-ui-border bg-surface-1 px-3 py-3">
          <label className="inline-flex items-center gap-2 text-sm font-medium text-body">
            <input
              type="checkbox"
              checked={!!egress.allow_private}
              onChange={(e: any) => updateEgress({ allow_private: !!e.target.checked })}
            />
            Allow private/internal egress
          </label>
          <div className="mt-3 grid grid-cols-2 gap-3">
            <Field label="Allowed Hosts" className="col-span-2" description="Exact hosts allowed when private egress is enabled. One per line.">
              <textarea
                value={listToText(egress.allowed_hosts)}
                onChange={(e) => updateEgress({ allowed_hosts: textToList(e.target.value) })}
                onInput={(e: any) => updateEgress({ allowed_hosts: textToList(e.target.value) })}
                rows={3}
                placeholder={"goveyes-qwen3-embedding-8b\ngoveyes-qwen3-embedding-8b.main-app-cluster.svc.cluster.local"}
                className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
              />
            </Field>
            <Field label="Allowed Paths" description="Optional exact URL paths. Blank allows the template path.">
              <textarea
                value={listToText(egress.allowed_paths)}
                onChange={(e) => updateEgress({ allowed_paths: textToList(e.target.value) })}
                onInput={(e: any) => updateEgress({ allowed_paths: textToList(e.target.value) })}
                rows={3}
                placeholder="/embed"
                className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
              />
            </Field>
            <Field label="Allowed Methods" description="Optional methods. Blank allows the template method.">
              <textarea
                value={listToText(egress.allowed_methods)}
                onChange={(e) => updateEgress({ allowed_methods: textToList(e.target.value).map((item) => item.toUpperCase()) })}
                onInput={(e: any) => updateEgress({ allowed_methods: textToList(e.target.value).map((item) => item.toUpperCase()) })}
                rows={3}
                placeholder="POST"
                className="flex w-full rounded-md border border-ui-border bg-ui-bg text-ui-text px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand-blue/40"
              />
            </Field>
          </div>
        </div>

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

  // ─── Custom: raw JSON editor ───
  if (kind === "custom") {
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

  // ─── Dynamic rendering from credential-types API ───
  const typeDef = credentialTypes.find((t) => t.kind === kind);
  if (!typeDef || !typeDef.fields?.length) {
    // Unknown type — fallback to raw JSON editor
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

  return (
    <div className="grid grid-cols-2 gap-3">
      {typeDef.fields.map((field) => (
        <DynamicField key={field.key} field={field} secret={secret} onChange={onChange} />
      ))}
    </div>
  );
}

// ── CredentialDialog ──────────────────────────────────────────────────────────

function CredentialDialog({ open, onClose, mode, editItem, apiList, apiItemBase, onSaved, credentialTypes }) {
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
      setNotes(""); setSecret(defaultSecretForKind("postgres", credentialTypes)); setStatus("Fill fields and save."); setStatusTone("info");
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
              <Select value={kind} onChange={(e) => { const nextKind = e.target.value; setKind(nextKind); setSecret(defaultSecretForKind(nextKind, credentialTypes)); }} disabled={busy}>
                {credentialTypes.length > 0
                  ? credentialTypes.map((t) => <SelectOption key={t.kind} value={t.kind} label={t.title} />)
                  : FALLBACK_KINDS.map((k) => <SelectOption key={k} value={k} label={k} />)
                }
              </Select>
            </Field>
            <Field label="Title" className="col-span-2">
              <Input value={title} onChange={(e) => setTitle(e.target.value)} onInput={(e: any) => setTitle(e.target.value)} placeholder="Main Postgres" required disabled={busy} />
            </Field>
          </div>

          {/* Dynamic secret fields */}
          <SecretFields kind={kind} secret={secret} onChange={setSecretField} credentialTypes={credentialTypes} />

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
            {mode === "edit" && kind === "oauth2" && (
              <Button type="button" variant="outline" size="sm" disabled={busy} onClick={async () => {
                setBusy(true); setStatus("Redirecting to provider…"); setStatusTone("info");
                try {
                  const data = await requestJson(`${apiItemBase}/${encodeURIComponent(credentialId)}/oauth/authorize`);
                  if (data?.redirect_url) { window.location.href = data.redirect_url; }
                  else { setStatus("No redirect URL returned."); setStatusTone("error"); setBusy(false); }
                } catch (err: any) {
                  setStatus(`Authorize failed: ${err?.message || err}`); setStatusTone("error"); setBusy(false);
                }
              }}>Authorize</Button>
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
  const [credentialTypes, setCredentialTypes] = useState<any[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<"create" | "edit">("create");
  const [editItem, setEditItem] = useState<any>(null);

  async function loadList() {
    try {
      const payload = await requestJson(apiList);
      setItems(Array.isArray(payload?.items) ? payload.items : []);
    } catch {}
  }

  useEffect(() => {
    loadList();
    // Fetch credential type definitions from API
    requestJson(`/api/projects/${input.owner}/${input.project}/credential-types`)
      .then((data) => { if (data?.types) setCredentialTypes(data.types); })
      .catch(() => {});
    // Handle OAuth callback redirect params
    const params = new URLSearchParams(window.location.search);
    const oauthResult = params.get("oauth");
    if (oauthResult) {
      // Clean URL
      const url = new URL(window.location.href);
      url.searchParams.delete("oauth");
      window.history.replaceState({}, "", url.pathname + url.search);
    }
  }, []);

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
                        <td className="px-3 py-2 text-sm text-body-soft">
                          {item.kind === "oauth2" && item.oauth2_status ? (
                            <span className="inline-flex items-center gap-1.5">
                              <span className={cx("inline-block w-1.5 h-1.5 rounded-full",
                                item.oauth2_status === "authorized" ? "bg-green-500" :
                                item.oauth2_status === "expired" ? "bg-amber-500" : "bg-zinc-400"
                              )} />
                              {item.oauth2_status}
                            </span>
                          ) : item.has_secret ? "yes" : "no"}
                        </td>
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
        credentialTypes={credentialTypes}
      />
    </ProjectStudioShell>
  );
}
