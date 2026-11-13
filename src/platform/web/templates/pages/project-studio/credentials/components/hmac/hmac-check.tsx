import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";
import Button from "@/components/ui/button";
import CheckboxField from "@/components/ui/checkbox-field";

/**
 * Paste one delivery from the vendor's dashboard and see whether the saved
 * check passes it, and if not, which step fails. Nothing is stored, and the
 * answer never shows the secret or a signature. Works on a saved credential,
 * so save first after changing a setting.
 */
export default function HmacCheck({ checkUrl }) {
  const [headers, setHeaders] = useState("");
  const [body, setBody] = useState("");
  const [url, setUrl] = useState("");
  const [ignoreTime, setIgnoreTime] = useState(true);
  const [result, setResult] = useState<any>(null);
  const [busy, setBusy] = useState(false);

  if (!checkUrl) {
    return <p className="text-xs text-muted-foreground">Save the credential to test it with a delivery from the vendor.</p>;
  }

  async function run() {
    setBusy(true);
    setResult(null);
    try {
      const answer = await requestJson(checkUrl, {
        method: "POST",
        body: JSON.stringify({ headers, body, url: url || undefined, ignore_time: ignoreTime }),
      });
      setResult(answer);
    } catch (err: any) {
      setResult({ passed: false, code: err?.code || "ERROR", message: err?.message || String(err) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-3 rounded-md border border-border px-3 py-3">
      <p className="text-sm font-medium text-foreground">Test with a delivery</p>
      <Field label="Headers" description="One per line, Name: value, as the vendor's dashboard shows them.">
        <Textarea className="font-mono" rows={4} value={headers} onInput={(e: any) => setHeaders(e.target.value)} placeholder={"X-Vendor-Timestamp: 1790000000\nX-Vendor-Signature: v1=…"} />
      </Field>
      <Field label="Body" description="Exactly as delivered: spaces and line breaks count.">
        <Textarea className="font-mono" rows={4} value={body} onInput={(e: any) => setBody(e.target.value)} />
      </Field>
      <Field label="Address called" description="Only needed when the signed text uses {url}.">
        <Input className="font-mono" value={url} onInput={(e: any) => setUrl(e.target.value)} placeholder="https://example.com/hooks/vendor" />
      </Field>
      <CheckboxField
        label="Ignore the time window"
        description="A pasted delivery is older than any tolerance. The timestamp is still read and still part of the signed text."
        checked={ignoreTime}
        onChange={(e: any) => setIgnoreTime(!!e.target.checked)}
      />
      <div className="flex items-center gap-3">
        <Button type="button" size="sm" variant="outline" onClick={run} disabled={busy || !headers.trim()}>Check</Button>
        {result ? (
          <span className={result.passed ? "text-xs text-success" : "text-xs text-destructive"}>
            {result.passed ? "Passes: this delivery matches the saved check." : `${result.code}: ${result.message}`}
          </span>
        ) : null}
      </div>
    </div>
  );
}
