import { useState, cx } from "zeb/react";
import Button from "@/components/ui/button";
import { requestJson } from "@/components/lib/http";
import SettingsSection from "@/pages/project-studio/settings/components/settings-section";
import { settingsStatusToneClass } from "@/pages/project-studio/settings/components/settings-lib";
import TransferActionsGrid from "@/pages/project-studio/settings/components/transfer-actions-grid";
import TransferOperationRow from "@/pages/project-studio/settings/components/transfer-operation-row";

const GHOST =
  "!rounded-none !border !border-border !bg-transparent !text-muted-foreground hover:!bg-border hover:!text-foreground";

/**
 * Moving a project in or out as a ProjectBundle archive.
 *
 * Owns the operation history and the one status line; the grid below does the
 * transfers and reports back.
 */
export default function ProjectTransferPanel({ owner, project, api, initialOperations }) {
  const [operations, setOperations] = useState(Array.isArray(initialOperations) ? initialOperations : []);
  const [status, setStatus] = useState({ message: "Ready.", tone: "info" });

  async function refreshOperations() {
    if (!api?.operations) return;
    try {
      const payload = await requestJson(api.operations);
      setOperations(Array.isArray(payload?.items) ? payload.items : []);
    } catch (_err) {
      // The history is a courtesy; a failure to reload it should not bury the
      // transfer result the reader is actually waiting on.
    }
  }

  return (
    <SettingsSection
      id="settings-transfer"
      title="Project Export"
      description={
        <>
          Download or apply ProjectBundle archives. Bundle carries the <code>repo</code> and{" "}
          <code>store</code> classes (<code>repo/</code> + <code>data/store/</code>, plus direct
          dependency bytes). Files carries the <code>files</code> class (Zebflow FS objects). Full
          carries all three. Imports stage, verify digests, then swap per class with recovery copies
          under <code>data/recovery/</code>.
        </>
      }
      tag="Portability"
    >
      <TransferActionsGrid
        api={api}
        onOperationsChanged={refreshOperations}
        onStatus={(message, tone) => setStatus({ message, tone })}
      />

      <div className="mt-4 flex flex-wrap items-center gap-[0.7rem]">
        <span className={cx("text-[0.72rem]", settingsStatusToneClass(status.tone))}>
          {status.message}
        </span>
        <Button type="button" variant="ghost" size="sm" className={GHOST} onClick={refreshOperations}>
          Refresh Operations
        </Button>
      </div>

      <div className="mt-4">
        <p className="mb-2 text-[0.8rem] font-medium text-foreground">Recent Operations</p>
        <div className="grid gap-2">
          {operations.length === 0 ? (
            <div className="text-[0.78rem] text-muted-foreground">No transfer operations yet.</div>
          ) : (
            operations.map((item, index) => (
              <TransferOperationRow
                key={`${item?.operation_id ?? "op"}-${index}`}
                owner={owner}
                project={project}
                item={item}
              />
            ))
          )}
        </div>
      </div>
    </SettingsSection>
  );
}
