import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import ProjectRow from "@/pages/home/components/project-row";

/**
 * Projects hidden from home. Collapsed by default, so a hidden project stays
 * out of the way; opened, each row can be edited or shown again. A project
 * hidden on another office is listed with that office's name: its setting
 * lives there, so it is changed in that project's settings.
 */
export default function HiddenProjects({ items }) {
  const [open, setOpen] = useState(false);
  const rows = Array.isArray(items) ? items : [];
  if (rows.length === 0) return null;

  return (
    <section className="mt-4" data-hidden-projects>
      <Button type="button" variant="ghost" size="sm" aria-expanded={open} onClick={() => setOpen((value) => !value)}>
        {open ? "▾" : "▸"} Hidden projects ({rows.length})
      </Button>
      {open ? (
        <div className="mt-2 divide-y divide-border rounded-[14px] border border-border bg-popover px-5 shadow-sm">
          {rows.map((item, index) => (
            <ProjectRow
              key={`${item?.owner ?? ""}/${item?.project ?? index}`}
              item={item}
              remote={!item?.edit_path}
              onOpenRemote={null}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}
