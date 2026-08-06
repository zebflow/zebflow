import {
  StudioChip,
  StudioEmptyState,
  StudioMetric,
  StudioMetricGrid,
  StudioPanel,
  StudioPanelBody,
  StudioPanelHeader,
  StudioStatusBadge,
} from "@/pages/project-studio/components/studio-panel";

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
    <StudioPanel>
      <StudioPanelHeader
        title="Offices"
        description="Execution-plane offices registered to this controller."
        trailing={
          <span className="rounded-md border border-border bg-surface-2 px-2 py-1 font-mono text-[0.64rem] font-medium uppercase tracking-[0.14em] text-body-muted">
          {items.length} node{items.length === 1 ? "" : "s"}
          </span>
        }
      />
      <StudioPanelBody>
      {items.length === 0 ? (
        <StudioEmptyState>
          No remote offices are currently registered.
        </StudioEmptyState>
      ) : (
        <div className="space-y-2.5">
          {items.map((worker) => {
            const caps = capabilityLabels(worker?.capabilities);
            return (
            <article
              key={worker.node_id}
              className="rounded-md border border-border-soft bg-surface-2 p-3"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h3 className="truncate text-[0.82rem] font-semibold text-body">
                    {worker.label || worker.node_id}
                  </h3>
                  <p className="truncate font-mono text-[0.68rem] text-body-muted">{worker.node_id}</p>
                </div>
                <StudioStatusBadge status={worker.status || "online"} />
              </div>
              <StudioMetricGrid className="mt-3">
                <StudioMetric label="Base URL" value={worker.base_url || "Not advertised"} mono />
                <StudioMetric label="Last heartbeat" value={formatTs(worker.last_heartbeat_at)} />
                <StudioMetric label="Registered" value={formatTs(worker.registered_at)} />
                <StudioMetric label="Office ID" value={worker.office_id || worker.node_id} mono />
              </StudioMetricGrid>
              {caps.length ? (
                <div className="mt-3 flex flex-wrap gap-2">
                  {caps.map((cap) => (
                    <StudioChip key={cap}>
                      {cap}
                    </StudioChip>
                  ))}
                </div>
              ) : null}
            </article>
          );
          })}
        </div>
      )}
      </StudioPanelBody>
    </StudioPanel>
  );
}
