import { useState, cx } from "zeb/react";
import Button from "@/components/ui/button";
import Field from "@/components/ui/field";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";
import { Dialog } from "@/components/ui/dialog";
import DialogContent from "@/components/ui/dialog-content";
import DialogHeader from "@/components/ui/dialog-header";
import DialogTitle from "@/components/ui/dialog-title";
import DialogFooter from "@/components/ui/dialog-footer";
import { sqlStringLiteral } from "@/components/db/table-data";
import { RelationTargetSearchDialog } from "@/components/db/relations-graph";
import {
  edgeInsertSql,
  edgePropertyColumns,
  edgeTablesTouching,
  labelColumn,
  tableByKey,
} from "@/components/db/edge-tables";

/**
 * Drawing a new edge from the selected row, through an edge table.
 *
 * An edge table fixes both ends: which table the edge starts in and which it
 * reaches. So the form asks only which edge table, which row on the other
 * side, and the edge's own properties. With no edge table touching this
 * table there is nothing to offer, and the trigger is not shown. What it will
 * not decide is whether the input was acceptable — a missing row is reported
 * through `onInvalidInput`, whose dialog belongs to the page.
 */
export default function RelationCreatePanel({ runDbQuery, tables, current, onCreated, onInvalidInput }) {
  const { table, record } = current;
  const edgeTables = edgeTablesTouching(tables, table?.graphName);

  const [open, setOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [edgeKey, setEdgeKey] = useState("");
  const [otherKey, setOtherKey] = useState("");
  const [properties, setProperties] = useState({});

  if (!edgeTables.length || !table) return null;

  const edgeTable = edgeTables.find((item) => item.key === edgeKey) || edgeTables[0];
  const edge = edgeTable.edge;
  const currentIsSource = edge.source_table === table.graphName;
  const otherTable = tableByKey(tables, currentIsSource ? edge.destination_table : edge.source_table);
  const otherTableKey = currentIsSource ? edge.destination_table : edge.source_table;
  const currentKey = String(record?._key || "").trim();

  function openDialog() {
    setEdgeKey(edgeTables[0].key);
    setOtherKey("");
    setProperties({});
    setStatus("");
    setOpen(true);
  }

  async function submit(event) {
    event?.preventDefault?.();
    const other = otherKey.trim();
    if (!currentKey) {
      setStatus("Error · Select a row first.");
      return;
    }
    if (!other) {
      setStatus(`Error · Choose the ${otherTableKey} row this edge reaches.`);
      return;
    }
    setBusy(true);
    setStatus("Creating edge…");
    try {
      const exists = await runDbQuery(
        `SELECT _key FROM ${otherTableKey} WHERE _key = '${sqlStringLiteral(other)}'`,
        { readOnly: true, tableName: otherTableKey, limit: 1 },
      );
      if (!exists.rows.length) {
        onInvalidInput({
          title: "Row Not Found",
          message: `${otherTableKey} has no row '${other}'. Search and pick an existing row.`,
          example: `${otherTableKey}/existing_key`,
        });
        setStatus(`Error · ${otherTableKey}/${other} not found.`);
        return;
      }
      const [sourceKey, destinationKey] = currentIsSource ? [currentKey, other] : [other, currentKey];
      await runDbQuery(edgeInsertSql(edgeTable, sourceKey, destinationKey, properties), {
        readOnly: false,
        tableName: edgeTable.key,
        limit: 0,
      });
      setOpen(false);
      await onCreated();
    } catch (error) {
      setStatus(`Error · ${String(error?.message || error)}`);
    } finally {
      setBusy(false);
    }
  }

  /** Rows of the other table whose label or key contains the text. */
  async function searchTargets(collection, query) {
    const target = tableByKey(tables, collection);
    const label = labelColumn(target);
    const needle = String(query || "").trim();
    const select = label ? `_key, ${label}` : "_key";
    // One ILIKE, on the label or else the key: sekejap answers a LIKE from
    // the row, so it cannot be one side of an OR.
    const where = needle ? ` WHERE ${label || "_key"} ILIKE '%${sqlStringLiteral(needle)}%'` : "";
    const result = await runDbQuery(`SELECT ${select} FROM ${collection}${where} LIMIT 50`, {
      readOnly: true,
      tableName: collection,
      limit: 50,
    });
    return (result.objects || []).map((row) => ({
      slug: String(row._key || ""),
      label: String((label && row[label]) || row._key || ""),
    }));
  }

  return (
    <>
      <Button type="button" variant="outline" size="sm" onClick={openDialog}>
        New Relation
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-xl border-border bg-card text-foreground">
          <DialogHeader className="px-6 pt-6">
            <DialogTitle>New Relation</DialogTitle>
            <p className="text-sm text-muted-foreground">
              {currentIsSource
                ? `From this ${table.table} row to a ${otherTableKey} row, through ${edgeTable.table}.`
                : `From a ${otherTableKey} row to this ${table.table} row, through ${edgeTable.table}.`}
            </p>
            {status ? (
              <p className={cx("text-xs", status.startsWith("Error") ? "text-danger" : "text-muted-foreground")}>{status}</p>
            ) : null}
          </DialogHeader>

          <form onSubmit={submit} className="flex flex-col gap-4 px-6 py-4">
            <Field label="Edge table">
              <Select value={edgeTable.key} onChange={(event) => { setEdgeKey(event?.target?.value || ""); setOtherKey(""); setProperties({}); }} disabled={busy}>
                {edgeTables.map((item) => (
                  <SelectOption
                    key={item.key}
                    value={item.key}
                    label={`${item.table} · ${item.edge.source_table} → ${item.edge.destination_table}`}
                  />
                ))}
              </Select>
            </Field>

            <Field label={`${otherTableKey} row`}>
              <div className="flex gap-2">
                <Input value={otherKey} onInput={(event) => setOtherKey(event?.target?.value || "")} placeholder="_key" required disabled={busy} />
                <Button type="button" variant="outline" size="sm" disabled={busy || !otherTable} onClick={() => setSearchOpen(true)}>
                  Search
                </Button>
              </div>
            </Field>

            {edgePropertyColumns(edgeTable).map((attr) => (
              <Field key={attr.name} label={`${attr.name}${attr.declared ? ` · ${attr.declared}` : ""}`}>
                <Input
                  value={properties[attr.name] || ""}
                  onInput={(event) => setProperties({ ...properties, [attr.name]: event?.target?.value || "" })}
                  placeholder={attr.default ? `default ${attr.default}` : ""}
                  disabled={busy}
                />
              </Field>
            ))}

            <DialogFooter>
              <Button type="button" variant="ghost" size="sm" onClick={() => setOpen(false)} disabled={busy}>
                Cancel
              </Button>
              <Button type="submit" size="sm" disabled={busy}>
                {busy ? "Creating…" : "Create"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <RelationTargetSearchDialog
        open={searchOpen}
        onOpenChange={setSearchOpen}
        tables={otherTable ? [otherTable] : []}
        onSearch={searchTargets}
        onSelect={setOtherKey}
      />
    </>
  );
}
