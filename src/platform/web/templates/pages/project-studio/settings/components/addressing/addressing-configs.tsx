import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import { copyText } from "@/pages/project-studio/settings/components/addressing/addressing-lib";

/**
 * The copyable output of the Addressing section: one generated configuration
 * per web server, rendered from the hosts above, each ending in the command
 * that proves it. Pick the one you run; nothing here is typed by hand.
 */
export default function AddressingConfigs({ configs }) {
  const list = Array.isArray(configs) ? configs : [];
  const [active, setActive] = useState(list[0]?.id ?? "");
  const [copied, setCopied] = useState(false);
  const current = list.find((c) => c.id === active) ?? list[0];

  async function copy() {
    if (!current) return;
    if (await copyText(current.text)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    }
  }

  return (
    <section>
      <h4 className="text-[0.8rem] font-semibold text-foreground">Web server configuration</h4>
      <p className="mt-1 text-[0.78rem] leading-[1.45] text-muted-foreground">
        Generated from the hosts above. Copy the one you run; each passes the Host header (Zebflow routes by it),
        keeps WebSocket upgrades, allows this instance&apos;s upload size, and ends with the check that proves it works.
      </p>
      {list.length === 0 ? (
        <p className="mt-3 text-[0.78rem] text-muted-foreground">Add a production host and the configurations appear here.</p>
      ) : (
        <>
          <div className="mt-3 flex flex-wrap gap-1">
            {list.map((c) => (
              <Button
                key={c.id}
                type="button"
                size="sm"
                variant={c.id === (current?.id ?? "") ? "secondary" : "outline"}
                onClick={() => setActive(c.id)}
              >
                {c.title}
              </Button>
            ))}
          </div>
          {current ? (
            <div className="mt-3 border border-border">
              <div className="flex items-center justify-between gap-3 border-b border-border bg-muted px-3 py-1.5">
                <span className="text-[0.74rem] font-medium text-muted-foreground">{current.title}</span>
                <Button type="button" size="sm" variant="ghost" onClick={copy}>{copied ? "Copied" : "Copy all"}</Button>
              </div>
              <pre className="max-h-[28rem] overflow-auto p-3 font-mono text-[0.74rem] leading-[1.5] text-foreground">{current.text}</pre>
            </div>
          ) : null}
        </>
      )}
    </section>
  );
}
