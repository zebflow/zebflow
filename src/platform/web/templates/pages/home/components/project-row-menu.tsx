import { useState } from "zeb/react";
import Button from "@/components/ui/button";
import DropdownMenu from "@/components/ui/dropdown-menu";
import DropdownMenuItem from "@/components/ui/dropdown-menu-item";
import { requestJson } from "@/components/lib/http";

/**
 * A project card's menu: hide it from home, or show it again.
 *
 * Offered only when the server gave the card a `home_api` — that is, when the
 * viewer may change this project's settings. The home list is rendered by the
 * server, so after the change the page is read again rather than patched.
 */
export default function ProjectRowMenu({ item }) {
  const [problem, setProblem] = useState("");
  if (!item?.home_api) return null;
  const hidden = Boolean(item.hidden);

  async function toggle() {
    setProblem("");
    try {
      await requestJson(item.home_api, {
        method: "PUT",
        body: JSON.stringify({ data: { hidden: !hidden } }),
      });
      window.location.reload();
    } catch (err) {
      setProblem(String(err?.message || err));
    }
  }

  return (
    <DropdownMenu
      align="right"
      trigger={
        <Button type="button" variant="ghost" size="sm" aria-label="Project menu" title={problem || "Project menu"}>
          ⋯
        </Button>
      }
    >
      <DropdownMenuItem label={hidden ? "Show on home" : "Hide from home"} onClick={toggle} />
    </DropdownMenu>
  );
}
