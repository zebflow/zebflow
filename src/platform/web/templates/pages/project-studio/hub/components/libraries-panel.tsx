import { useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Badge from "@/components/ui/badge";
import Button from "@/components/ui/button";

/** Installed libraries and their versions. */
export default function LibrariesPanel({ items, api }) {
  const [libs, setLibs] = useState(Array.isArray(items) ? items : []);
  const [loading, setLoading] = useState(null);
  const [errorMsg, setErrorMsg] = useState(null);

  async function toggle(lib) {
    setLoading(lib.name);
    setErrorMsg(null);
    try {
      if (lib.enabled) {
        await requestJson(`${api}/disable?name=${encodeURIComponent(lib.name)}`, { method: "DELETE" });
      } else {
        await requestJson(`${api}/enable`, {
          method: "POST",
          body: JSON.stringify({
            name: lib.name,
            version: lib.packed_version,
            source: "offline",
          }),
        });
      }
      const updated = await requestJson(api);
      setLibs(Array.isArray(updated) ? updated : libs);
    } catch (err) {
      setErrorMsg(String(err?.message || err));
    } finally {
      setLoading(null);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      {errorMsg ? (
        <p className="text-[0.72rem] text-red-300">{errorMsg}</p>
      ) : null}
      {libs.map((lib) => (
        <article key={lib.name} className="border border-border rounded-lg bg-surface p-[0.85rem] mb-[0.9rem]">
          <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
            <div className="flex flex-col gap-1">
              <div className="flex items-center gap-2">
                <h3 className="project-card-title">{lib.name}</h3>
                <Badge label={lib.packed_version} variant="secondary" />
                <Badge label={lib.packed_kind} variant={lib.packed_kind === "full" ? "default" : "secondary"} />
              </div>
              <p className="project-card-copy">{lib.description}</p>
              {lib.enabled ? (
                <p className="project-card-copy" data-tone="ok">
                  locked: {lib.installed_version} · {lib.source}
                </p>
              ) : null}
            </div>
            <Button
              variant={lib.enabled ? "outline" : "primary"}
              size="sm"
              disabled={loading === lib.name}
              label={loading === lib.name ? "..." : lib.enabled ? "Disable" : "Enable"}
              onClick={() => toggle(lib)}
            />
          </header>
        </article>
      ))}
    </div>
  );
}
