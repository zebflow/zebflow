import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import { requestJson } from "@/components/lib/http";
import { formatTs } from "@/components/lib/format";

const headCls = "px-2 py-1.5 text-left text-[0.6rem] font-semibold tracking-widest text-muted-foreground uppercase";
const cellCls = "px-2 py-1.5 align-top text-[0.7rem] text-foreground";

/** The project's keys, oldest first: who each acts as, what it may do, and a revoke button. */
export function McpKeyList({ keys, keysUrl, onChanged }) {
  const [asking, setAsking] = useState("");
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");

  async function revoke(keyId) {
    if (asking !== keyId) {
      setAsking(keyId);
      return;
    }
    setBusy(keyId);
    setError("");
    try {
      await requestJson(`${keysUrl}/${encodeURIComponent(keyId)}`, { method: "DELETE" });
      onChanged();
    } catch (err) {
      setError(String(err?.message || err));
    }
    setAsking("");
    setBusy("");
  }

  if (!keys.length) {
    return (
      <p className="p-4 text-[0.72rem] text-muted-foreground">
        No keys yet. A key lets one person's agent use this project's MCP, with only the rights that person holds.
      </p>
    );
  }

  return (
    <div className="max-h-[420px] overflow-y-auto">
      <table className="w-full">
        <thead>
          <tr className="border-b border-border">
            <th className={headCls}>Name</th>
            <th className={headCls}>Acts as</th>
            <th className={headCls}>May do</th>
            <th className={headCls}>Made</th>
            <th className={headCls}>Last used</th>
            <th className={headCls} />
          </tr>
        </thead>
        <tbody>
          {keys.map((key) => (
            <tr key={key.key_id} className="border-b border-border">
              <td className={cellCls}>{key.name}</td>
              <td className={cellCls}>{key.person}</td>
              <td className={cellCls} title={key.capabilities.join(", ")}>{key.capabilities.length} rights</td>
              <td className={cellCls}>{formatTs(key.created_at)} by {key.made_by}</td>
              <td className={cellCls}>{formatTs(key.last_used_at)}</td>
              <td className={cellCls}>
                <Button variant="outline" size="xs" disabled={busy === key.key_id} onClick={() => revoke(key.key_id)}
                  className="text-red-400 border-red-800/50 hover:bg-red-900/20">
                  {busy === key.key_id ? "Revoking…" : asking === key.key_id ? "Sure?" : "Revoke"}
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {error ? <p className="px-4 py-2 text-[0.68rem] text-red-400" role="alert">{error}</p> : null}
    </div>
  );
}
