import NodeFieldText from "@/components/nodes/node-field-text";
import NodeFieldTextarea from "@/components/nodes/node-field-textarea";
import NodeFieldCodeEditor from "@/components/nodes/node-field-code-editor";
import NodeFieldSelect from "@/components/nodes/node-field-select";
import NodeFieldDatalist from "@/components/nodes/node-field-datalist";
import NodeFieldMethodButtons from "@/components/nodes/node-field-method-buttons";
import NodeFieldCopyUrl from "@/components/nodes/node-field-copy-url";
import NodeFieldCheckbox from "@/components/nodes/node-field-checkbox";
import NodeFieldSection from "@/components/nodes/node-field-section";

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
    default:
      return <NodeFieldText field={field} value={value} onChange={onChange} />;
  }
}
