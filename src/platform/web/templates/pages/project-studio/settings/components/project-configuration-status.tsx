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
        <div className="border border-dark-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-body-soft">File</p>
          <p className="mt-1 font-mono text-[0.76rem] text-body">{config?.path ?? "zebflow.yaml"}</p>
        </div>
        <div className="border border-dark-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-body-soft">API Version</p>
          <p className="mt-1 font-mono text-[0.76rem] text-body">{config?.api_version ?? "-"}</p>
        </div>
        <div className="border border-dark-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-body-soft">Kind</p>
          <p className="mt-1 font-mono text-[0.76rem] text-body">{config?.kind ?? "-"}</p>
        </div>
        <div className="border border-dark-border bg-dark-panel px-3 py-2.5">
          <p className="text-[0.66rem] uppercase tracking-[0.08em] text-body-soft">Project</p>
          <p className="mt-1 font-mono text-[0.76rem] text-body">{config?.metadata_name ?? "-"}</p>
        </div>
      </div>
    </SettingsSection>
  );
}
