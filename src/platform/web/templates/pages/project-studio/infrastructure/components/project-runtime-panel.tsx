/**
 * Project placement scaffold.
 *
 * This panel is the future home of runtime placement policy controls such as:
 *
 * - local vs pinned office
 * - required capability tags
 * - later replicas and execution-backend hints
 *
 * It exists now to keep runtime placement UX separate from office inventory and to preserve a
 * stable file boundary for future implementation.
 */

export function ProjectRuntimePanel({ placement, summary }) {
  return (
    <section className="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm">
      <div className="mb-4 space-y-1">
        <h2 className="text-lg font-semibold text-slate-900">Project runtime</h2>
        <p className="text-sm text-slate-600">
          New projects choose which office should host their resident runtime.
        </p>
      </div>
      <div className="space-y-4 rounded-2xl border border-dashed border-slate-300 bg-slate-50 p-4 text-sm leading-6 text-slate-600">
        <div>
          <p className="text-xs font-medium uppercase tracking-[0.18em] text-slate-500">
            Current placement
          </p>
          <p className="mt-1 text-base font-semibold text-slate-900">
            {summary || "Local"}
          </p>
        </div>
        <div className="grid gap-3 rounded-2xl border border-slate-200 bg-white p-4 text-sm text-slate-700">
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-slate-500">
              Mode
            </span>
            <p className="mt-1 font-medium text-slate-900">
              {placement?.mode || "shared"}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-slate-500">
              Office
            </span>
            <p className="mt-1 font-medium text-slate-900">
              {placement?.worker_id || "local"}
            </p>
          </div>
        </div>
        <div>
          <p>Planned next policies:</p>
          <ul className="list-disc space-y-1 pl-5">
            <li>Run local inside the current office</li>
            <li>Pin project runtime to one office</li>
            <li>Later select offices by capability tags</li>
          </ul>
        </div>
      </div>
    </section>
  );
}
