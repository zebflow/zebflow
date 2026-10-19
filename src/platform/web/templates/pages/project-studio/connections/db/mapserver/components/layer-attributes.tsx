import Badge from "@/components/ui/badge";
import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";
import { formatNumber } from "@/pages/project-studio/connections/db/mapserver/components/format-number";

/** A layer's attribute columns: whether each is public, its type and what its values look like. */
export default function LayerAttributes({ columns, exposed }) {
  const isExposed = (name) => exposed.some((p) => p === name);
  return (
    <div className="rounded-lg border border-border/80">
      <div className="px-4 py-2.5 border-b border-border/60">
        <p className="text-sm font-medium text-foreground">Attributes</p>
      </div>
      <div className="overflow-x-auto">
        <StudioTable>
          <StudioThead>
            <tr>
              <StudioTh>Field</StudioTh>
              <StudioTh>Public</StudioTh>
              <StudioTh>Type</StudioTh>
              <StudioTh>Cardinality</StudioTh>
              <StudioTh>Min</StudioTh>
              <StudioTh>Max</StudioTh>
              <StudioTh>Nulls</StudioTh>
              <StudioTh>Top Values</StudioTh>
            </tr>
          </StudioThead>
          <tbody>
            {columns.map((col, i) => (
              <tr key={`col-${col.name}-${i}`}>
                <StudioTd>
                  <span className="font-mono text-xs">{col.name}</span>
                </StudioTd>
                <StudioTd>
                  <Badge
                    variant={isExposed(col.name) ? "default" : "secondary"}
                    label={isExposed(col.name) ? "Public" : "Hidden"}
                  />
                </StudioTd>
                <StudioTd>
                  <Badge variant="secondary" label={col.data_type || "unknown"} />
                </StudioTd>
                <StudioTd>{formatNumber(col.cardinality)}</StudioTd>
                <StudioTd>{col.min != null ? String(col.min) : "—"}</StudioTd>
                <StudioTd>{col.max != null ? String(col.max) : "—"}</StudioTd>
                <StudioTd>{formatNumber(col.null_count)}</StudioTd>
                <StudioTd>
                  <div className="flex flex-wrap gap-1 max-w-xs">
                    {(col.top_values || []).slice(0, 5).map(([val, count], j) => (
                      <span key={`tv-${i}-${j}`} className="inline-flex items-center gap-1 rounded bg-accent px-1.5 py-0.5 text-[11px] text-muted-foreground">
                        <span className="truncate max-w-[100px]">{val}</span>
                        <span className="text-muted-foreground">({count})</span>
                      </span>
                    ))}
                    {(col.top_values?.length || 0) > 5 ? (
                      <span className="text-[11px] text-muted-foreground">+{col.top_values.length - 5}</span>
                    ) : null}
                  </div>
                </StudioTd>
              </tr>
            ))}
          </tbody>
        </StudioTable>
      </div>
    </div>
  );
}
