import { cx, useState } from "zeb/react";
import { requestJson } from "@/components/lib/http";
import Button from "@/components/ui/button";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

/** Which model answers, and with which credential. */
export default function AssistantPanel({ api, credentials, initialConfig }) {
  const creds = Array.isArray(credentials) ? credentials : [];
  const [highModel, setHighModel] = useState(
    String(initialConfig?.llm_high_credential_id || "")
  );
  const [generalModel, setGeneralModel] = useState(
    String(initialConfig?.llm_general_credential_id || "")
  );
  const [maxSteps, setMaxSteps] = useState(Number(initialConfig?.max_steps ?? 50));
  const [maxReplans, setMaxReplans] = useState(Number(initialConfig?.max_replans ?? 2));
  const [historyPairs, setHistoryPairs] = useState(
    Number(initialConfig?.chat_history_pairs ?? 10)
  );
  const [enabled, setEnabled] = useState(Boolean(initialConfig?.enabled));
  const [statusMsg, setStatusMsg] = useState("Ready.");
  const [statusTone, setStatusTone] = useState("info");
  const [saving, setSaving] = useState(false);

  async function handleSubmit(e) {
    e.preventDefault();
    setSaving(true);
    setStatusMsg("Saving...");
    setStatusTone("info");
    try {
      await requestJson(api, {
        method: "PUT",
        body: JSON.stringify({
          llm_high_credential_id: highModel.trim() || null,
          llm_general_credential_id: generalModel.trim() || null,
          max_steps: maxSteps,
          max_replans: maxReplans,
          chat_history_pairs: historyPairs,
          enabled,
        }),
      });
      setStatusMsg("Saved.");
      setStatusTone("ok");
    } catch (err) {
      setStatusMsg(`Failed: ${err?.message || String(err)}`);
      setStatusTone("error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <article className="border border-border rounded-lg bg-card p-[0.85rem] mb-[0.9rem]">
      <header className="flex items-start justify-between gap-3 mb-[0.65rem]">
        <div>
          <h3 className="project-card-title">Project Assistant</h3>
          <p className="project-card-copy">Bind credential profiles for assistant reasoning tiers.</p>
        </div>
        <span className="project-inline-chip">Automaton</span>
      </header>

      <form className="grid grid-cols-2 gap-[0.65rem]" onSubmit={handleSubmit}>
        <label className="pipeline-editor-field">
          <span>High Model</span>
          <Select name="llm_high_credential_id" value={highModel} onChange={(e) => setHighModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Planning and decomposition model.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>General Model</span>
          <Select name="llm_general_credential_id" value={generalModel} onChange={(e) => setGeneralModel(e.target.value)}>
            <SelectOption value="" label="None" />
            {creds.map((item, index) => (
              <SelectOption
                key={`${item?.credential_id ?? "credential-general"}-${index}`}
                value={item?.credential_id ?? ""}
                label={`${item?.title} · ${item?.credential_id}`}
              />
            ))}
          </Select>
          <small className="pipeline-editor-field-help">Default model for regular project chat requests.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Steps</span>
          <Input
            type="number"
            name="max_steps"
            min={1}
            max={1000}
            value={maxSteps}
            onChange={(e) => setMaxSteps(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Upper bound for future multi-step agent execution.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Max Replans</span>
          <Input
            type="number"
            name="max_replans"
            min={0}
            max={64}
            value={maxReplans}
            onChange={(e) => setMaxReplans(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">Maximum replanning attempts before stopping.</small>
        </label>

        <label className="pipeline-editor-field">
          <span>Chat History Pairs</span>
          <Input
            type="number"
            name="chat_history_pairs"
            min={0}
            max={50}
            value={historyPairs}
            onChange={(e) => setHistoryPairs(Number(e.target.value))}
          />
          <small className="pipeline-editor-field-help">
            Number of previous user/assistant exchanges kept as context (0 = no history).
          </small>
        </label>

        <div className="flex flex-col gap-2 pt-1">
          <Checkbox
            name="enabled"
            label="Enable assistant for this project"
            checked={enabled}
            onChange={(e) => setEnabled(e.target.checked)}
          />
        </div>

        <div className="col-span-full flex items-center gap-[0.7rem]">
          <Button
            type="submit"
            variant="primary"
            size="sm"
            disabled={saving}
            label={saving ? "Saving..." : "Save Assistant Config"}
          />
          <span className={cx("text-[0.72rem]", statusTone === "ok" ? "text-info" : statusTone === "error" ? "text-red-300" : "text-muted-foreground")}>{statusMsg}</span>
        </div>
      </form>
    </article>
  );
}
