import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";

/** Declared dependencies and whether each one resolved. */
export function dependencyStatusClass(status) {
  if (status === "resolved") return "border-dark-accent2 text-dark-accent2";
  if (status === "missing" || status === "integrity_mismatch") {
    return "border-dark-accent4 text-dark-accent4";
  }
  return "border-dark-accent3 text-dark-accent3";
}

export default function DependenciesPanel({ api, initialStatus }: any) {
  const [report, setReport] = useState(initialStatus ?? { ok: true, resolved: 0, problems: 0, items: [] });
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const items = Array.isArray(report?.items) ? report.items : [];

  async function run(action) {
    setBusy(action);
    setError("");
    try {
      const payload = await requestJson(api, action === "repair" ? { method: "POST", body: "{}" } : {});
      if (payload?.report) setReport(payload.report);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy("");
    }
  }

  return (
    <section className="border border-border rounded-lg bg-surface overflow-hidden">
      <header className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
        <div className="min-w-0">
          <h3 className="text-[0.83rem] font-semibold tracking-[0.01em] text-body">Dependency Lock</h3>
          <p className="mt-1 text-[0.78rem] leading-[1.45] text-body-soft">
            Exact RWE libraries and installed node bundles required to reproduce this project.
          </p>
        </div>
        <span className="inline-flex items-center border border-border bg-surface-2 px-2 py-[0.38rem] text-[0.66rem] font-mono uppercase tracking-[0.12em] text-body-soft">
          {report?.ok ? "Resolved" : `${report?.problems ?? 0} problem(s)`}
        </span>
      </header>
      <div className="px-4 py-4">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-dark-border pb-3">
        <div className="flex items-center gap-4 text-[0.74rem] text-body-soft">
          <span><strong className="text-body">{report?.resolved ?? 0}</strong> resolved</span>
          <span><strong className={report?.problems ? "text-dark-accent4" : "text-body"}>{report?.problems ?? 0}</strong> problems</span>
          <code>{report?.lock_file ?? "zeb.lock"}</code>
        </div>
        <div className="flex items-center gap-2">
          <Button type="button" variant="outline" disabled={Boolean(busy)} onClick={() => run("refresh")}>
            {busy === "refresh" ? "Refreshing..." : "Refresh"}
          </Button>
          <Button type="button" disabled={Boolean(busy)} onClick={() => run("repair")}>
            {busy === "repair" ? "Repairing..." : "Repair"}
          </Button>
        </div>
      </div>
      {error ? <p className="border-b border-dark-border py-3 text-[0.74rem] text-dark-accent4">{error}</p> : null}
      {items.length ? (
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-left text-[0.74rem]">
            <thead>
              <tr className="border-b border-dark-border text-body-soft">
                <th className="px-2 py-2 font-medium">Dependency</th>
                <th className="px-2 py-2 font-medium">Family</th>
                <th className="px-2 py-2 font-medium">Version</th>
                <th className="px-2 py-2 font-medium">Source</th>
                <th className="px-2 py-2 font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {items.map((item, index) => (
                <tr key={`${item?.family}-${item?.name}-${index}`} className="border-b border-dark-border last:border-b-0">
                  <td className="min-w-[15rem] px-2 py-2.5 align-top">
                    <code className="text-body">{item?.name}</code>
                    <p className="mt-1 text-[0.69rem] leading-[1.4] text-body-soft">{item?.message}</p>
                    {Array.isArray(item?.definitions) && item.definitions.length ? (
                      <p className="mt-1 font-mono text-[0.66rem] text-body-soft">{item.definitions.join(", ")}</p>
                    ) : null}
                  </td>
                  <td className="whitespace-nowrap px-2 py-2.5 align-top text-body-soft">{String(item?.family ?? "").replaceAll("_", " ")}</td>
                  <td className="whitespace-nowrap px-2 py-2.5 align-top font-mono text-body">{item?.version}</td>
                  <td className="whitespace-nowrap px-2 py-2.5 align-top text-body-soft">{item?.source}</td>
                  <td className="whitespace-nowrap px-2 py-2.5 align-top">
                    <span className={cx("inline-flex border px-2 py-1 font-mono text-[0.65rem]", dependencyStatusClass(item?.status))}>
                      {String(item?.status ?? "unknown").replaceAll("_", " ")}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <p className="py-5 text-[0.76rem] text-body-soft">This project has no external runtime dependencies.</p>
      )}
      </div>
    </section>
  );
}
