import NodeField from "@/pages/project-studio/pipelines/registry/components/nodes/node-field";

function renderItem(item, fieldMap, onChange) {
  if (typeof item === "string") {
    const f = fieldMap.get(item);
    if (!f) return null;
    return (
      <NodeField key={f.name} field={f} value={f.value} onChange={(v) => onChange(f.name, v)} />
    );
  }
  if (item && "row" in item) {
    return (
      <div className="flex flex-row gap-4 items-start">
        {item.row.map((child, i) => (
          <div key={i} className="flex-1 min-w-0">
            {renderItem(child, fieldMap, onChange)}
          </div>
        ))}
      </div>
    );
  }
  if (item && "col" in item) {
    return (
      <div className="flex flex-col gap-4">
        {item.col.map((child, i) => renderItem(child, fieldMap, onChange))}
      </div>
    );
  }
  return null;
}

export default function NodeLayout({ layout, fieldMap, onChange }) {
  return (
    <div className="flex flex-col gap-4">
      {layout.map((item, i) => (
        <div key={i}>{renderItem(item, fieldMap, onChange)}</div>
      ))}
    </div>
  );
}
