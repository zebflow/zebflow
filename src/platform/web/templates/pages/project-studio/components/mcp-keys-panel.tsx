import { useState, useEffect, useRef } from "zeb/react";
import { useWindowEvent } from "zeb/use";
import Button from "@/components/ui/button";
import DropdownMenuContent from "@/components/ui/dropdown-menu-content";
import { requestJson } from "@/components/lib/http";
import { SessionIcon } from "@/pages/project-studio/components/icons";
import { useStudioChrome } from "@/pages/project-studio/components/studio-chrome-context";
import { McpKeyList } from "@/pages/project-studio/components/mcp-key-list";
import { McpKeyForm } from "@/pages/project-studio/components/mcp-key-form";
import { McpKeySecret } from "@/pages/project-studio/components/mcp-key-secret";

/**
 * The project's MCP keys (`project-access` § MCP keys): one per person's agent,
 * each doing only what its person may. Lists them, makes one, and shows a new
 * key's secret the one time it exists.
 */
export function McpKeysPanel({ owner, project }) {
  const { activePanel, openHeaderPanel } = useStudioChrome();
  const [open, setOpen] = useState(false);
  const [keys, setKeys] = useState([]);
  const [people, setPeople] = useState([]);
  const [view, setView] = useState("list");
  const [created, setCreated] = useState(null);
  const [error, setError] = useState("");
  const detailsRef = useRef(null);
  const keysUrl = `/api/projects/${owner}/${project}/mcp/keys`;

  async function load(quiet) {
    try {
      const data = await requestJson(keysUrl);
      if (data) {
        setKeys(Array.isArray(data.keys) ? data.keys : []);
        setPeople(Array.isArray(data.people) ? data.people : []);
      }
      setError("");
    } catch (err) {
      if (quiet) return;
      setError(err?.status === 403 ? "Making and listing MCP keys needs the maintainer role on this project." : String(err?.message || err));
    }
  }

  function close() {
    const details = detailsRef.current;
    if (details?.open) {
      details.open = false;
      setOpen(false);
    }
  }

  useEffect(() => {
    load(true);
  }, []);

  useEffect(() => {
    if (open) load(false);
  }, [open]);

  useEffect(() => {
    if (activePanel !== "session") close();
  }, [activePanel]);

  useWindowEvent("rwe:nav", close);

  function made(next) {
    setCreated(next);
    setView("made");
    load(true);
  }

  return (
    <details ref={detailsRef} className="relative inline-block group" data-dropdown-menu="true">
      <summary
        className="list-none cursor-pointer outline-none relative flex h-9 items-center justify-center gap-1.5 rounded-none bg-[#ed752e] px-2.5 font-mono text-[0.68rem] font-semibold tracking-widest text-white hover:bg-[#f6863c]"
        onClick={() => {
          if (!open) openHeaderPanel("session");
          setOpen((was) => !was);
        }}
      >
        <SessionIcon className="h-4 w-4" />
        <span>MCP</span>
        {keys.length ? <span className="absolute bottom-0 right-0 w-[7px] h-[7px] bg-green-500 pointer-events-none" /> : null}
      </summary>

      <DropdownMenuContent align="right" className="w-[760px] max-w-[calc(100vw-2rem)] border-border bg-card">
        <div className="flex items-center justify-between gap-3 px-4 py-3 border-b border-border">
          <div className="min-w-0 flex-1">
            <p className="text-sm font-semibold text-foreground">MCP keys</p>
            <p className="text-[0.68rem] text-muted-foreground mt-0.5 leading-snug">
              Each key lets one person's agent (Claude Code, Cursor, …) use this project, doing only what that person may.
            </p>
          </div>
          {view === "list" ? (
            <Button size="xs" onClick={() => setView("new")} disabled={!people.length}>New key</Button>
          ) : view === "new" ? (
            <Button variant="outline" size="xs" onClick={() => setView("list")}>Back</Button>
          ) : null}
        </div>

        {view === "list" ? <McpKeyList keys={keys} keysUrl={keysUrl} onChanged={() => load(false)} /> : null}
        {view === "new" ? <McpKeyForm keysUrl={keysUrl} people={people} onMade={made} /> : null}
        {view === "made" && created ? (
          <McpKeySecret created={created} onDone={() => { setCreated(null); setView("list"); load(false); }} />
        ) : null}

        {error ? (
          <p className="px-4 py-3 text-[0.68rem] text-red-400 border-t border-border" role="alert">{error}</p>
        ) : null}
      </DropdownMenuContent>
    </details>
  );
}
