import { stringifyCell } from "@/components/db/cell-format";
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


/** The constraints one described column carries, as short badges. */
function constraintBadges(row) {
  const [, , nullable, primaryKey, defaultValue, unique] = Array.isArray(row) ? row : [];
  const badges = [];
  if (primaryKey === "true") badges.push({ key: "pk", label: "key" });
  if (nullable === "false" && primaryKey !== "true") badges.push({ key: "nn", label: "not null" });
  if (unique === "true") badges.push({ key: "uq", label: "unique" });
  if (defaultValue) badges.push({ key: "df", label: `default ${defaultValue}` });
  return badges;
}

function Badges({ items }) {
  return items.length ? (
    <div className="flex flex-wrap gap-1.5">
      {items.map((badge) => (
        <span key={badge.key} className="inline-flex rounded-full border border-border px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
          {badge.label}
        </span>
      ))}
    </div>
  ) : (
    <span className="text-muted-foreground">—</span>
  );
}

export default function StructureTable({ activeTable, schemaColumns, schemaRows, schemaError, emptyMessage = "No declared or inferred structure available yet." }) {
  // Every engine describes a column the same way: name, the type it
  // declared, then what constrains it — shown as badges rather than a
  // column of true/false per constraint.
  if (schemaRows.length) {
    return (
      <StudioTable>
        <StudioThead>
          <tr>
            <StudioTh>Field</StudioTh>
            <StudioTh>Type</StudioTh>
            <StudioTh>Constraints</StudioTh>
          </tr>
        </StudioThead>
        <tbody>
          {schemaRows.map((row, rowIndex) => (
            <tr key={`srow-${rowIndex}`}>
              <StudioTd>{stringifyCell(row?.[0])}</StudioTd>
              <StudioTd><span className="font-mono text-[12px]">{stringifyCell(row?.[1])}</span></StudioTd>
              <StudioTd><Badges items={constraintBadges(row)} /></StudioTd>
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
                        <span key={badge.key} className="inline-flex rounded-full border border-border px-2 py-0.5 text-[11px] text-muted-foreground">
                          {badge.label}
                        </span>
                      ))
                    ) : (
                      <span className="text-muted-foreground">—</span>
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

