import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import { requestJson } from "@/components/lib/http";
import { MCP_RIGHTS, startingRights } from "@/pages/project-studio/components/mcp-key-guide";

const labelCls = "block text-[0.65rem] font-semibold tracking-widest text-muted-foreground uppercase mb-1";

function heldBy(people, person) {
  return (people.find((entry) => entry.person === person) || { rights: [] }).rights || [];
}

/**
 * Makes one key. `people` are those a key can act as, each with the rights
 * they hold on the project; a key never carries more than its person holds.
 */
export function McpKeyForm({ keysUrl, people, onMade }) {
  const first = people[0]?.person || "";
  const [person, setPerson] = useState(first);
  const [name, setName] = useState("");
  const [rights, setRights] = useState(startingRights(heldBy(people, first)));
  const [days, setDays] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const held = heldBy(people, person);

  function pick(next) {
    setPerson(next);
    setRights(startingRights(heldBy(people, next)));
  }

  function flip(key, on) {
    setRights(on ? [...rights, key] : rights.filter((item) => item !== key));
  }

  async function make(event) {
    event.preventDefault();
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      const body = { name, person, capabilities: rights, ...(days ? { expires_in_days: Number(days) } : {}) };
      const data = await requestJson(keysUrl, { method: "POST", body: JSON.stringify(body) });
      if (data?.created) onMade(data.created);
    } catch (err) {
      setError(String(err?.message || err));
    }
    setBusy(false);
  }

  return (
    <form className="p-4 space-y-3" onSubmit={make}>
      <div className="grid grid-cols-3 gap-3">
        <label>
          <span className={labelCls}>Acts as</span>
          <Select value={person} onChange={(event) => pick(event?.target?.value || "")} disabled={busy}>
            {people.map((entry) => (
              <SelectOption key={entry.person} value={entry.person} label={`${entry.person} (${entry.role})`} />
            ))}
          </Select>
        </label>
        <label>
          <span className={labelCls}>Name</span>
          <Input value={name} placeholder="laptop" onInput={(event) => setName(event.target.value)} disabled={busy} />
        </label>
        <label>
          <span className={labelCls}>Ends after (days)</span>
          <Input type="number" min="1" max="3650" value={days} placeholder="never" onInput={(event) => setDays(event.target.value)} disabled={busy} />
        </label>
      </div>
      <div>
        <p className={labelCls}>What it may do</p>
        <div className="grid grid-cols-3 gap-x-2 gap-y-0.5">
          {MCP_RIGHTS.map((right) => (
            <Checkbox
              key={right.key}
              label={right.label}
              checked={rights.includes(right.key)}
              disabled={busy || !held.includes(right.key)}
              title={held.includes(right.key) ? right.key : `${person} does not hold ${right.key}`}
              onChange={(event) => flip(right.key, event.target.checked)}
            />
          ))}
        </div>
      </div>
      <div className="flex items-center gap-3">
        <Button type="submit" size="sm" disabled={busy || !person || !rights.length}>
          {busy ? "Making…" : "Make key"}
        </Button>
        <p className="text-[0.68rem] text-muted-foreground">The key is shown once, right after it is made.</p>
      </div>
      {error ? <p className="text-[0.68rem] text-red-400" role="alert">{error}</p> : null}
    </form>
  );
}
