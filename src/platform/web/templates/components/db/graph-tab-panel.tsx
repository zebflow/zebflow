import Button from "@/components/ui/button";

/** The graph inserter, announced but not yet built. */
export default function GraphTabPanel() {
  return (
    <section className="db-suite-panel db-suite-panel-fill">
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 py-10 text-center">
        <p className="text-base font-semibold text-ui-text">Interactive Graph Inserter</p>
        <p className="max-w-2xl text-sm text-ui-text-soft">
          This workspace will let users draft multiple Sekejap nodes and native edges on a canvas,
          validate the graph, then commit it as one transaction.
        </p>
        <Button type="button" variant="outline" size="sm" disabled>
          Insert Graph
        </Button>
      </div>
    </section>
  );
}
