import CheckboxField from "@/components/ui/checkbox-field";
import TagsInput from "@/pages/project-studio/credentials/components/fields/tags-input";
import ControlSection from "@/pages/project-studio/credentials/components/zebflow-control/control-section";
import { DEFAULT_DENIED } from "@/pages/project-studio/credentials/components/zebflow-control/control-lib";

/** Which nodes a pipeline the app makes may contain: all but a list, or only a list. */
export default function ControlNodes({ mode, nodes, onChange }: { mode: string; nodes: string[]; onChange: (mode: string, nodes: string[]) => void }) {
  const allow = mode === "allow";
  return (
    <ControlSection
      title="Which nodes its pipelines may use"
      hint="Write a node (postgres.query.run) or a whole family (logic.*). zebflow.* is never allowed, whatever this says."
    >
      <div className="grid gap-1.5">
        <CheckboxField
          label="Every node except the ones listed"
          description={`With an empty list, the project's own stores are refused: ${DEFAULT_DENIED.join(", ")}.`}
          checked={!allow}
          onChange={() => onChange("deny", nodes)}
        />
        <CheckboxField
          label="Only the nodes listed"
          description="List the triggers too, for example trigger.webhook."
          checked={allow}
          onChange={() => onChange("allow", nodes)}
        />
      </div>
      <TagsInput value={nodes} onChange={(next) => onChange(allow ? "allow" : "deny", next)} placeholder={allow ? "trigger.webhook" : "kv.*"}
        empty={allow ? "Nothing listed yet, so no node is allowed." : "Nothing listed, so the project's own stores are refused."}
      />
    </ControlSection>
  );
}
