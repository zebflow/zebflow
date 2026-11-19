import { cx } from "zeb/react";
import { Select, SelectOption } from "@/components/ui/select";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import KeyValueEditor from "@/pages/project-studio/credentials/components/fields/key-value-editor";
import SecureRequestVariablesEditor from "@/pages/project-studio/credentials/components/fields/secure-request-variables";
import DynamicField from "@/pages/project-studio/credentials/components/fields/dynamic-field";
import { REQUEST_METHODS, listToText, textToList } from "@/pages/project-studio/credentials/components/credential-lib";

export default function SecretFields({ kind, secret, onChange, credentialTypes }: { kind: string; secret: Record<string, any>; onChange: (key: string, val: any) => void; credentialTypes: any[] }) {
  const s = (key: string, fallback = "") => secret[key] ?? fallback;

  // ─── OAuth2: custom UI with status indicator + authorize flow ───
  if (kind === "oauth2") {
    const statusLabel = s("refresh_token") ? (Number(s("expires_at", "0")) * 1000 > Date.now() ? "Authorized" : "Token Expired") : "Not Configured";
    const statusColor = s("refresh_token") ? (Number(s("expires_at", "0")) * 1000 > Date.now() ? "bg-green-500" : "bg-amber-500") : "bg-zinc-400";
    return (
      <div className="flex flex-col gap-4">
        <div className="rounded-md border border-border bg-surface-1 px-3 py-3">
          <p className="text-sm font-medium text-foreground">OAuth2 Authorization Code Grant</p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
            Configure the OAuth2 provider's client credentials and endpoints.
            After saving, use the <strong>Authorize</strong> button to complete the consent flow.
          </p>
        </div>

        <div className="flex items-center gap-2 px-1">
          <span className={cx("inline-block w-2 h-2 rounded-full", statusColor)} />
          <span className="text-xs text-muted-foreground">{statusLabel}</span>
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
        <div className="rounded-md border border-border bg-surface-1 px-3 py-3">
          <p className="text-sm font-medium text-foreground">Secure Request Profile</p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
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

        <Field label="Body Template" description="Optional raw request body template. Leave blank and the node's own --body is sent (or none).">
          <textarea
            value={requestBody}
            onChange={(e) => updateRequest({ body: e.target.value })}
            rows={5}
            placeholder='{"user_id":"<USER_ID>","programme":"<PROGRAMME_CODE>"}'
            className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
          />
        </Field>

        <div className="rounded-md border border-border bg-surface-1 px-3 py-3">
          <label className="inline-flex items-center gap-2 text-sm font-medium text-foreground">
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
                rows={3}
                placeholder={"internal-service\ninternal-service.namespace.svc.cluster.local"}
                className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
              />
            </Field>
            <Field label="Allowed Paths" description="Optional exact URL paths. Blank allows the template path.">
              <textarea
                value={listToText(egress.allowed_paths)}
                onChange={(e) => updateEgress({ allowed_paths: textToList(e.target.value) })}
                rows={3}
                placeholder="/embed"
                className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
              />
            </Field>
            <Field label="Allowed Methods" description="Optional methods. Blank allows the template method.">
              <textarea
                value={listToText(egress.allowed_methods)}
                onChange={(e) => updateEgress({ allowed_methods: textToList(e.target.value).map((item) => item.toUpperCase()) })}
                rows={3}
                placeholder="POST"
                className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
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
          rows={8}
          placeholder={'{\n  "key": "value"\n}'}
          className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
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
          rows={8}
          placeholder={'{\n  "key": "value"\n}'}
          className="flex w-full rounded-md border border-border bg-popover text-foreground px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/40"
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
