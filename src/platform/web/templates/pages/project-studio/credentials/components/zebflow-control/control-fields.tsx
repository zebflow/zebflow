import Input from "@/components/ui/input";
import ControlSection from "@/pages/project-studio/credentials/components/zebflow-control/control-section";
import ControlTools from "@/pages/project-studio/credentials/components/zebflow-control/control-tools";
import ControlNodes from "@/pages/project-studio/credentials/components/zebflow-control/control-nodes";
import ControlCredentials from "@/pages/project-studio/credentials/components/zebflow-control/control-credentials";
import { asList } from "@/pages/project-studio/credentials/components/zebflow-control/control-lib";

/**
 * The zebflow_control credential, written for the person drawing it: a fence
 * an app's door (zebflow.command.run) works inside. Each part says in one
 * sentence what it allows; nothing the app sends can change it.
 */
export default function ControlFields({ secret, onChange, apiList, credentialTypes }) {
  const value = secret && typeof secret === "object" ? secret : {};
  const set = (patch) => onChange({ ...value, ...patch });
  const tools = asList(value.tools);

  return (
    <div className="grid gap-3">
      <p className="text-xs text-muted-foreground">
        A fence for an app that changes its own project through a zebflow.command.run door. The app can do only what you allow here.
      </p>

      <ControlSection title="Where it may work" hint="Its files and pipelines go only in this folder. The web addresses it makes all start with this prefix.">
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="grid gap-1 text-xs text-muted-foreground">
            Folder in the project
            <Input value={value.scope ?? ""} onChange={(e) => set({ scope: e.target.value })} placeholder="sandbox/app-lab/" required />
          </label>
          <label className="grid gap-1 text-xs text-muted-foreground">
            Web address prefix
            <Input value={value.routes ?? ""} onChange={(e) => set({ routes: e.target.value })} placeholder="/lab/" required />
          </label>
        </div>
      </ControlSection>

      <ControlTools tools={tools} onChange={(next) => set({ tools: next })} />

      <ControlNodes
        mode={value.nodes_mode === "allow" ? "allow" : "deny"}
        nodes={asList(value.nodes)}
        onChange={(mode, nodes) => set({ nodes_mode: mode, nodes })}
      />

      <ControlCredentials
        fence={value}
        set={set}
        apiList={apiList}
        credentialTypes={credentialTypes}
        canCreate={tools.includes("credential_create")}
      />

      <ControlSection title="Name in the change record" hint="Every change the app makes is recorded as control · this name, with the person it acted for.">
        <Input value={value.record_as ?? ""} onChange={(e) => set({ record_as: e.target.value })} placeholder="app-assistant" />
      </ControlSection>
    </div>
  );
}
