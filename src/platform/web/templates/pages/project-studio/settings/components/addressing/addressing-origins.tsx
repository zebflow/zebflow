import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";

/**
 * Other sites whose pages may open this project's room sockets
 * (`docs/contracts/rooms.md`, Who may connect). A page on the project's own
 * hosts needs no entry; a browser page anywhere else is refused unless its
 * origin is listed here. The panel above owns the list and saves it.
 */
export default function AddressingOrigins({ config, onChange, busy }) {
  const [draft, setDraft] = useState("");
  const origins = Array.isArray(config?.origins) ? config.origins : [];

  function submit(e) {
    e.preventDefault();
    const origin = draft.trim().toLowerCase().replace(/\/+$/, "");
    if (!origin || origins.includes(origin)) return;
    onChange({ ...config, origins: [...origins, origin] }, `Pages on ${origin} may open this project's rooms.`);
    setDraft("");
  }

  function remove(origin) {
    onChange({ ...config, origins: origins.filter((o) => o !== origin) }, `Pages on ${origin} are refused again.`);
  }

  return (
    <div>
      <h5 className="text-[0.78rem] font-semibold text-foreground">Other sites that may open room sockets</h5>
      <p className="mt-1 text-[0.78rem] leading-[1.45] text-muted-foreground">
        A browser page on another site cannot join this project&apos;s rooms unless its origin is listed here, so a
        page elsewhere cannot borrow a visitor&apos;s sign-in. Pages on this project&apos;s own hosts need no entry; a
        game or server that connects without a browser is not affected.
      </p>
      {origins.length ? (
        <ul className="mt-2 flex flex-col divide-y divide-border border border-border">
          {origins.map((origin) => (
            <li key={origin} className="flex flex-wrap items-center gap-3 px-3 py-2">
              <code className="font-mono text-[0.8rem] text-foreground">{origin}</code>
              <span className="ml-auto">
                <Button type="button" size="sm" variant="ghost" onClick={() => remove(origin)} disabled={busy}>Remove</Button>
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      <form onSubmit={submit} className="mt-2 flex flex-wrap items-center gap-2">
        <Input
          id="addressing-new-origin"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="https://game.example.com"
          className="w-[18rem] font-mono"
        />
        <Button type="submit" size="sm" variant="outline" disabled={busy || !draft.trim()}>Add site</Button>
        <span className="text-[0.74rem] text-muted-foreground">An origin: https:// and a host, or https://*.example.com for its subdomains; no path.</span>
      </form>
    </div>
  );
}
