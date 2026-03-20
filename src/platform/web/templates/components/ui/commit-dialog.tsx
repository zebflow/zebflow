import { cx } from "zeb";
import Input from "@/components/ui/input";
import Button from "@/components/ui/button";

/**
 * CommitDialog — overlay dialog that prompts the user to enter a git commit
 * message before saving settings. Fully controlled: show/hide via `open` prop.
 *
 * Props:
 *   open           - whether to show the dialog
 *   section        - short label rendered as a chip (e.g. "rwe", "assistant")
 *   defaultMessage - pre-filled commit message
 *   onConfirm(msg) - called with the (possibly edited) message when the user confirms
 *   onCancel()     - called when the user cancels or clicks the backdrop
 */
export default function CommitDialog({ open, section, defaultMessage, onConfirm, onCancel }) {
  const [message, setMessage] = useState(defaultMessage ?? "");

  // Re-sync input value whenever the dialog opens with a (possibly new) defaultMessage.
  useEffect(() => {
    if (open) setMessage(defaultMessage ?? "");
  }, [open, defaultMessage]);

  if (!open) return null;

  function handleSubmit(e) {
    e.preventDefault();
    onConfirm((message ?? "").trim() || (defaultMessage ?? "chore: update settings"));
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onCancel} />

      {/* Panel */}
      <div className="relative bg-white dark:bg-slate-950 rounded-xl border border-slate-200 dark:border-slate-800 shadow-2xl w-full max-w-md overflow-hidden">
        <div className="flex items-center justify-between gap-3 px-5 py-3.5 border-b border-slate-200 dark:border-slate-800">
          <span className="text-sm font-semibold text-slate-900 dark:text-slate-100">Commit changes</span>
          {section ? <span className="project-inline-chip">{section}</span> : null}
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4 p-5">
          <p className="text-xs text-slate-500 dark:text-slate-400">
            Your changes will be saved and committed to the project repository.
          </p>

          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-slate-700 dark:text-slate-300">
              Commit message
            </label>
            <Input
              name="commit_message"
              value={message}
              onInput={(e) => setMessage(e.currentTarget.value)}
              autoComplete="off"
              className="font-mono"
            />
          </div>

          <div className="flex justify-end gap-2">
            <Button type="button" variant="outline" size="sm" label="Cancel" onClick={onCancel} />
            <Button type="submit" variant="primary" size="sm" label="Commit & Save" />
          </div>
        </form>
      </div>
    </div>
  );
}
