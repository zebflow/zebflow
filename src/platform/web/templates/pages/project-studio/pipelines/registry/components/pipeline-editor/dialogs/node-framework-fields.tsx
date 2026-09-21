import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import NodeFieldPreview from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-preview";
import type { PreviewCellValue } from "@/pages/project-studio/pipelines/registry/components/pipeline-editor/nodes/extract";

/**
 * The fields every node carries whatever its kind: the instance slug, the
 * display title, and the two canvas previews. Server-declared fields come
 * after these, through NodeForm.
 */

interface NodeFrameworkFieldsProps {
  formState: Record<string, unknown>;
  titlePlaceholder: string;
  onChange: (name: string, value: unknown) => void;
  /** An `input.*` node's widget is its input view, so it takes no `--preview-in`. */
  hidePreviewIn?: boolean;
}

export default function NodeFrameworkFields({
  formState,
  titlePlaceholder,
  onChange,
  hidePreviewIn = false,
}: NodeFrameworkFieldsProps) {
  return (
    <>
      <div className="pipeline-editor-fields-grid">
        <Field label="Slug">
          <Input
            type="text"
            value={String(formState.__node_slug || "")}
            onInput={(e) => onChange("__node_slug", e.currentTarget.value)}
          />
          <small className="text-xs text-gray-500 mt-1">
            Unique key for this node in pipeline graph edges.
          </small>
        </Field>
        <Field label="Title">
          <Input
            type="text"
            value={String(formState.title || "")}
            onInput={(e) => onChange("title", e.currentTarget.value)}
            placeholder={titlePlaceholder}
          />
          <small className="text-xs text-gray-500 mt-1">
            Custom display label. Falls back to node kind title.
          </small>
        </Field>
      </div>
      <div className="pipeline-editor-fields-grid">
        {hidePreviewIn ? null : (
          <NodeFieldPreview
            label="Preview input"
            help="Draw this node's input under its box on the canvas, from the latest run. Presentation only — the engine never reads it, but it records this payload so there is something to draw. The path is a dot path into the payload; leave it empty to take the first value that fits."
            value={formState.preview_in as PreviewCellValue}
            onChange={(next) => onChange("preview_in", next)}
          />
        )}
        <NodeFieldPreview
          label="Preview output"
          help="Draw this node's output under its box on the canvas, from the latest run. Presentation only — the engine never reads it, but it records this payload so there is something to draw. The path is a dot path into the payload; leave it empty to take the first value that fits."
          value={formState.preview_out as PreviewCellValue}
          onChange={(next) => onChange("preview_out", next)}
        />
      </div>
    </>
  );
}
