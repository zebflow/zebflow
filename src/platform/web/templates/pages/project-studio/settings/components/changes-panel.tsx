import { useEffect, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { formatOperationTimestamp } from "@/pages/project-studio/settings/components/settings-lib";

const ROW_GRID = "grid grid-cols-[10rem_7rem_9rem_7rem_minmax(0,1fr)] gap-2 border-b border-border px-3 py-1.5";
const PAGE = 100;

function where(line) {
  return line.to_path ? `${line.path} → ${line.to_path}` : line.path || "—";
}

/**
 * The project's change record (`project.md` § Change record): who changed
 * which file, through what, and when — newest first, committed or not.
 */
export default function ChangesPanel({ api }) {
  const [lines, setLines] = useState([]);
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [more, setMore] = useState(false);
  const [error, setError] = useState("");

  async function load(before) {
    if (!api || busy) return;
    setBusy(true);
    setError("");
    try {
      const query = new URLSearchParams({ limit: String(PAGE) });
      if (path.trim()) query.set("path", path.trim());
      if (before) query.set("before", String(before));
      const data = await requestJson(`${api}?${query.toString()}`);
      const page = Array.isArray(data?.changes) ? data.changes : [];
      setLines(before ? [...lines, ...page] : page);
      setMore(page.length === PAGE);
    } catch (err) {
      setError(String(err?.message || err));
    }
    setBusy(false);
  }

  useEffect(() => {
    load(0);
  }, []);

  return (
    <article className="border border-border rounded-lg bg-card p-[0.85rem] mb-[0.9rem]">
      <header className="mb-[0.65rem]">
        <h3 className="project-card-title">Changes</h3>
        <p className="project-card-copy">
          Every change Zebflow made in this project's files, and who made it: in Studio, through an MCP key, or by the system. Changes not yet committed to git are here too.
        </p>
      </header>
      <form className="flex items-center gap-2 mb-2" onSubmit={(event) => { event.preventDefault(); load(0); }}>
        <Input value={path} placeholder="a file or folder, e.g. pages/" onInput={(event) => setPath(event.target.value)} />
        <Button type="submit" size="sm" variant="outline" disabled={busy}>{busy ? "Loading…" : "Show"}</Button>
      </form>
      <div className="border border-border">
        <div className={`${ROW_GRID} bg-dark-panel text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-muted-foreground`}>
          <span>When</span>
          <span>Who</span>
          <span>How</span>
          <span>What</span>
          <span>File</span>
        </div>
        <div className="max-h-[420px] overflow-auto">
          {lines.length ? lines.map((line) => (
            <div key={line.id} className={`${ROW_GRID} text-[0.72rem] last:border-b-0`}>
              <span className="text-muted-foreground">{formatOperationTimestamp(line.at)}</span>
              <span className="truncate text-foreground">{line.person || "—"}</span>
              <span className="truncate text-muted-foreground">{line.via}</span>
              <span className="text-foreground">{line.action}</span>
              <span className="truncate font-mono text-foreground" title={where(line)}>{where(line)}</span>
            </div>
          )) : (
            <p className="px-3 py-3 text-[0.72rem] text-muted-foreground">{busy ? "Loading…" : "No changes recorded yet."}</p>
          )}
        </div>
      </div>
      {more ? (
        <Button size="sm" variant="ghost" className="mt-2" disabled={busy} onClick={() => load(lines[lines.length - 1]?.id)}>
          Older
        </Button>
      ) : null}
      {error ? <p className="mt-2 text-[0.68rem] text-red-400" role="alert">{error}</p> : null}
    </article>
  );
}
