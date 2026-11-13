import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

const ENCODINGS = [
  { value: "text", label: "Text, as shown by the vendor" },
  { value: "base64", label: "Base64 (decoded before use)" },
  { value: "hex", label: "Hex (decoded before use)" },
];

/**
 * The shared secrets. `previous` keeps the old secret working while the
 * vendor moves to a new one; a delivery passes when it matches either.
 */
export default function HmacSecrets({ value, onChange }) {
  const secrets = value.secrets && typeof value.secrets === "object" ? value.secrets : {};
  const setSecret = (slot, text) => onChange({ secrets: { ...secrets, [slot]: text } });

  return (
    <div className="grid grid-cols-2 gap-3">
      <Field label="Secret" className="col-span-2" description="The signing secret from the vendor's webhook settings.">
        <Input type="password" value={secrets.current || ""} onInput={(e: any) => setSecret("current", e.target.value)} />
      </Field>
      <Field
        label="Previous secret"
        className="col-span-2"
        description="Optional. While the vendor switches to a new secret, put the old one here so deliveries signed with either pass. Clear it when the switch is done."
      >
        <Input type="password" value={secrets.previous || ""} onInput={(e: any) => setSecret("previous", e.target.value)} />
      </Field>
      <Field label="Secret format" description="How the vendor's secret is written. Most vendors use the text as it is.">
        <Select value={value.secret_encoding || "text"} onChange={(e) => onChange({ secret_encoding: e.target.value })}>
          {ENCODINGS.map((enc) => <SelectOption key={enc.value} value={enc.value} label={enc.label} />)}
        </Select>
      </Field>
      <Field label="Secret prefix" description="Removed from the secret before it is used, for example whsec_.">
        <Input value={value.secret_prefix || ""} onInput={(e: any) => onChange({ secret_prefix: e.target.value })} placeholder="whsec_" />
      </Field>
    </div>
  );
}
