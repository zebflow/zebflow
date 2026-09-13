import SettingsSection from "@/pages/project-studio/settings/components/settings-section";

/** Where this project's configuration is read from, and whether it parsed. */
export default function ProjectConfigurationStatus({ config }) {
  return (
    <SettingsSection
      title="Project Configuration"
      description="Canonical, portable project settings stored with the project source."
      tag={config?.status ?? (config?.valid ? "Valid" : "Missing")}
    >
      <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
        <div className="border border-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-muted-foreground">File</p>
          <p className="mt-1 font-mono text-[0.76rem] text-foreground">{config?.path ?? "zebflow.yaml"}</p>
        </div>
        <div className="border border-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-muted-foreground">API Version</p>
          <p className="mt-1 font-mono text-[0.76rem] text-foreground">{config?.api_version ?? "-"}</p>
        </div>
        <div className="border border-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-muted-foreground">Kind</p>
          <p className="mt-1 font-mono text-[0.76rem] text-foreground">{config?.kind ?? "-"}</p>
        </div>
        <div className="border border-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-muted-foreground">Project</p>
          <p className="mt-1 font-mono text-[0.76rem] text-foreground">{config?.metadata_name ?? "-"}</p>
        </div>
      </div>
    </SettingsSection>
  );
}
