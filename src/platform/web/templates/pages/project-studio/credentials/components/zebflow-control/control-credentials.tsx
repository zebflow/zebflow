import { useState, useEffect } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import CheckboxField from "@/components/ui/checkbox-field";
import Input from "@/components/ui/input";
import ControlSection from "@/pages/project-studio/credentials/components/zebflow-control/control-section";
import { asList, toggled } from "@/pages/project-studio/credentials/components/zebflow-control/control-lib";

const FENCE_KIND = "zebflow_control";

/**
 * The credentials side of a fence: which of the project's credentials the app
 * may use, and which kinds it may create for itself.
 */
export default function ControlCredentials({ fence, set, apiList, credentialTypes, canCreate }) {
  const [project, setProject] = useState<any[]>([]);
  useEffect(() => {
    if (!apiList) return;
    requestJson(apiList)
      .then((payload) => setProject(Array.isArray(payload?.items) ? payload.items : []))
      .catch(() => setProject([]));
  }, [apiList]);

  const delegate = asList(fence.delegate);
  const kinds = asList(fence.manage_kinds);
  // A person's credentials can be handed over; the door's own are already its.
  const given = project.filter((item) => item.kind !== FENCE_KIND && !item.managed_by);
  const creatable = credentialTypes.filter((type) => type.kind !== FENCE_KIND);

  return (
    <>
      <ControlSection
        title="Credentials it may use"
        hint="Its pipelines may connect with these. It never sees their passwords, and cannot change them."
      >
        {given.length === 0 ? (
          <p className="text-xs text-muted-foreground">This project has no other credentials yet.</p>
        ) : (
          <div className="grid gap-1.5 sm:grid-cols-2">
            {given.map((item) => (
              <CheckboxField
                key={item.credential_id}
                label={`${item.title || item.credential_id} (${item.kind})`}
                description={item.credential_id}
                checked={delegate.includes(item.credential_id)}
                onChange={(e) => set({ delegate: toggled(delegate, item.credential_id, e.target.checked) })}
              />
            ))}
          </div>
        )}
      </ControlSection>

      <ControlSection
        title="Credentials it may create"
        hint={canCreate
          ? "It may create, change and delete its own credentials of these kinds, and only ones that connect to outside addresses."
          : "Tick \"Create its own credentials\" above to choose kinds here."}
      >
        <div className="grid gap-1.5 sm:grid-cols-3">
          {creatable.map((type) => (
            <CheckboxField
              key={type.kind}
              label={type.title || type.kind}
              disabled={!canCreate}
              checked={kinds.includes(type.kind)}
              onChange={(e) => set({ manage_kinds: toggled(kinds, type.kind, e.target.checked) })}
            />
          ))}
        </div>
        <label className="grid max-w-xs gap-1 text-xs text-muted-foreground">
          At most this many (empty for no limit)
          <Input
            type="number"
            min="1"
            max="10000"
            value={fence.manage_limit ?? ""}
            disabled={!canCreate}
            onChange={(e) => set({ manage_limit: e.target.value === "" ? "" : Number(e.target.value) })}
            placeholder="50"
          />
        </label>
      </ControlSection>
    </>
  );
}
