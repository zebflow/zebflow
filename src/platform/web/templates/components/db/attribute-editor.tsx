import { cx } from "zeb/react";
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Select, SelectOption } from "@/components/ui/select";

/**
 * One table's columns as rows of a form, shared by creating a table and by
 * changing one.
 *
 * Attribute kinds and index options are data, so an engine widens them
 * without the editor changing. NOT NULL, DEFAULT and UNIQUE are editable where
 * the engine declares `column_constraints`; elsewhere they are shown disabled,
 * because a column has them and hiding them would misrepresent the table.
 */

/**
 * Index kinds one type family can carry.
 *
 * Keyed by family rather than by type name, so an engine adding `timestamptz`
 * or `int8` needs no change here — the family it declares decides.
 */
const INDEX_OPTIONS_BY_FAMILY = {
  text: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  number: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  date_time: [
    { id: "hash", label: "Exact" },
    { id: "range", label: "Range" },
  ],
  uuid: [{ id: "hash", label: "Exact" }],
  boolean: [{ id: "hash", label: "Exact" }],
  json: [],
  binary: [],
  geometry: [{ id: "spatial", label: "Spatial" }],
  vector: [{ id: "vector", label: "Vector" }],
  other: [],
};

/** The mark shown beside a column name, the way a database tool does it. */
const FAMILY_GLYPH = {
  text: "A",
  number: "#",
  boolean: "✓",
  json: "{}",
  date_time: "◴",
  uuid: "⚇",
  binary: "▦",
  geometry: "◈",
  vector: "∴",
  other: "·",
};

const GRID = "grid-cols-[minmax(0,1.3fr)_minmax(0,1.2fr)_3rem_minmax(0,1fr)_3rem_minmax(0,9rem)_1.75rem]";

export function familyOfType(types, name) {
  const match = typeEntry(types, name);
  return String(match?.family || "other");
}

export function glyphForFamily(family) {
  return FAMILY_GLYPH[family] || FAMILY_GLYPH.other;
}

export const DEFAULT_ATTRIBUTE = {
  name: "",
  kind: "",
  index_types: [],
  not_null: false,
  default: "",
  unique: false,
};

/**
 * A form row for a column the table already has. `existing` marks it, and
 * `wasUnique` remembers the UNIQUE it came with, because what a column had
 * decides what can still change about it.
 */
export function attributeFromTable(attr) {
  return {
    name: attr?.name || "",
    kind: attr?.declared || attr?.kind || "",
    index_types: Array.isArray(attr?.index_types) ? [...attr.index_types] : [],
    not_null: attr?.not_null === true,
    default: String(attr?.default || ""),
    unique: attr?.unique === true,
    existing: true,
    wasUnique: attr?.unique === true,
  };
}

/** The attributes a create or alter request sends; unnamed rows are dropped. */
export function attributesPayload(attributes) {
  return (attributes || [])
    .map((item) => ({
      name: String(item?.name || "").trim(),
      kind: String(item?.kind || ""),
      index_types: Array.isArray(item?.index_types) ? item.index_types : [],
      not_null: item?.not_null === true,
      default: String(item?.default || "").trim(),
      unique: item?.unique === true,
    }))
    .filter((item) => item.name);
}

/** The header the attribute rows line up under. */
export function AttributeEditorHeader() {
  return (
    <div className={cx("grid items-center gap-2 border-b border-border/70 px-2 pb-1 text-[10px] font-medium uppercase tracking-[0.12em] text-muted-foreground", GRID)}>
      <span>Column</span>
      <span>Data type</span>
      <span>Not null</span>
      <span>Default</span>
      <span>Unique</span>
      <span>Index</span>
      <span />
    </div>
  );
}

/**
 * One column, laid out as a row rather than a card so a table with twenty of
 * them stays readable without scrolling past each one.
 *
 * On a column the table already has, NOT NULL and DEFAULT are read-only and
 * UNIQUE can be added but not removed: that is what the engine can change
 * after a column exists, and the form offers no more than that.
 */
export function AttributeEditorRow({ item, types, constraints, onChange, onRemove }) {
  const family = familyOfType(types, item?.kind);
  const options = INDEX_OPTIONS_BY_FAMILY[family] || [];
  const fixed = !constraints || item?.existing === true;
  const why = !constraints ? "This engine does not set it from here" : fixed ? "Set when the column was added" : "";

  return (
    <div className={cx("grid min-w-0 items-center gap-2 border-b border-border/40 px-2 py-1.5", GRID)}>
      <div className="flex min-w-0 items-center gap-1.5">
        <span className="w-4 shrink-0 text-center text-[11px] text-muted-foreground" title={family}>
          {glyphForFamily(family)}
        </span>
        <Input
          className="min-w-0 h-7 text-xs"
          value={item?.name || ""}
          onInput={(event) => onChange({ ...item, name: event?.target?.value || "" })}
          placeholder="column_name"
          disabled={item?.existing === true}
        />
      </div>

      <TypeCell types={types} value={item?.kind || ""} onChange={(kind) => onChange({ ...item, kind, index_types: [] })} />

      <label className="flex items-center justify-center" title={why}>
        <input
          type="checkbox"
          checked={item?.not_null === true}
          disabled={fixed}
          className={fixed ? "opacity-40" : ""}
          onChange={(event) => onChange({ ...item, not_null: event?.target?.checked === true })}
        />
      </label>

      <Input
        className={cx("min-w-0 h-7 text-xs", fixed ? "opacity-60" : "")}
        value={item?.default || ""}
        disabled={fixed}
        title={why}
        onInput={(event) => onChange({ ...item, default: event?.target?.value || "" })}
        placeholder={fixed ? "" : "none"}
      />

      <label className="flex items-center justify-center" title={item?.wasUnique ? "Already unique" : !constraints ? why : ""}>
        <input
          type="checkbox"
          checked={item?.unique === true}
          disabled={!constraints || item?.wasUnique === true}
          className={!constraints || item?.wasUnique ? "opacity-40" : ""}
          onChange={(event) => onChange({ ...item, unique: event?.target?.checked === true })}
        />
      </label>

      <IndexCell options={options} item={item} onChange={onChange} />

      <Button type="button" variant="ghost" size="sm" className="h-7 w-7 shrink-0 p-0" onClick={onRemove} title="Remove column">
        {"✕"}
      </Button>
    </div>
  );
}

/**
 * The type picker. A type that takes a parameter — a vector's dimension, a
 * geometry's shape — gets a small field beside it, and the two are written
 * back as one type: `VECTOR(384)`.
 */
function TypeCell({ types, value, onChange }) {
  const catalog = Array.isArray(types) ? types : [];
  const base = String(value || "").split("(")[0].trim();
  const inner = String(value || "").includes("(") ? String(value).slice(String(value).indexOf("(") + 1).replace(/\)\s*$/, "") : "";
  const entry = typeEntry(catalog, base);
  const listed = !base || !!entry;
  const compose = (name, param) => (param ? `${name}(${param})` : name);

  return (
    <div className="flex min-w-0 items-center gap-1">
      <Select
        className="min-w-0 h-7 flex-1 text-xs"
        title={entry?.note || value}
        value={listed ? entry?.name || "" : base}
        onChange={(event) => onChange(event?.target?.value || "")}
      >
        {catalog.length === 0 || !base ? <SelectOption value="" label="—" /> : null}
        {listed ? null : <SelectOption value={base} label={base} />}
        {catalog.map((item) => (
          <SelectOption key={item.name} value={item.name} label={item.name} />
        ))}
      </Select>
      {entry?.parameterized ? (
        <Input
          className="h-7 w-16 shrink-0 text-xs"
          value={inner}
          title={entry.note || ""}
          placeholder={entry.family === "vector" ? "384" : "Point"}
          onInput={(event) => onChange(compose(entry.name, String(event?.target?.value || "").trim()))}
        />
      ) : null}
    </div>
  );
}

/** The index kinds the column's type family can carry, as checkboxes. */
function IndexCell({ options, item, onChange }) {
  if (!options.length) return <span className="text-[11px] text-muted-foreground">—</span>;
  const chosen = Array.isArray(item?.index_types) ? item.index_types : [];
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {options.map((option) => (
        <label key={option.id} className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
          <input
            type="checkbox"
            checked={chosen.includes(option.id)}
            onChange={(event) => {
              const next = new Set(chosen);
              if (event?.target?.checked) next.add(option.id);
              else next.delete(option.id);
              onChange({ ...item, index_types: Array.from(next) });
            }}
          />
          <span>{option.label}</span>
        </label>
      ))}
    </div>
  );
}

/** The catalog entry a type names, matched without its parameter or case. */
function typeEntry(types, name) {
  const wanted = String(name || "").split("(")[0].trim().toLowerCase();
  return (types || []).find((entry) => String(entry?.name || "").toLowerCase() === wanted) || null;
}
