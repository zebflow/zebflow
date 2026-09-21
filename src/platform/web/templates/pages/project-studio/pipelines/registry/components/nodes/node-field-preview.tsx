import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import {
  PREVIEW_KINDS,
  normalizePreviewCell,
  previewCellSize,
} from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";
import type { PreviewCellValue } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";

/**
 * One half of a node's canvas preview — the input side or the output side.
 *
 * The value is the `{ as, path, width?, height? }` cell stored under
 * `config.preview.in` / `config.preview.out`; "off" is the absence of the
 * cell, so choosing it answers `null` rather than a cell with an empty kind.
 * The size is set by dragging the panel's corner on the canvas, not here:
 * changing the kind or the path carries it through untouched, and only
 * "off" drops it, with the rest of the cell.
 */

interface NodeFieldPreviewProps {
  label: string;
  help: string;
  value: PreviewCellValue;
  onChange: (next: PreviewCellValue) => void;
}

export default function NodeFieldPreview({
  label,
  help,
  value,
  onChange,
}: NodeFieldPreviewProps) {
  const cell = normalizePreviewCell(value);
  const as = cell?.as || "";
  const size = previewCellSize(cell) || {};

  return (
    <Field label={label} description={help}>
      <div className="flex items-center gap-2">
        <Select
          className="w-32 shrink-0"
          value={as}
          onChange={(e) => {
            const next = String((e.currentTarget as HTMLSelectElement).value || "");
            onChange(next ? { as: next, ...(cell?.path ? { path: cell.path } : {}), ...size } : null);
          }}
        >
          <SelectOption value="" label="off" />
          {PREVIEW_KINDS.map((kind) => (
            <SelectOption key={kind} value={kind} label={kind} />
          ))}
        </Select>
        {as ? (
          <Input
            type="text"
            value={cell?.path || ""}
            placeholder="payload path (optional)"
            onInput={(e) => {
              const path = String(e.currentTarget.value || "").trim();
              onChange({ as, ...(path ? { path } : {}), ...size });
            }}
          />
        ) : null}
        {as && "width" in size ? (
          <span className="shrink-0 text-xs text-muted-foreground" title="Panel size on the canvas — drag its corner to change">
            {size.width}×{size.height}
          </span>
        ) : null}
      </div>
    </Field>
  );
}
