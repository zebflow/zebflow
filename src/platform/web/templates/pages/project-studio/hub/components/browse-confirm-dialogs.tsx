import ConfirmDialog from "@/components/ui/confirm-dialog";
import { libraryNameOf } from "@/pages/project-studio/hub/components/use-browse-actions";

/**
 * The two questions the Hub browser stops to ask: removing a package, and
 * overwriting files an add would replace. `browse` is `useBrowseActions()`.
 */
export default function BrowseConfirmDialogs({ browse }) {
  const removePrompt = browse.removePrompt;
  const overwritePrompt = browse.overwritePrompt;
  return (
    <>
      <ConfirmDialog
        open={!!removePrompt}
        onClose={browse.closeRemove}
        onConfirm={browse.confirmRemove}
        title="Remove this package from the project?"
        confirmLabel="Remove it"
        cancelLabel="Cancel"
        variant="destructive"
        busy={browse.busy}
      >
        <p className="text-[0.8rem] text-muted-foreground">
          <span className="font-mono">{libraryNameOf(removePrompt)}</span> is deleted from{" "}
          <code className="font-mono">zeb.lock</code> and its installed files are removed. Any
          template importing it stops working until it is installed again.
        </p>
      </ConfirmDialog>

      <ConfirmDialog
        open={!!overwritePrompt}
        onClose={browse.closeOverwrite}
        onConfirm={browse.confirmOverwrite}
        title="Replace files already in this project?"
        confirmLabel="Replace them"
        cancelLabel="Cancel"
        variant="destructive"
        busy={browse.busy}
      >
        <p className="text-[0.8rem] text-muted-foreground">
          Adding <span className="font-mono">{overwritePrompt?.item?.package_id}</span> writes over
          {" "}{overwritePrompt?.files?.length} file(s) that already exist here. What is in them now
          is lost.
        </p>
        <ul className="mt-3 flex flex-col gap-1">
          {(overwritePrompt?.files ?? []).map((file) => (
            <li key={file} className="font-mono text-[0.72rem] text-muted-foreground">
              {file}
            </li>
          ))}
        </ul>
      </ConfirmDialog>
    </>
  );
}
