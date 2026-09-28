import { useEffect, useState } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import GraphDiagram from "@/components/db/graph-diagram";
import GraphWalkRunner from "@/components/db/graph-walk-runner";
import { graphModel, walkTemplates } from "@/components/db/graph-walks";

/**
 * The Graph tab: every table of rows and every kind of edge between them,
 * drawn; choose an arrow to walk it. Offered only where the engine's
 * relations are a graph.
 *
 * The edge types come from the catalog's edge tables, with `SHOW EDGES`
 * adding the loose ones; `tablesHref` is where a box opens its table.
 */
export default function GraphTabPanel({ tables, runDbQuery, tablesHref }) {
  const [shownEdges, setShownEdges] = useState([]);
  const [error, setError] = useState("");
  const [selectedId, setSelectedId] = useState("");
  const [key, setKey] = useState("");

  async function load() {
    try {
      const shown = await runDbQuery("SHOW EDGES", { readOnly: true, limit: 500 });
      setShownEdges(shown.objects || []);
      setError("");
    } catch (failure) {
      setShownEdges([]);
      setError(String(failure?.message || failure));
    }
  }

  useEffect(() => {
    load();
  }, [tables?.length]);

  const { nodes, links } = graphModel(tables, shownEdges);
  const selected = links.find((link) => link.id === selectedId) || null;

  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto p-4 lg:flex-row">
        <div className="min-w-0 flex-1 overflow-auto rounded-lg border border-border/70 bg-accent/10">
          <div className="flex items-center justify-between gap-3 border-b border-border/70 px-3 py-2 text-xs text-muted-foreground">
            <span>
              {nodes.length} {nodes.length === 1 ? "table" : "tables"} · {links.length} edge {links.length === 1 ? "type" : "types"} · dashed is loose
            </span>
            <Button type="button" variant="ghost" size="sm" onClick={load}>Refresh</Button>
          </div>
          {error ? <p className="px-3 py-2 text-xs text-danger">{error}</p> : null}
          {nodes.length ? (
            <GraphDiagram
              nodes={nodes}
              links={links}
              selectedId={selectedId}
              onSelectLink={setSelectedId}
              tableHref={(table) => `${tablesHref}?table=${encodeURIComponent(table)}`}
            />
          ) : (
            <div className="db-suite-empty">No tables yet.</div>
          )}
        </div>

        <div className="flex w-full min-w-0 flex-col gap-3 lg:w-[28rem]">
          {selected ? (
            <>
              <div>
                <p className="text-sm font-medium text-foreground">{selected.type}</p>
                <p className="text-xs text-muted-foreground">
                  {selected.from} → {selected.to} · {selected.loose ? "loose: written by an API or an import, no edge table" : `edge table ${selected.edgeTable.key}`}
                </p>
              </div>
              <Input
                className="h-8 text-xs"
                value={key}
                placeholder={`start row: a ${selected.from} or ${selected.to} key`}
                onInput={(event) => setKey(event?.target?.value || "")}
              />
              <GraphWalkRunner walks={walkTemplates(selected, { tables, links, key })} runDbQuery={runDbQuery} />
            </>
          ) : (
            <p className="text-sm text-muted-foreground">
              Choose an arrow to walk that kind of edge. Choose a box to open its table.
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
