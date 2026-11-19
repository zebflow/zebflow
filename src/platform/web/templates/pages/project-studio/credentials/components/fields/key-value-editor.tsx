import Button from "@/components/ui/button";
import Input from "@/components/ui/input";

export default function KeyValueEditor({
  value,
  onChange,
  addLabel = "+ Add",
  keyPlaceholder = "key",
  valuePlaceholder = "value",
  secretValue = false,
}: {
  value: Record<string, any>;
  onChange: (next: Record<string, string>) => void;
  addLabel?: string;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  secretValue?: boolean;
}) {
  const entries = Object.entries(value && typeof value === "object" ? value : {}).map(([key, item]) => [key, String(item ?? "")] as [string, string]);

  function commit(nextEntries: [string, string][]) {
    const out: Record<string, string> = {};
    for (const [key, item] of nextEntries) {
      const cleanKey = String(key || "").trim();
      if (!cleanKey) continue;
      out[cleanKey] = item;
    }
    onChange(out);
  }

  function updateAt(index: number, nextKey: string, nextValue: string) {
    const next = [...entries];
    next[index] = [nextKey, nextValue];
    commit(next);
  }

  function removeAt(index: number) {
    commit(entries.filter((_, itemIndex) => itemIndex !== index));
  }

  function addRow() {
    const existing = new Set(entries.map(([key]) => key));
    let candidate = "";
    let index = 0;
    while (!candidate || existing.has(candidate)) {
      index += 1;
      candidate = `KEY_${index}`;
    }
    commit([...entries, [candidate, ""]]);
  }

  return (
    <div className="flex flex-col gap-1.5">
      {entries.map(([key, item], index) => (
        <div key={index} className="flex items-center gap-1.5">
          <Input value={key} placeholder={keyPlaceholder} onInput={(e: any) => updateAt(index, e.target.value, item)} />
          <Input type={secretValue ? "password" : "text"} value={item} placeholder={valuePlaceholder} onInput={(e: any) => updateAt(index, key, e.target.value)} />
          <Button type="button" variant="ghost" size="xs" onClick={() => removeAt(index)}>×</Button>
        </div>
      ))}
      <div>
        <Button type="button" variant="outline" size="xs" onClick={addRow}>{addLabel}</Button>
      </div>
    </div>
  );
}
