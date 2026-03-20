import NodeField from "@/components/nodes/node-field";

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
      <div className="zf-node-layout-row">
        {item.row.map((child, i) => (
          <div key={i} className="zf-node-layout-row-cell">
            {renderItem(child, fieldMap, onChange)}
          </div>
        ))}
      </div>
    );
  }
  if (item && "col" in item) {
    return (
      <div className="zf-node-layout-col">
        {item.col.map((child, i) => renderItem(child, fieldMap, onChange))}
      </div>
    );
  }
  return null;
}

export default function NodeLayout({ layout, fieldMap, onChange }) {
  return (
    <div className="zf-node-layout">
      {layout.map((item, i) => (
        <div key={i}>{renderItem(item, fieldMap, onChange)}</div>
      ))}
    </div>
  );
}
