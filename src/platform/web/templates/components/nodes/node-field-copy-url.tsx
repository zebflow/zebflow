import { useState } from "zeb";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

export default function NodeFieldCopyUrl({ field, value }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    const v = String(value || "");
    if (!v) return;
    try {
      await navigator.clipboard.writeText(v);
      setCopied(true);
      setTimeout(() => setCopied(false), 900);
    } catch {
      // Fallback: select input
    }
  }

  return (
    <Field label={field.label} description={field.help}>
      <div className="flex gap-2">
        <Input readOnly value={String(value || "")} className="flex-1 font-mono text-xs" />
        <Button type="button" variant="outline" size="sm" onClick={handleCopy}>
          {copied ? "Copied!" : "Copy"}
        </Button>
      </div>
    </Field>
  );
}
