import { defineExtension } from "zeb/ui/editor-extension";

/**
 * Table extension — rows of cells, the first row a header.
 *
 *   <Editor extensions={[tableExtension()]} />   then `/table`
 *
 * Tab and Shift-Tab move between cells (Tab in the last cell adds a row);
 * Enter moves down. The caret in a table opens a panel to add or delete a
 * row or a column. Cells hold text and marks, not blocks.
 */

/** The cell around the caret: its row and column, and the table it is in. */
function cellAround(state) {
  const $from = state.selection.$from;
  for (let depth = $from.depth; depth > 2; depth--) {
    const name = $from.node(depth).type.name;
    if (name === "table_cell" || name === "table_header") {
      return { row: $from.index(depth - 2), col: $from.index(depth - 1), table: $from.node(depth - 2), tablePos: $from.before(depth - 2) };
    }
  }
  return null;
}

/** Position just inside the cell at (row, col). */
function cellStart(table, tablePos, row, col) {
  let pos = tablePos + 1;
  for (let r = 0; r < row; r++) pos += table.child(r).nodeSize;
  pos += 1;
  const cells = table.child(row);
  for (let c = 0; c < col; c++) pos += cells.child(c).nodeSize;
  return pos + 1;
}

function rowEnd(table, tablePos, row) {
  let pos = tablePos + 1;
  for (let r = 0; r <= row; r++) pos += table.child(r).nodeSize;
  return pos;
}

function commands(schema, pm) {
  const n = schema.nodes;
  const moveTo = (tr, cell, row, col) => tr.setSelection(pm.TextSelection.create(tr.doc, cellStart(tr.doc.nodeAt(cell.tablePos), cell.tablePos, row, col)));
  const plainRow = (cols) => n.table_row.create(null, Array.from({ length: cols }, () => n.table_cell.createAndFill()));
  const addRow = (state, dispatch) => {
    const cell = cellAround(state);
    if (!cell) return false;
    if (dispatch) {
      const tr = state.tr.insert(rowEnd(cell.table, cell.tablePos, cell.row), plainRow(cell.table.child(cell.row).childCount));
      dispatch(moveTo(tr, cell, cell.row + 1, cell.col).scrollIntoView());
    }
    return true;
  };
  const step = (dir) => (state, dispatch) => {
    const cell = cellAround(state);
    if (!cell) return false;
    const cols = cell.table.child(cell.row).childCount;
    const index = cell.row * cols + cell.col + dir;
    if (index < 0) return true;
    if (index >= cell.table.childCount * cols) return addRow(state, dispatch);
    if (dispatch) dispatch(moveTo(state.tr, cell, Math.floor(index / cols), index % cols).scrollIntoView());
    return true;
  };
  return {
    insertTable: (rows = 3, cols = 3) => (state, dispatch) => {
      const header = n.table_row.create(null, Array.from({ length: cols }, () => n.table_header.createAndFill()));
      const table = n.table.create(null, [header, ...Array.from({ length: rows - 1 }, () => plainRow(cols))]);
      if (dispatch) {
        const tr = state.tr.replaceSelectionWith(table);
        const $at = tr.doc.resolve(tr.mapping.map(state.selection.from, -1));
        dispatch(tr.setSelection(pm.Selection.near($at, 1)).scrollIntoView());
      }
      return true;
    },
    tableNextCell: step(1),
    tablePreviousCell: step(-1),
    tableCellBelow: (state, dispatch) => {
      const cell = cellAround(state);
      if (!cell) return false;
      if (cell.row + 1 >= cell.table.childCount) return addRow(state, dispatch);
      if (dispatch) dispatch(moveTo(state.tr, cell, cell.row + 1, cell.col).scrollIntoView());
      return true;
    },
    tableAddRow: addRow,
    tableAddColumn: (state, dispatch) => {
      const cell = cellAround(state);
      if (!cell) return false;
      if (dispatch) {
        const tr = state.tr;
        for (let r = cell.table.childCount - 1; r >= 0; r--) {
          const type = r === 0 && cell.table.child(0).child(0).type === n.table_header ? n.table_header : n.table_cell;
          tr.insert(cellStart(cell.table, cell.tablePos, r, cell.col) - 1 + cell.table.child(r).child(cell.col).nodeSize, type.createAndFill());
        }
        dispatch(tr);
      }
      return true;
    },
    tableDeleteRow: (state, dispatch) => {
      const cell = cellAround(state);
      if (!cell) return false;
      if (dispatch) {
        if (cell.table.childCount === 1) dispatch(state.tr.delete(cell.tablePos, cell.tablePos + cell.table.nodeSize));
        else dispatch(state.tr.delete(rowEnd(cell.table, cell.tablePos, cell.row) - cell.table.child(cell.row).nodeSize, rowEnd(cell.table, cell.tablePos, cell.row)));
      }
      return true;
    },
    tableDeleteColumn: (state, dispatch) => {
      const cell = cellAround(state);
      if (!cell) return false;
      if (dispatch) {
        const tr = state.tr;
        if (cell.table.child(0).childCount === 1) tr.delete(cell.tablePos, cell.tablePos + cell.table.nodeSize);
        else for (let r = cell.table.childCount - 1; r >= 0; r--) {
          const start = cellStart(cell.table, cell.tablePos, r, cell.col) - 1;
          tr.delete(start, start + cell.table.child(r).child(cell.col).nodeSize);
        }
        dispatch(tr);
      }
      return true;
    },
  };
}

const CELL = "border border-border px-3 py-1.5 align-top";
const HEADER = "border border-border bg-muted px-3 py-1.5 text-left font-semibold";

export function tableExtension() {
  return defineExtension({
    name: "table",
    nodes: {
      table: {
        spec: { content: "table_row+", group: "block", isolating: true },
        render: (_node, r) => r.h("div", { class: "my-4 overflow-x-auto", "data-table": "" }, r.h("table", { class: "w-full border-collapse text-sm" }, r.h("tbody", {}, r.content))),
        parse: [{ tag: "table" }],
      },
      table_row: {
        spec: { content: "(table_cell | table_header)+" },
        render: (_node, r) => r.h("tr", {}, r.content),
        parse: [{ tag: "tr" }],
      },
      table_header: {
        spec: { content: "inline*", isolating: true },
        render: (_node, r) => r.h("th", { class: HEADER }, r.content),
        parse: [{ tag: "th" }],
      },
      table_cell: {
        spec: { content: "inline*", isolating: true },
        render: (_node, r) => r.h("td", { class: CELL }, r.content),
        parse: [{ tag: "td" }],
      },
    },
    commands,
    keymap: (schema, pm) => {
      const c = commands(schema, pm);
      return { Tab: c.tableNextCell, "Shift-Tab": c.tablePreviousCell, Enter: c.tableCellBelow };
    },
    insert: [{ id: "table", label: "Table", hint: "Rows and columns", keys: "table spreadsheet rows columns", run: (api) => api.exec("insertTable", 3, 3) }],
    actions: [
      { label: "Add row", run: (editor) => editor.exec("tableAddRow") },
      { label: "Add column", run: (editor) => editor.exec("tableAddColumn") },
      { label: "Delete row", run: (editor) => editor.exec("tableDeleteRow") },
      { label: "Delete column", run: (editor) => editor.exec("tableDeleteColumn") },
    ],
  });
}

export default tableExtension;
