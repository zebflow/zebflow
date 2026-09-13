import {
  StudioMetric,
  StudioMetricGrid,
  StudioPanel,
  StudioPanelBody,
  StudioPanelHeader,
} from "@/pages/project-studio/components/studio-panel";

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
    <StudioPanel>
      <StudioPanelHeader
        title="Project runtime"
        description="Current resident runtime placement for this project."
      />
      <StudioPanelBody>
        <div className="rounded-md border border-border bg-muted px-3 py-2.5">
          <p className="font-mono text-[0.63rem] font-medium uppercase tracking-[0.14em] text-muted-foreground">
            Current placement
          </p>
          <p className="mt-1 text-[0.9rem] font-semibold text-foreground">
            {summary || "Local"}
          </p>
        </div>
        <StudioMetricGrid className="mt-3">
          <StudioMetric label="Mode" value={mode} />
          <StudioMetric label="Target" value={target} />
          <StudioMetric label="Office" value={valueOrLocal(officeId)} mono />
          <StudioMetric label="Node" value={valueOrLocal(workerId)} mono />
          <StudioMetric label="Replicas" value={placement?.desired_replicas || 1} />
          <StudioMetric label="State" value={state} />
        </StudioMetricGrid>
        <p className="mt-3 text-[0.72rem] leading-5 text-muted-foreground">
          Remote offices join with the cluster token configured outside the browser; the UI never exposes that secret.
        </p>
      </StudioPanelBody>
    </StudioPanel>
  );
}
