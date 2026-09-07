/**
 * Reading the server's payload for one database connection.
 *
 * Every optional surface is gated on a capability the driver declared, never
 * on the engine's name — that is what stopped sekejap's answers being applied
 * to PostgreSQL. Kept apart from the page so the gates are readable as one
 * list rather than scattered through the component that uses them.
 */
export function readConnectionInput(input) {
  const caps = input?.capabilities ?? {};
  const schemaApi = input?.db_schema_api ?? {};
  // Editing attributes and index kinds after creation is a separate capability
  // and travels its own route.
  const tablePropertiesApi = schemaApi.properties || "";

  return {
    navLinks: input?.nav?.links ?? {},
    suiteTabs: Array.isArray(input?.suite_tabs) ? input.suite_tabs : [],
    tabFlags: input?.tab_flags ?? {},
    connection: input?.connection ?? {},
    dbApi: input?.db_runtime_api ?? {},
    // The engine's own column types, used by the type picker and the column
    // glyphs, so PostgreSQL offers timestamptz where SQLite offers INTEGER.
    dbTypes: Array.isArray(input?.db_types) ? input.db_types : [],

    caps: {
      createTable: caps.create_table === true,
      dropTable: caps.drop_table === true,
      inlineEdit: caps.inline_edit === true,
      maintenance: caps.maintenance === true,
      graphRelations: caps.relations === "graph",
      geo: caps.geo === true,
      editProperties: caps.edit_table_properties === true,
      qualifySchema: caps.schemas === true,
      // The column that addresses one row. Declared by the driver because it
      // differs: sekejap answers `_key`, SQL engines answer their primary key.
      // No fallback: a driver that forgets to declare this should show a
      // grid that cannot select a row, not one that silently inherits
      // sekejap's column name and looks like it works.
      rowIdentity: String(caps.row_identity || ""),
    },

    // Schema-definition routes arrive only when the engine supports them, so
    // an absent URL is what disables the surface.
    api: {
      tables: schemaApi.tables || "",
      properties: tablePropertiesApi,
      schemaSync: schemaApi.schema_sync || "",
      maintenance: schemaApi.maintenance || "",
      // Built from `tablePropertiesApi` before, which pointed at a path the
      // server never registers — a guaranteed 404 on every engine's
      // "Download schema" link. Read whatever the server actually declares
      // instead, absent until a connection-scoped export route exists.
      schemaExport: schemaApi.schema_export || "",
    },

    // Which table the link asked for, before anything has loaded.
    initialTable:
      typeof window !== "undefined"
        ? new URLSearchParams(window.location.search).get("table") || ""
        : "",
  };
}

/** How many of a table's columns carry an index of any kind. */
export function countIndexedColumns(table) {
  if (!table) return 0;
  return new Set([
    ...(table.hashIndexed || []),
    ...(table.rangeIndexed || []),
    ...(table.fulltextFields || []),
    ...(table.vectorFields || []),
    ...(table.spatialFields || []),
  ]).size;
}

/**
 * Column names for the inspector's chips: whatever the engine described, or
 * the declared attributes when it described nothing. Internals are hidden.
 */
export function readableFieldNames(schemaRows, table) {
  const names = schemaRows.length
    ? schemaRows.map((row, index) =>
        String(Array.isArray(row) ? (row[0] ?? `field_${index + 1}`) : `field_${index + 1}`),
      )
    : (table?.attributes || []).map((attr) => String(attr?.name || ""));
  return names.filter((name) => name && !name.startsWith("_"));
}
