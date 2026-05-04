function valueOrLocal(value) {
  return value || "local";
}

export function ProjectRuntimePanel({ placement, summary }) {
  const target = placement?.target || "local";
  const mode = placement?.mode || "shared";
  const workerId = placement?.worker_id || placement?.target_node_id || "";
  const officeId = placement?.target_office_id || "";
  const state = placement?.effective_state || "local";

  return (
    <section className="rounded-lg border border-gray-200 bg-white p-5 shadow-sm">
      <div className="mb-4 space-y-1">
        <h2 className="text-lg font-semibold text-gray-900">Project runtime</h2>
        <p className="text-sm text-gray-600">
          Current resident runtime placement for this project.
        </p>
      </div>
      <div className="space-y-4 rounded-lg border border-gray-200 bg-gray-50 p-4 text-sm leading-6 text-gray-600">
        <div>
          <p className="text-xs font-medium uppercase tracking-[0.18em] text-gray-500">
            Current placement
          </p>
          <p className="mt-1 text-base font-semibold text-gray-900">
            {summary || "Local"}
          </p>
        </div>
        <div className="grid gap-3 rounded-lg border border-gray-200 bg-white p-4 text-sm text-gray-700 md:grid-cols-2">
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              Mode
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {mode}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              Target
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {target}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              Office
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {valueOrLocal(officeId)}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              Node
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {valueOrLocal(workerId)}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              Replicas
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {placement?.desired_replicas || 1}
            </p>
          </div>
          <div>
            <span className="text-xs uppercase tracking-[0.16em] text-gray-500">
              State
            </span>
            <p className="mt-1 font-medium text-gray-900">
              {state}
            </p>
          </div>
        </div>
        <p className="text-xs text-gray-500">
          Remote offices join with the cluster token configured outside the browser; the UI never exposes that secret.
        </p>
      </div>
    </section>
  );
}
