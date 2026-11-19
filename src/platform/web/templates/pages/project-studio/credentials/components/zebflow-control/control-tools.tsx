import CheckboxField from "@/components/ui/checkbox-field";
import ControlSection from "@/pages/project-studio/credentials/components/zebflow-control/control-section";
import { TOOL_GROUPS, toggled } from "@/pages/project-studio/credentials/components/zebflow-control/control-lib";

/** What the door may do: every operation as a box to tick, grouped by what it touches. */
export default function ControlTools({ tools, onChange }: { tools: string[]; onChange: (tools: string[]) => void }) {
  return (
    <ControlSection title="What the app may do" hint="Tick only what the app needs. Anything not ticked is refused.">
      <div className="grid gap-3 sm:grid-cols-3">
        {TOOL_GROUPS.map((group) => (
          <div key={group.title} className="grid content-start gap-1.5">
            <p className="text-xs font-medium text-muted-foreground">{group.title}</p>
            {group.tools.map((tool) => (
              <CheckboxField
                key={tool.id}
                label={tool.label}
                checked={tools.includes(tool.id)}
                onChange={(e) => onChange(toggled(tools, tool.id, e.target.checked))}
              />
            ))}
          </div>
        ))}
      </div>
    </ControlSection>
  );
}
