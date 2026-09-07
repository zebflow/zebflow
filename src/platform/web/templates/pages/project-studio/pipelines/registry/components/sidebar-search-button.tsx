import Button from "@/components/ui/button";
import { SearchIcon } from "@/pages/project-studio/pipelines/registry/components/editor-icons";
import { useFileSearchOptional } from "@/pages/project-studio/components/file-search-context";

export default function SidebarSearchButton({ editorBase, nav }) {
  const fileSearch = useFileSearchOptional();
  if (!fileSearch) return null;
  return (
    <Button
      size="sm"
      variant="ghost"
      title="Find file (⌘K)"
      onClick={() =>
        fileSearch.openFileSearch({
          onSelect: (relPath) => {
            const parts = relPath.split("/");
            const dir = parts.slice(0, -1).join("/");
            const type = relPath.endsWith(".zf.json") ? "pipeline" : "template";
            nav(`${editorBase}?type=${type}&path=${encodeURIComponent(dir)}&file=${encodeURIComponent(relPath)}`);
          },
        })
      }
      className="flex items-center gap-1.5"
    >
      <SearchIcon />
    </Button>
  );
}

// Unified pipelines registry + folder / template / doc / pipeline editors (studio).
