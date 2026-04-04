import NodeFieldText from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-text";
import NodeFieldTextarea from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-textarea";
import NodeFieldCodeEditor from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-code-editor";
import NodeFieldSelect from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-select";
import NodeFieldDatalist from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-datalist";
import NodeFieldMethodButtons from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-method-buttons";
import NodeFieldCopyUrl from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-copy-url";
import NodeFieldCheckbox from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-checkbox";
import NodeFieldSection from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-section";
import NodeFieldMultiCheckbox from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-multi-checkbox";
import NodeFieldKeyValuePairs from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-key-value-pairs";
import NodeFieldClaimsPairs from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-claims-pairs";
import NodeFieldParamsBuilder from "@/pages/project-studio/pipelines/registry/components/nodes/node-field-params-builder";

interface Props {
  field: any;
  value: unknown;
  onChange: (val: unknown) => void;
}

export default function NodeField({ field, value, onChange }: Props) {
  const type = String(field?.type || "text");

  switch (type) {
    case "section":
      return <NodeFieldSection field={field} />;
    case "code_editor":
      return <NodeFieldCodeEditor field={field} value={value} onChange={onChange as any} />;
    case "textarea":
      return <NodeFieldTextarea field={field} value={value} onChange={onChange} />;
    case "select":
      return <NodeFieldSelect field={field} value={value} onChange={onChange} />;
    case "datalist":
      return <NodeFieldDatalist field={field} value={value} onChange={onChange} />;
    case "method_buttons":
      return <NodeFieldMethodButtons field={field} value={value} onChange={onChange} />;
    case "copy_url":
      return <NodeFieldCopyUrl field={field} value={value} />;
    case "checkbox":
      return <NodeFieldCheckbox field={field} value={value} onChange={onChange} />;
    case "multi_checkbox":
      return <NodeFieldMultiCheckbox field={field} value={value} onChange={onChange} />;
    case "key_value_pairs":
      return <NodeFieldKeyValuePairs field={field} value={value} onChange={onChange} />;
    case "claims_pairs":
      return <NodeFieldClaimsPairs field={field} value={value} onChange={onChange} />;
    case "params_builder":
      return <NodeFieldParamsBuilder field={field} value={value} onChange={onChange} />;
    default:
      return <NodeFieldText field={field} value={value} onChange={onChange} />;
  }
}
