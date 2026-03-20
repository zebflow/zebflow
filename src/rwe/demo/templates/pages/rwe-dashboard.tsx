import { usePageState, useMemo } from "zeb";

export default function RweDashboard(input) {
  const data = input?.data || {
    users: 12492,
    revenue: 182430,
    failures: 7,
    latencyP95: 238,
    pipelines: [
      { name: "ingest-users", success: 99 },
      { name: "build-marts", success: 96 },
      { name: "stream-events", success: 93 },
      { name: "sync-crm", success: 88 },
    ],
  };

  const state = usePageState({
    range: "7d",
    showAlerts: true,
  });
  const range = state?.range ?? "7d";
  const showAlerts = state?.showAlerts ?? true;
  const setPageState = state.setPageState;

  const chart = useMemo(() => {
    if (range === "24h") return [20, 30, 28, 42, 55, 49, 62, 58];
    if (range === "30d") return [12, 22, 33, 31, 29, 48, 44, 56];
    return [14, 28, 37, 35, 42, 50, 57, 65];
  }, [range]);

  return (
    <div className="min-h-screen bg-gray-950 px-6 py-10 font-mono text-gray-100">
      <div className="mx-auto max-w-7xl rounded-2xl border border-gray-800 bg-gradient-to-br from-gray-900 to-gray-950 shadow-2xl">
        <header className="border-b border-gray-800 px-6 py-5">
          <p className="text-xs uppercase tracking-[0.22em] text-indigo-300">rwe</p>
          <h1 className="mt-2 text-3xl font-bold text-indigo-400">Operations Dashboard</h1>
          <p className="mt-1 text-sm text-gray-400">Complex example with KPI, chart, table-like lanes, and state controls.</p>
        </header>

        <main className="space-y-6 p-6">
          <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <p className="text-xs uppercase tracking-widest text-gray-500">Users</p>
              <p className="mt-2 text-3xl font-black text-indigo-300">{data.users}</p>
            </article>
            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <p className="text-xs uppercase tracking-widest text-gray-500">Revenue</p>
              <p className="mt-2 text-3xl font-black text-emerald-300">${data.revenue}</p>
            </article>
            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <p className="text-xs uppercase tracking-widest text-gray-500">Failures</p>
              <p className="mt-2 text-3xl font-black text-red-300">{data.failures}</p>
            </article>
            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <p className="text-xs uppercase tracking-widest text-gray-500">Latency P95</p>
              <p className="mt-2 text-3xl font-black text-yellow-300">{data.latencyP95}ms</p>
            </article>
          </section>

          <section className="rounded-xl border border-gray-800 bg-black/30 p-4">
            <div className="mb-4 flex flex-wrap items-center gap-2">
              {["24h", "7d", "30d"].map((nextRange) => (
                <button
                  key={nextRange}
                  type="button"
                  onClick={() => setPageState({ range: nextRange })}
                  className={
                    range === nextRange
                      ? "rounded-md border border-indigo-500 bg-indigo-600/30 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-indigo-100"
                      : "rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400 hover:text-gray-200"
                  }
                >
                  {nextRange}
                </button>
              ))}
              <button
                type="button"
                onClick={() => setPageState({ showAlerts: !showAlerts })}
                className="ml-auto rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold text-gray-300 hover:text-white"
              >
                alerts: {showAlerts ? "on" : "off"}
              </button>
            </div>

            <div className="grid h-40 grid-cols-8 items-end gap-2">
              {chart.map((point, idx) => (
                <div key={`point-${idx}`} className="rounded-t bg-indigo-500/70" style={{ height: `${point}%` }} />
              ))}
            </div>
          </section>

          <section className="grid gap-4 lg:grid-cols-[1.2fr_1fr]">
            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-sm font-semibold text-indigo-300">Pipeline Reliability</h2>
              <div className="mt-3 space-y-3">
                {data.pipelines.map((pipeline) => (
                  <div key={pipeline.name}>
                    <div className="mb-1 flex items-center justify-between text-xs">
                      <span className="text-gray-300">{pipeline.name}</span>
                      <span className={pipeline.success >= 95 ? "text-emerald-300" : "text-yellow-300"}>
                        {pipeline.success}%
                      </span>
                    </div>
                    <div className="h-2 rounded bg-gray-800">
                      <div
                        className={pipeline.success >= 95 ? "h-2 rounded bg-emerald-500" : "h-2 rounded bg-yellow-500"}
                        style={{ width: `${pipeline.success}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </article>

            <article className="rounded-xl border border-gray-800 bg-black/30 p-4">
              <h2 className="text-sm font-semibold text-indigo-300">Alert Feed</h2>
              <div className="mt-3 space-y-2 text-xs">
                {showAlerts ? (
                  <>
                    <p className="rounded border border-red-800 bg-red-900/20 px-2 py-1 text-red-200">[critical] sync-crm retries exceeded</p>
                    <p className="rounded border border-yellow-800 bg-yellow-900/20 px-2 py-1 text-yellow-200">[warn] latency P95 above baseline</p>
                    <p className="rounded border border-gray-700 bg-gray-900 px-2 py-1 text-gray-300">[info] weekly backup completed</p>
                  </>
                ) : (
                  <p className="text-gray-500">alerts are hidden</p>
                )}
              </div>
            </article>
          </section>
        </main>
      </div>
    </div>
  );
}
