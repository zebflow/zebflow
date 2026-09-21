import Button from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import type { PreviewCell } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/preview-data";

/**
 * The same preview cell the canvas draws under a node, at full size.
 * Opened by clicking the panel; it shows, it does not edit.
 */

function PreviewBody({ cell }: { cell: PreviewCell }) {
  if (cell.status === "error") {
    return (
      <pre className="m-0 max-h-[70vh] overflow-auto rounded border border-red-500/40 bg-black/40 p-3 text-xs text-red-200 whitespace-pre-wrap break-words">
        {cell.error || "error"}
      </pre>
    );
  }
  if (cell.status !== "ok") {
    return (
      <div className="rounded border border-dashed border-border px-4 py-10 text-center text-sm text-muted-foreground">
        No run yet.
      </div>
    );
  }
  if (cell.as === "image" && cell.src) {
    return <img src={cell.src} alt="" className="max-h-[70vh] w-full object-contain" />;
  }
  if (cell.as === "video" && cell.src) {
    return <video src={cell.src} controls className="max-h-[70vh] w-full" />;
  }
  if (cell.as === "audio" && cell.src) {
    return <audio src={cell.src} controls className="w-full" />;
  }
  if ((cell.as === "pdf" || cell.as === "html") && cell.src) {
    return <iframe src={cell.src} sandbox="" title="preview" className="h-[70vh] w-full border-0 bg-white" />;
  }
  if (cell.as === "table") {
    const rows = Array.isArray(cell.rows) ? cell.rows : [];
    const columns = Array.from(new Set(rows.flatMap((row) => Object.keys(row || {}))));
    if (rows.length === 0) {
      return <div className="px-4 py-8 text-center text-sm text-muted-foreground">No rows.</div>;
    }
    return (
      <div className="max-h-[70vh] overflow-auto rounded border border-border">
        <table className="w-full border-collapse text-xs">
          <tr>
            {columns.map((column) => (
              <th key={column} className="border-b border-border px-2 py-1 text-left font-semibold text-muted-foreground">
                {column}
              </th>
            ))}
          </tr>
          {rows.map((row, index) => (
            <tr key={index}>
              {columns.map((column) => {
                const value = (row as any)?.[column];
                return (
                  <td key={column} className="border-b border-border px-2 py-1 align-top">
                    {value !== null && typeof value === "object" ? JSON.stringify(value) : String(value ?? "")}
                  </td>
                );
              })}
            </tr>
          ))}
        </table>
      </div>
    );
  }
  return (
    <pre className="m-0 max-h-[70vh] overflow-auto rounded border border-border bg-black/40 p-3 text-xs text-foreground whitespace-pre-wrap break-words">
      {cell.text || ""}
    </pre>
  );
}

interface PreviewDialogProps {
  nodeId: string;
  which: "in" | "out";
  cell: PreviewCell | null;
  onClose: () => void;
}

export default function PreviewDialog({ nodeId, which, cell, onClose }: PreviewDialogProps) {
  if (!cell) return null;
  return (
    <Dialog open={true} onOpenChange={(open: boolean) => !open && onClose()}>
      <DialogContent className="max-w-3xl border-border bg-card text-foreground" onKeyDown={(e) => e.stopPropagation()}>
        <DialogHeader className="px-6 pt-6">
          <DialogTitle>
            {nodeId} · {which === "in" ? "input" : "output"}
          </DialogTitle>
          <p className="text-xs text-muted-foreground">
            Rendered as {cell.as} from the latest invocation.
          </p>
        </DialogHeader>
        <div className="px-6 py-4">
          <PreviewBody cell={cell} />
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-border px-6 py-4">
          <Button variant="outline" size="xs" type="button" onClick={onClose}>
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
