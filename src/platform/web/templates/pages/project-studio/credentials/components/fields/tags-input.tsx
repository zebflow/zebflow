import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";

export default function TagsInput({ value, onChange, placeholder, empty }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string; empty?: string }) {
  const [text, setText] = useState("");

  function addTag() {
    const v = text.trim();
    if (!v || value.includes(v)) { setText(""); return; }
    onChange([...value, v]);
    setText("");
  }

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex flex-wrap gap-1 min-h-6">
        {value.map((tag) => (
          <span key={tag} className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-muted border border-border text-foreground">
            {tag}
            <button
              type="button"
              onClick={() => onChange(value.filter((t) => t !== tag))}
              className="text-muted-foreground hover:text-danger leading-none cursor-pointer"
              aria-label={`Remove ${tag}`}
            >×</button>
          </span>
        ))}
        {value.length === 0 && <span className="text-xs text-muted-foreground italic">{empty || "None added yet"}</span>}
      </div>
      <div className="flex gap-1.5">
        <Input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={placeholder || "Add…"}
          className="flex-1"
          onKeyDown={(e: any) => { if (e.key === "Enter") { e.preventDefault(); addTag(); } }}
        />
        <Button type="button" variant="outline" size="sm" onClick={addTag}>+ Add</Button>
      </div>
    </div>
  );
}
