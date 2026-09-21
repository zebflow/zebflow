import Toggle from "@/components/ui/toggle";

/**
 * The two switches beside the surfaces (`docs/contracts/addressing.md` §2a).
 * There is no dev mode: what a visitor gets on any project host, the dev host
 * included, is what these say. The platform address always serves the API.
 */
export default function AddressingSwitches({ config, onChange, busy }) {
  const apiOnHosts = !!config?.api_on_hosts;
  const shown = config?.errors === "shown";
  return (
    <div>
      <h5 className="text-[0.78rem] font-semibold text-foreground">What a visitor gets</h5>
      <p className="mt-1 text-[0.78rem] text-muted-foreground">
        The same on every host of this project. A webhook can override the errors switch for its own route with <code>--errors show</code> or <code>--errors hide</code>.
      </p>
      <ul className="mt-2 flex flex-col divide-y divide-border border border-border">
        <li className="flex flex-wrap items-center gap-3 px-3 py-2">
          <Toggle
            label="Platform API on this project's hosts"
            checked={apiOnHosts}
            onChange={() => onChange({ ...config, api_on_hosts: !apiOnHosts }, apiOnHosts ? "Platform API off on project hosts — it stays on the platform address." : "Platform API on project hosts — still behind sign-in.")}
            disabled={busy}
          />
          <code className="font-mono text-[0.74rem] text-muted-foreground">/api/projects/…</code>
        </li>
        <li className="flex flex-wrap items-center gap-3 px-3 py-2">
          <Toggle
            label="Show failure details on a 500"
            checked={shown}
            onChange={() => onChange({ ...config, errors: shown ? "hidden" : "shown" }, shown ? "A failure shows the error page with a reference only." : "A failure also shows its code, message, node and a link to the run — for everyone who reaches the route.")}
            disabled={busy}
          />
          <span className="text-[0.74rem] text-muted-foreground">Hidden: the page and a reference. The status code is 500 either way; the full failure is always in the run log.</span>
        </li>
      </ul>
    </div>
  );
}
