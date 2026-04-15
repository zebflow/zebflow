/**
 * Office inventory scaffold.
 *
 * This component will eventually show:
 *
 * - online/offline offices
 * - runtime capabilities and tags
 * - version and last heartbeat
 * - drain / maintenance actions
 *
 * Keeping it as a dedicated component now avoids packing future cluster UI into one page file.
 */

export function WorkersPanel({ workers }) {
  const items = Array.isArray(workers) ? workers : [];

  return (
    <section className="rounded-3xl border border-gray-200 bg-white p-5 shadow-sm">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold text-gray-900">Offices</h2>
          <p className="text-sm text-gray-600">
            Execution-plane offices registered to this controller appear here.
          </p>
        </div>
        <span className="rounded-full border border-gray-200 px-3 py-1 text-xs font-medium uppercase tracking-[0.18em] text-gray-500">
          {items.length} node{items.length === 1 ? "" : "s"}
        </span>
      </div>
      {items.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-gray-300 bg-gray-50 p-4 text-sm leading-6 text-gray-600">
          No remote offices are currently registered.
        </div>
      ) : (
        <div className="space-y-3">
          {items.map((worker) => (
            <article
              key={worker.node_id}
              className="rounded-2xl border border-gray-200 bg-gray-50 p-4"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <h3 className="text-sm font-semibold text-gray-900">
                    {worker.label || worker.node_id}
                  </h3>
                  <p className="text-xs text-gray-500">{worker.node_id}</p>
                </div>
                <span className="rounded-full bg-emerald-100 px-2.5 py-1 text-[0.7rem] font-medium uppercase tracking-[0.16em] text-emerald-700">
                  {worker.status || "online"}
                </span>
              </div>
              <p className="mt-2 text-sm text-gray-600">
                {worker.base_url || "No advertised base URL"}
              </p>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
