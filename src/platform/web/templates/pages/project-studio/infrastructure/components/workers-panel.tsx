function formatTs(value) {
  if (!value) return "Never";
  try {
    return new Date(value * 1000).toLocaleString();
  } catch (_) {
    return "Unknown";
  }
}

function capabilityLabels(capabilities) {
  const labels = [];
  if (capabilities?.supports_resident) labels.push("Resident");
  if (capabilities?.supports_k8s_job) labels.push("K8s job");
  if (capabilities?.supports_spark_submit) labels.push("Spark submit");
  for (const tag of capabilities?.tags ?? []) labels.push(tag);
  return labels;
}

export function WorkersPanel({ workers }) {
  const items = Array.isArray(workers) ? workers : [];

  return (
    <section className="rounded-lg border border-gray-200 bg-white p-5 shadow-sm">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">Offices</h2>
          <p className="text-sm text-gray-600">
            Execution-plane offices registered to this controller.
          </p>
        </div>
        <span className="rounded border border-gray-200 px-3 py-1 text-xs font-medium uppercase tracking-[0.18em] text-gray-500">
          {items.length} node{items.length === 1 ? "" : "s"}
        </span>
      </div>
      {items.length === 0 ? (
        <div className="rounded-lg border border-dashed border-gray-300 bg-gray-50 p-4 text-sm leading-6 text-gray-600">
          No remote offices are currently registered.
        </div>
      ) : (
        <div className="space-y-3">
          {items.map((worker) => {
            const caps = capabilityLabels(worker?.capabilities);
            return (
            <article
              key={worker.node_id}
              className="rounded-lg border border-gray-200 bg-gray-50 p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h3 className="text-sm font-semibold text-gray-900">
                    {worker.label || worker.node_id}
                  </h3>
                  <p className="text-xs text-gray-500">{worker.node_id}</p>
                </div>
                <span className="rounded bg-emerald-100 px-2.5 py-1 text-[0.7rem] font-medium uppercase tracking-[0.16em] text-emerald-700">
                  {worker.status || "online"}
                </span>
              </div>
              <dl className="mt-3 grid gap-3 text-sm text-gray-600 md:grid-cols-2">
                <div>
                  <dt className="text-xs uppercase tracking-[0.16em] text-gray-500">Base URL</dt>
                  <dd className="mt-1 break-all text-gray-900">{worker.base_url || "Not advertised"}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.16em] text-gray-500">Last heartbeat</dt>
                  <dd className="mt-1 text-gray-900">{formatTs(worker.last_heartbeat_at)}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.16em] text-gray-500">Registered</dt>
                  <dd className="mt-1 text-gray-900">{formatTs(worker.registered_at)}</dd>
                </div>
                <div>
                  <dt className="text-xs uppercase tracking-[0.16em] text-gray-500">Office ID</dt>
                  <dd className="mt-1 text-gray-900">{worker.office_id || worker.node_id}</dd>
                </div>
              </dl>
              {caps.length ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {caps.map((cap) => (
                    <span key={cap} className="rounded border border-gray-200 bg-white px-2 py-1 text-xs text-gray-600">
                      {cap}
                    </span>
                  ))}
                </div>
              ) : null}
            </article>
          );
          })}
        </div>
      )}
    </section>
  );
}
