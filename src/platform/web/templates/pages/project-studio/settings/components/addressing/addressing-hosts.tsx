import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { checkSummary, copyText } from "@/pages/project-studio/settings/components/addressing/addressing-lib";

/**
 * "Your site" and "Production hosts": the dev host every project has, and the
 * domains the operator adds. Each custom host carries its DNS + verify status
 * and the record to paste when it is wrong. The panel above owns the list;
 * this only renders it and asks for changes.
 */
export default function AddressingHosts({ data, hosts, checks, checking, onAdd, onRemove, onCheck, busy }) {
  const [draft, setDraft] = useState("");
  const [copied, setCopied] = useState("");

  async function copy(text) {
    if (await copyText(text)) {
      setCopied(text);
      setTimeout(() => setCopied(""), 1200);
    }
  }

  function submit(e) {
    e.preventDefault();
    const host = draft.trim().toLowerCase();
    if (!host) return;
    onAdd(host);
    setDraft("");
  }

  const devUrl = String(data?.dev_url ?? "");

  return (
    <div className="flex flex-col gap-6">
      <section>
        <h4 className="text-[0.8rem] font-semibold text-foreground">Your site</h4>
        <div className="mt-2 flex flex-wrap items-center gap-2">
          <code className="rounded border border-border bg-muted px-2 py-1 font-mono text-[0.8rem] text-foreground">{devUrl}</code>
          <a href={devUrl} target="_blank" rel="noreferrer" className="text-[0.78rem] text-info underline-offset-4 hover:underline">Open ↗</a>
          <Button type="button" size="sm" variant="ghost" onClick={() => copy(devUrl)}>{copied === devUrl ? "Copied" : "Copy"}</Button>
        </div>
        <p className="mt-2 text-[0.78rem] leading-[1.45] text-muted-foreground">
          Works in every browser on this machine with no setup. Links written as <code>/book</code> land here,
          the agent verifies pages here, and the Studio&apos;s Open buttons use it.
        </p>
      </section>

      <section>
        <h4 className="text-[0.8rem] font-semibold text-foreground">Production hosts</h4>
        <p className="mt-1 text-[0.78rem] leading-[1.45] text-muted-foreground">
          Deploying to production means a web server (or a tunnel) in front of Zebflow sends traffic for your
          domain to this instance. You configure that server once; nothing in the project changes. Add the
          domain, then copy the matching server configuration below.
        </p>

        {hosts.length === 0 ? (
          <p className="mt-3 text-[0.78rem] text-muted-foreground">No production host yet — add one to generate the server configuration.</p>
        ) : (
          <ul className="mt-3 flex flex-col divide-y divide-border border border-border">
            {hosts.map((host) => {
              const summary = checkSummary(checks?.[host]);
              const tone = summary.tone === "ok" ? "text-success" : summary.tone === "error" ? "text-warning" : "text-muted-foreground";
              const record = checks?.[host] && !checks[host]?.dns?.ok ? `A  ${host}  <this instance's public IP>` : "";
              return (
                <li key={host} className="flex flex-wrap items-center gap-3 px-3 py-2">
                  <code className="font-mono text-[0.8rem] text-foreground">{host}</code>
                  <span className={`text-[0.76rem] ${tone}`}>{checking === host ? "checking…" : summary.text}</span>
                  {record ? (
                    <Button type="button" size="sm" variant="ghost" onClick={() => copy(record)}>
                      {copied === record ? "Copied" : "Copy DNS record"}
                    </Button>
                  ) : null}
                  <span className="ml-auto flex items-center gap-1">
                    <Button type="button" size="sm" variant="outline" onClick={() => onCheck(host)} disabled={busy || checking === host}>Verify</Button>
                    <a href={`https://${host}/`} target="_blank" rel="noreferrer" className="px-2 text-[0.78rem] text-info underline-offset-4 hover:underline">Open ↗</a>
                    <Button type="button" size="sm" variant="ghost" onClick={() => onRemove(host)} disabled={busy}>Remove</Button>
                  </span>
                </li>
              );
            })}
          </ul>
        )}

        <form onSubmit={submit} className="mt-3 flex flex-wrap items-center gap-2">
          <Input
            id="addressing-new-host"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="northside.example"
            className="w-[18rem] font-mono"
          />
          <Button type="submit" size="sm" variant="outline" disabled={busy || !draft.trim()}>Add host</Button>
          <span className="text-[0.74rem] text-muted-foreground">A hostname only — no https://, no port, no path.</span>
        </form>
      </section>
    </div>
  );
}
