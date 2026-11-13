import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import CheckboxField from "@/components/ui/checkbox-field";
import { Select, SelectOption } from "@/components/ui/select";

/**
 * Where the signature, the timestamp and any key=value parts come from. A
 * place is written `header:Name`, or `part:key` for one piece of the header
 * the parts section names (Stripe's `t=…,v1=…`).
 */
export default function HmacSources({ value, onChange }) {
  const signature = value.signature && typeof value.signature === "object" ? value.signature : {};
  const timestamp = value.timestamp && typeof value.timestamp === "object" ? value.timestamp : null;
  const parts = value.parts && typeof value.parts === "object" ? value.parts : null;
  const setSignature = (patch) => onChange({ signature: { ...signature, ...patch } });
  const setTimestamp = (patch) => onChange({ timestamp: { ...(timestamp || {}), ...patch } });
  const setParts = (patch) => onChange({ parts: { ...(parts || {}), ...patch } });

  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-2 gap-3">
        <Field label="Signature from" description="header:Name, or part:key when the signature is one piece of a header.">
          <Input className="font-mono" value={signature.from || ""} onInput={(e: any) => setSignature({ from: e.target.value })} placeholder="header:X-Vendor-Signature" />
        </Field>
        <Field label="Signature prefix" description="Text before the signature, such as sha256= or v1=. A delivery without it is refused by name.">
          <Input className="font-mono" value={signature.prefix || ""} onInput={(e: any) => setSignature({ prefix: e.target.value })} placeholder="v1=" />
        </Field>
        <Field label="Signature encoding">
          <Select value={signature.encoding || "hex"} onChange={(e) => setSignature({ encoding: e.target.value })}>
            <SelectOption value="hex" label="Hex" />
            <SelectOption value="base64" label="Base64" />
          </Select>
        </Field>
        <Field label="Several signatures, separated by" description="Leave empty for one signature. A space for the Standard Webhooks format.">
          <Input className="font-mono" value={signature.split || ""} onInput={(e: any) => setSignature({ split: e.target.value })} placeholder="(one signature)" />
        </Field>
      </div>

      <CheckboxField
        label="The vendor sends a timestamp"
        description="Refuses a delivery older than the tolerance, so a copied delivery cannot be replayed later."
        checked={!!timestamp}
        onChange={(e: any) => onChange({ timestamp: e.target.checked ? { from: "", unit: "seconds", tolerance: 300 } : null })}
      />
      {timestamp ? (
        <div className="grid grid-cols-3 gap-3">
          <Field label="Timestamp from" description="header:Name or part:key.">
            <Input className="font-mono" value={timestamp.from || ""} onInput={(e: any) => setTimestamp({ from: e.target.value })} placeholder="header:X-Vendor-Timestamp" />
          </Field>
          <Field label="Unit">
            <Select value={timestamp.unit || "seconds"} onChange={(e) => setTimestamp({ unit: e.target.value })}>
              <SelectOption value="seconds" label="Seconds" />
              <SelectOption value="milliseconds" label="Milliseconds" />
            </Select>
          </Field>
          <Field label="Tolerance (seconds)" description="How far from now the timestamp may be. 300 is usual.">
            <Input type="number" value={String(timestamp.tolerance ?? 300)} onInput={(e: any) => setTimestamp({ tolerance: Number(e.target.value) || 0 })} />
          </Field>
        </div>
      ) : null}

      <CheckboxField
        label="A header carries key=value parts"
        description="For headers such as Stripe-Signature (t=…,v1=…) or Paddle-Signature (ts=…;h1=…). Then use part:key above."
        checked={!!parts}
        onChange={(e: any) => onChange({ parts: e.target.checked ? { from: "", separator: ",", assign: "=" } : null })}
      />
      {parts ? (
        <div className="grid grid-cols-3 gap-3">
          <Field label="Parts from" description="header:Name.">
            <Input className="font-mono" value={parts.from || ""} onInput={(e: any) => setParts({ from: e.target.value })} placeholder="header:Stripe-Signature" />
          </Field>
          <Field label="Between parts">
            <Input className="font-mono" value={parts.separator ?? ","} onInput={(e: any) => setParts({ separator: e.target.value })} />
          </Field>
          <Field label="Between key and value">
            <Input className="font-mono" value={parts.assign ?? "="} onInput={(e: any) => setParts({ assign: e.target.value })} />
          </Field>
        </div>
      ) : null}
    </div>
  );
}
