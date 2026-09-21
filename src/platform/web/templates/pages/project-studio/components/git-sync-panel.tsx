import { useState, Link } from "zeb/react";
import Button from "@/components/ui/button";

/**
 * Where the project stands against its remote, and the one thing to do about
 * it (docs/contracts/project.md "Git"): unpushed → Push; behind → Sync;
 * diverged → Sync; conflict → the files, each settled as mine / theirs /
 * edit, then Continue or Abort. Every verb is a POST the panel already knows;
 * `onDone` reloads the status afterwards.
 */
const verbs = {
  clean: null,
  unknown: { label: "Fetch", path: "fetch" },
  unpushed: { label: "Push", path: "push" },
  behind: { label: "Sync", path: "sync" },
  diverged: { label: "Sync", path: "sync" },
};

export function describeSync(sync, word) {
  if (!sync || word === "no-remote") return "No remote connected.";
  const a = sync.ahead ?? null, b = sync.behind ?? null;
  if (word === "conflict") return `${sync.conflicts.length} file${sync.conflicts.length === 1 ? "" : "s"} conflict with the remote.`;
  if (word === "unknown") return "Not fetched yet.";
  if (word === "clean") return "In sync with the remote.";
  if (word === "unpushed") return `${a} commit${a === 1 ? "" : "s"} not pushed.`;
  if (word === "behind") return `${b} commit${b === 1 ? "" : "s"} to pull.`;
  return `${a} to push · ${b} to pull.`;
}

export function GitSyncPanel({ owner, project, sync, word, onDone }) {
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const base = `/api/projects/${owner}/${project}/git`;

  async function post(path, body) {
    setBusy(path);
    setError("");
    try {
      const res = await fetch(`${base}/${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify(body ?? {}),
      });
      const data = await res.json().catch(() => ({}));
      // A conflict is a state the panel shows, not an error to print.
      if (!res.ok && data.outcome !== "conflict") setError(data.error || data.message || "Failed");
    } catch (e) {
      setError(e.message || "Failed");
    }
    setBusy("");
    onDone?.();
  }

  const verb = verbs[word] ?? null;
  const tone = word === "conflict" ? "text-red-400" : word === "clean" ? "text-green-500" : "text-muted-foreground";

  return (
    <div className="border-t border-border px-3 py-[0.45rem] text-[0.72rem]">
      <div className="flex items-center justify-between gap-2">
        <span className={tone}>{describeSync(sync, word)}</span>
        {verb ? (
          <Button size="xs" variant="outline" onClick={() => post(verb.path)} disabled={!!busy}>
            {busy === verb.path ? "…" : verb.label}
          </Button>
        ) : null}
      </div>
      {error ? <p className="mt-1 text-[0.68rem] text-red-400">{error}</p> : null}

      {word === "conflict" ? (
        <div className="mt-2 flex flex-col gap-1">
          {sync.conflicts.map((path) => (
            <div key={path} className="flex flex-wrap items-center justify-between gap-2 rounded-[0.35rem] bg-accent px-2 py-1">
              <span className="font-mono text-[0.66rem]">{path}</span>
              <span className="flex gap-1">
                <Button size="xs" variant="outline" title="Keep this project's version" disabled={!!busy} onClick={() => post("resolve", { path, resolution: "mine" })}>Keep mine</Button>
                <Button size="xs" variant="outline" title="Take the remote's version" disabled={!!busy} onClick={() => post("resolve", { path, resolution: "theirs" })}>Take theirs</Button>
                <Link href={`/projects/${owner}/${project}/editor?type=file&file=${encodeURIComponent(path)}`} className="inline-flex h-6 items-center rounded-[0.35rem] border border-border px-2 text-[0.66rem] hover:bg-muted" title="Open with the conflict markers; save, then mark it resolved here">
                  Edit
                </Link>
                <Button size="xs" variant="ghost" title="The file is edited and saved; stage it as resolved" disabled={!!busy} onClick={() => post("resolve", { path, resolution: "content", content: null })}>Resolved</Button>
              </span>
            </div>
          ))}
          <div className="mt-1 flex justify-end gap-1">
            <Button size="xs" variant="ghost" disabled={!!busy} onClick={() => post("abort")}>Abort</Button>
            <Button size="xs" disabled={!!busy} onClick={() => post("continue")}>Continue</Button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
