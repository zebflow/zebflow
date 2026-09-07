import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Dialog from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogDescription from "@/components/ui/dialog-description";
import DialogFooter from "@/components/ui/dialog-footer";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import Input from "@/components/ui/input";
import Textarea from "@/components/ui/textarea";

function scriptSlug(value: string) {
  return String(value || "custom-script")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "custom-script";
}

export function buildCustomScriptPrompt({ name, need, inputExample, outputExample }: any) {
  const slug = scriptSlug(name);
  const purpose = String(need || "").trim() || "Describe the requested behavior here.";
  const input = String(inputExample || "").trim() || "No example supplied.";
  const output = String(outputExample || "").trim() || "No example supplied.";

  return `Create one reusable TypeScript module for Zebflow RWE.

Target file: scripts/${slug}.ts

What it must do:
${purpose}

Input example:
${input}

Expected output example:
${output}

Zebflow rules:
1. Return only the complete TypeScript source. Do not wrap it in Markdown.
2. Export clear camelCase functions and values. Never export UPPER_SNAKE_CASE names.
3. Do not import npm, JSR, CDN, React, Preact, or Node packages.
4. Use standard TypeScript and Web APIs. Use local project imports through @/ only when necessary.
5. Do not call render(). Do not create a second application root.
6. Do not embed secrets, credentials, tokens, or private URLs.
7. Avoid hidden network calls, eval, new Function, and global side effects.
8. Validate inputs and return useful errors instead of silently changing bad data.
9. Keep the public input and output shapes aligned with the examples.
10. Keep the module focused and easy to test.`;
}

export default function ScriptPromptDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [name, setName] = useState("custom-script");
  const [need, setNeed] = useState("");
  const [inputExample, setInputExample] = useState("");
  const [outputExample, setOutputExample] = useState("");
  const [generated, setGenerated] = useState("");
  const [copyState, setCopyState] = useState("");

  function generate() {
    setGenerated(buildCustomScriptPrompt({ name, need, inputExample, outputExample }));
    setCopyState("");
  }

  async function copyPrompt() {
    if (!generated) return;
    try {
      await navigator.clipboard.writeText(generated);
      setCopyState("Copied");
    } catch (_) {
      setCopyState("Copy failed. Select the prompt text manually.");
    }
  }

  return (
    <Dialog open={open} onOpenChange={(value) => { if (!value) onClose(); }}>
      <DialogContent size="lg" className="border-border bg-surface text-body">
        <DialogHeader className="shrink-0">
          <DialogTitle>Create Script Prompt</DialogTitle>
          <DialogDescription>
            Describe a reusable TypeScript module, then paste the generated prompt into your preferred coding assistant.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 px-6 py-5">
          <label className="grid gap-1.5 text-xs text-body-soft">
            <span>Script name</span>
            <Input value={name} onInput={(event) => setName(event.currentTarget.value)} placeholder="format-address" />
          </label>
          <label className="grid gap-1.5 text-xs text-body-soft">
            <span>What should it do?</span>
            <Textarea value={need} onInput={(event) => setNeed(event.currentTarget.value)} rows={4} placeholder="Normalize an address and return a stable display label." />
          </label>
          <div className="grid gap-4 md:grid-cols-2">
            <label className="grid min-w-0 gap-1.5 text-xs text-body-soft">
              <span>Input example</span>
              <Textarea value={inputExample} onInput={(event) => setInputExample(event.currentTarget.value)} rows={6} className="font-mono text-xs" placeholder={'{"street":"1 Swanston St","city":"Melbourne"}'} />
            </label>
            <label className="grid min-w-0 gap-1.5 text-xs text-body-soft">
              <span>Output example</span>
              <Textarea value={outputExample} onInput={(event) => setOutputExample(event.currentTarget.value)} rows={6} className="font-mono text-xs" placeholder={'{"label":"1 Swanston St, Melbourne"}'} />
            </label>
          </div>

          {generated ? (
            <label className="grid gap-1.5 text-xs text-body-soft">
              <span>Generated prompt</span>
              <Textarea value={generated} readOnly rows={14} className="font-mono text-xs" />
            </label>
          ) : null}
          {copyState ? <p className="m-0 text-xs text-body-soft">{copyState}</p> : null}
        </div>

        <DialogFooter className="shrink-0">
          <Button type="button" size="sm" variant="outline" onClick={onClose}>Close</Button>
          <Button type="button" size="sm" variant="outline" disabled={!generated} onClick={copyPrompt}>Copy Prompt</Button>
          <Button type="button" size="sm" onClick={generate}>Generate Prompt</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
