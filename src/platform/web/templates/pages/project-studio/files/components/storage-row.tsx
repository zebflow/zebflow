import { cx } from "zeb/react";
import Badge from "@/components/ui/badge";

/** The namespace every project starts with: pinned first and marked green. */
export function isDefaultStorage(storage) {
  const tags = Array.isArray(storage?.tags) ? storage.tags : [];
  return storage?.name === "default" || tags.includes("default");
}

export function StorageRow({ storage }) {
  const tags = Array.isArray(storage.tags) ? storage.tags : [];
  const isDefault = isDefaultStorage(storage);
  return (
    <tr className={cx("border-b border-border last:border-b-0", isDefault && "bg-success/5")}>
      <td className="px-3 py-2.5 text-foreground font-medium">
        <span className="inline-flex items-center gap-2">
          {isDefault ? <span className="inline-block h-2 w-2 rounded-full bg-success" aria-hidden="true" /> : null}
          {storage.name}
        </span>
      </td>
      <td className="px-3 py-2.5 text-muted-foreground">{storage.backend}</td>
      <td className="px-3 py-2.5 text-muted-foreground font-mono text-[0.74rem]">{storage.namespace}</td>
      <td className="px-3 py-2.5">
        <div className="flex flex-wrap gap-1">
          {tags.map((tag) => (
            <Badge
              key={tag}
              variant="outline"
              className={cx("text-[0.65rem]", tag === "default" && "border-success/50 bg-success/10 text-success")}
            >
              {tag}
            </Badge>
          ))}
        </div>
      </td>
      <td className="px-3 py-2.5 text-right">
        <a
          href={storage.open_href}
          className="inline-flex items-center justify-center min-h-7 px-2.5 rounded border border-border bg-muted text-foreground hover:border-primary hover:text-primary transition-colors"
        >
          Open
        </a>
      </td>
    </tr>
  );
}
