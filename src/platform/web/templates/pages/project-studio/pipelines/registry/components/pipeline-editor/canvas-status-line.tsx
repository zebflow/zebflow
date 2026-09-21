/**
 * The one line that says how the Run is going — "v_check: waiting — attempt
 * 3/40 (video still processing)" while it streams, then "Manual run
 * completed. pipeline-exec-…", "Run refused: …" or "Run failed: …". Only
 * the "Run failed:" prefix is red: a retry is a wait, told in the muted
 * style like everything else. It sits at the bottom-left of the canvas,
 * over it, with no background: the header stays two rows (title/path,
 * actions) whatever happened. The canvas's own controls live at the
 * bottom-right, so the line keeps clear of them by width and never takes
 * the pointer.
 */
export default function CanvasStatusLine({ status }: { status: string }) {
  if (!status) return null;
  const tone = status.startsWith("Run failed:") ? "text-red-400" : "text-muted-foreground";
  return (
    <div
      className={`pipeline-editor-run-status absolute left-3 bottom-3.5 z-[35] max-w-[calc(100%-320px)] truncate text-[0.72rem] pointer-events-none ${tone}`}
      title={status}
      role="status"
    >
      {status}
    </div>
  );
}
