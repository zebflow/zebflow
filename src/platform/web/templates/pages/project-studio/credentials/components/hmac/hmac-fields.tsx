import { useState, useEffect } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import HmacSecrets from "@/pages/project-studio/credentials/components/hmac/hmac-secrets";
import HmacSources from "@/pages/project-studio/credentials/components/hmac/hmac-sources";
import HmacCheck from "@/pages/project-studio/credentials/components/hmac/hmac-check";

const ALGORITHMS = [
  { value: "sha256", label: "SHA-256" },
  { value: "sha512", label: "SHA-512" },
  { value: "sha1", label: "SHA-1 (old vendors)" },
];

/**
 * The hmac credential: a whole webhook signature check. A preset fills the
 * form and every field stays editable; the saved credential keeps its own
 * copy (`services::webhook_signature`). The secrets keep their masks until
 * they are typed over, so saving without retyping keeps them.
 */
export default function HmacFields({ secret, onChange, apiItemBase, credentialId }) {
  const [presets, setPresets] = useState<any[]>([]);
  const projectApi = String(apiItemBase || "").replace(/\/credentials\/?$/, "");

  useEffect(() => {
    if (!projectApi) return;
    requestJson(`${projectApi}/credential-types/hmac/presets`)
      .then((payload) => setPresets(Array.isArray(payload?.presets) ? payload.presets : []))
      .catch(() => setPresets([]));
  }, [projectApi]);

  const value = secret && typeof secret === "object" ? secret : {};
  const chosen = presets.find((p) => p.id === value.preset);
  const set = (patch) => onChange({ ...value, ...patch });

  function choosePreset(id) {
    const preset = presets.find((p) => p.id === id);
    const keep = { secrets: value.secrets || { current: "" } };
    if (!preset) {
      onChange({ ...keep, preset: "", signed: value.signed || "{body}", signature: value.signature || { from: "header:X-Signature" } });
      return;
    }
    onChange({ ...keep, ...preset.settings, preset: preset.id });
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-md border border-border bg-surface-1 px-3 py-3">
        <p className="text-sm font-medium text-foreground">Webhook signature check</p>
        <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
          A vendor signs each delivery with a shared secret. Say what it signs and where it puts the signature, and
          Zebflow refuses any delivery that does not match before a node runs. Pick the vendor if it is listed;
          otherwise choose Custom and copy the settings from the vendor's documentation.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <Field label="Vendor" description="Fills the settings below. You can change any of them afterwards.">
          <Select value={value.preset || ""} onChange={(e) => choosePreset(e.target.value)}>
            <SelectOption value="" label="Custom" />
            {presets.map((p) => <SelectOption key={p.id} value={p.id} label={p.title} />)}
          </Select>
        </Field>
        <Field label="Algorithm">
          <Select value={value.algorithm || "sha256"} onChange={(e) => set({ algorithm: e.target.value })}>
            {ALGORITHMS.map((a) => <SelectOption key={a.value} value={a.value} label={a.label} />)}
          </Select>
        </Field>
        {chosen?.note ? <p className="col-span-2 text-xs leading-relaxed text-muted-foreground">{chosen.note}</p> : null}
      </div>

      <HmacSecrets value={value} onChange={set} />

      <Field
        label="Signed text"
        description="Exactly what the vendor signs. Pieces: {body} (the raw body), {timestamp}, {header:Name}, {part:key}, {method}, {path}, {url}. Anything else is literal text; write {{ or }} for a brace."
      >
        <Input
          className="font-mono"
          value={value.signed || ""}
          onInput={(e: any) => set({ signed: e.target.value })}
          placeholder="{timestamp}.{body}"
        />
      </Field>

      <HmacSources value={value} onChange={set} />

      <HmacCheck checkUrl={credentialId ? `${apiItemBase}/${encodeURIComponent(credentialId)}/hmac/check` : ""} />
    </div>
  );
}
