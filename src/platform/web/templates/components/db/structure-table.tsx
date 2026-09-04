import { StudioTable, StudioTd, StudioThead, StudioTh } from "@/components/ui/studio-data-table";

/**
 * Column structure for one table, for every engine.
 *
 * Reads declared or inferred columns as they arrive from `describe`, so a
 * SQL engine and a document engine render through the same component.
 */

function indexBadgesForAttribute(tableItem, attrName) {
  const badges = [];
  if ((tableItem?.hashIndexed || []).includes(attrName)) badges.push({ key: "hash", label: "exact" });
  if ((tableItem?.rangeIndexed || []).includes(attrName)) badges.push({ key: "range", label: "range" });
  if ((tableItem?.fulltextFields || []).includes(attrName)) badges.push({ key: "fulltext", label: "fulltext" });
  if ((tableItem?.vectorFields || []).includes(attrName)) badges.push({ key: "vector", label: "vector" });
  if ((tableItem?.spatialFields || []).includes(attrName)) badges.push({ key: "spatial", label: "geo" });
  return badges;
}


export default function StructureTable({ activeTable, schemaColumns, schemaRows, schemaError, emptyMessage = "No declared or inferred structure available yet." }) {
  if (schemaRows.length) {
    return (
      <StudioTable>
        <StudioThead>
          <tr>
            {schemaColumns.map((col, index) => (
              <StudioTh key={`scol-${col}-${index}`}>{col}</StudioTh>
            ))}
          </tr>
        </StudioThead>
        <tbody>
          {schemaRows.map((row, rowIndex) => (
            <tr key={`srow-${rowIndex}`}>
              {(Array.isArray(row) ? row : []).map((cell, cellIndex) => (
                <StudioTd key={`scell-${rowIndex}-${cellIndex}`}>{stringifyCell(cell)}</StudioTd>
              ))}
            </tr>
          ))}
        </tbody>
      </StudioTable>
    );
  }

  if (schemaError) {
    return <div className="db-suite-empty">Failed to load structure: {schemaError}</div>;
  }

  if (activeTable?.attributes?.length) {
    return (
      <StudioTable>
        <StudioThead>
          <tr>
            <StudioTh>Name</StudioTh>
            <StudioTh>Kind</StudioTh>
            <StudioTh>Indexes</StudioTh>
          </tr>
        </StudioThead>
        <tbody>
          {activeTable.attributes.map((attr, index) => {
            const badges = indexBadgesForAttribute(activeTable, attr.name);
            return (
              <tr key={`${attr.name}-${index}`}>
                <StudioTd>{attr.name}</StudioTd>
                <StudioTd>{attr.kind || "string"}</StudioTd>
                <StudioTd>
                  <div className="flex flex-wrap gap-2">
                    {badges.length ? (
                      badges.map((badge) => (
                        <span key={badge.key} className="inline-flex rounded-full border border-ui-border px-2 py-0.5 text-[11px] text-ui-text-soft">
                          {badge.label}
                        </span>
                      ))
                    ) : (
                      <span className="text-ui-text-muted">—</span>
                    )}
                  </div>
                </StudioTd>
              </tr>
            );
          })}
        </tbody>
      </StudioTable>
    );
  }

  return <div className="db-suite-empty">{emptyMessage}</div>;
}

