import type { APIRequestContext, Page } from "@playwright/test";
import { test, expect, visit, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";

/**
 * The Sekejap management pages, on a sekejap connection this spec creates.
 *
 * `db-suite.spec.ts` walks whatever connections a machine happens to have and
 * skips when there are none; this one makes its own, so Sekejap is always
 * covered. Every sekejap connection of a project reaches the same project
 * store, so the spec works in a throwaway project — rows it writes cannot land
 * in anyone's `default` — and deletes the connection, then the project.
 *
 * Each page is asserted on content only its panel draws, never on a status
 * code: a page whose component threw still answers 200.
 */

const PROJECT = `e2e-sekejap-${Date.now()}`;
const SLUG = "demo-store";
const api = `/api/projects/${OWNER}/${PROJECT}`;
const suite = `/projects/${OWNER}/${PROJECT}/db/sekejap/${SLUG}`;

let connectionId = "";

/** One statement through the connection's query API. */
async function sql(request: APIRequestContext, statement: string, write = true) {
  const response = await request.post(`${api}/db/connections/${connectionId}/query`, {
    data: { sql: statement, read_only: !write },
  });
  expect(response.ok(), `${statement}: ${await response.text()}`).toBeTruthy();
  return (await response.json()).result;
}

test.beforeAll(async ({ playwright }) => {
  const request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
  const created = await request.post("/home/projects/create", {
    form: { project: PROJECT, title: "E2E Sekejap" },
    maxRedirects: 0,
  });
  expect(created.status(), "create project").toBe(303);

  const connection = await request.post(`${api}/db/connections`, {
    data: { connection_slug: SLUG, connection_label: "Demo store", database_kind: "sekejap", config: {} },
  });
  expect(connection.ok(), `create connection: ${await connection.text()}`).toBeTruthy();
  connectionId = (await connection.json()).connection.connection_id;

  // Two tables of rows and an edge table between them: what the Tables,
  // Query, Schema and Graph pages all have something real to draw from.
  for (const statement of [
    "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
    "CREATE TABLE knows (src TEXT REFERENCES people, dst TEXT REFERENCES people, since INT, PRIMARY KEY (src, dst))",
    "CREATE PROPERTY GRAPH social VERTEX TABLES (people) EDGE TABLES (knows SOURCE KEY (src) REFERENCES people (_key) DESTINATION KEY (dst) REFERENCES people (_key))",
    "INSERT INTO people (_key, name) VALUES ('p1', 'Ann')",
    "INSERT INTO people (_key, name) VALUES ('p2', 'Bo')",
    "INSERT INTO knows (src, dst, since) VALUES ('p1', 'p2', 2020)",
  ]) {
    await sql(request, statement);
  }
  await request.dispose();
});

test.afterAll(async ({ playwright }) => {
  // Clean up even when an assertion failed, so a bad run leaves nothing behind.
  const request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
  const removed = await request.delete(`${api}/db/connections/${SLUG}`);
  expect([204, 404], "delete connection").toContain(removed.status());
  await request.delete(`/api/users/${OWNER}/projects/${PROJECT}`, {
    data: { project_name: PROJECT, password: PASSWORD },
  });
  await request.dispose();
});

/** Opens a suite tab and checks what every one of them must hold. */
async function openTab(page: Page, tab: string, query = "") {
  await visit(page, `${suite}/${tab}${query}`);
  await expect(page.locator('[data-db-suite="true"]')).toHaveAttribute("data-db-kind", "sekejap");
  await expect(page.locator('[data-db-suite="true"]')).toHaveAttribute("data-connection-slug", SLUG);
  expect(await page.content(), `component error on ${tab}`).not.toContain("RWE component error");
}

test("the connection is listed with the project's databases", async ({ page, consoleErrors }) => {
  await visit(page, `/projects/${OWNER}/${PROJECT}/db/connections`);
  await expect(page.locator(`a[href$="/db/sekejap/${SLUG}/tables"]`).first()).toBeVisible();
  expect(consoleErrors).toEqual([]);
});

test("tables lists the seeded tables and opens one's rows", async ({ page, consoleErrors }) => {
  await openTab(page, "tables", "?table=people");
  await expect(page.locator("[data-db-suite-object-tree]")).toBeVisible();
  const items = page.locator(".db-suite-object-item");
  // The edge table's item also says "people → people", so match the name line.
  await expect(items.locator("span.block.truncate").filter({ hasText: /^people$/ })).toHaveCount(1);
  // The edge table is listed with its two ends.
  await expect(items.filter({ hasText: "knows" })).toContainText("people → people");
  // The grid came back from the engine, not merely drawn.
  await expect(page.locator(".db-suite-grid-editor-wrap")).toContainText("Ann", { timeout: 15_000 });
  await expect(page.locator(".db-suite-grid-editor-wrap")).toContainText("Bo");
  expect(consoleErrors).toEqual([]);
});

test("query runs a statement and shows its rows", async ({ page, consoleErrors }) => {
  await openTab(page, "query");
  const status = page.locator(".db-suite-query-status");
  await expect(status).toHaveText(/Ready/);
  // The sekejap query page starts on a statement of its own.
  await expect(page.locator(".db-suite-query-editor-host")).toHaveValue("SHOW TABLES");

  await page.locator(".db-suite-query-editor-host").fill("SELECT _key, name FROM people ORDER BY name");
  await clickUntil(page, 'button:has-text("Run Query")', async () => {
    await expect(status).toHaveText(/OK · rows 2/, { timeout: 5_000 });
  });
  const grid = page.locator(".db-suite-grid-wrap");
  await expect(grid).toContainText("Ann");
  await expect(grid).toContainText("Bo");

  // A malformed statement reports the engine's refusal in the status line.
  await page.locator(".db-suite-query-editor-host").fill("SELECT FROM WHERE people");
  await page.getByRole("button", { name: "Run Query" }).click();
  await expect(status).toHaveText(/^Error · /, { timeout: 10_000 });
  // An engine error is the page working; only the status line may say so.
  expect(consoleErrors.filter((line) => !/status of 4\d\d|status of 5\d\d/.test(line))).toEqual([]);
});

test("schema describes the open table", async ({ page, consoleErrors }) => {
  await openTab(page, "schema", "?table=people");
  await expect(page.locator(".db-suite-panel").getByText("people", { exact: true }).first()).toBeVisible();
  await expect(page.getByText(/^\d+ rows$/).first()).toBeVisible();
  // The structure table names the declared column.
  await expect(page.locator(".db-suite-panel").first()).toContainText("name");
  expect(consoleErrors).toEqual([]);
});

test("graph draws the tables and the edge between them", async ({ page, consoleErrors }) => {
  await openTab(page, "graph");
  const diagram = page.getByRole("img", { name: "Tables and the edges between them" });
  await expect(diagram).toBeVisible({ timeout: 15_000 });
  await expect(diagram).toContainText("people");
  await expect(diagram).toContainText("knows");
  await expect(page.getByText(/1 table · 1 edge type/)).toBeVisible();

  // Choosing the arrow opens its walks — only a hydrated page does that.
  // The label is inside the arrow's <g>, whose onClick receives it; the
  // arrow's own hit area is a transparent path a centre click may miss.
  await clickUntil(page, "svg g.cursor-pointer text", async () => {
    await expect(page.getByText(/· edge table /)).toBeVisible({ timeout: 5_000 });
  });
  expect(consoleErrors).toEqual([]);
});

test("mart renders its panel", async ({ page, consoleErrors }) => {
  await openTab(page, "mart");
  // The mart tab lists fixed drafts and reaches no engine (mart-tab-panel.tsx).
  await expect(page.locator(".db-suite-mart-full")).toContainText("mart_sales_daily");
  expect(consoleErrors).toEqual([]);
});

test("maintenance reports the store's health", async ({ page, consoleErrors }) => {
  await openTab(page, "maintenance");
  await expect(page.getByText("Sekejap Store Maintenance")).toBeVisible();
  // The health card replaces its "Not loaded" placeholder once the store answers.
  await expect(page.getByText("Not loaded")).toHaveCount(0, { timeout: 15_000 });
  expect(consoleErrors).toEqual([]);
});

test("a table created in the UI takes a row added in the grid", async ({ page, consoleErrors }) => {
  const table = `albums_${Date.now() % 100000}`;
  await openTab(page, "tables");
  await expect(page.locator(".db-suite-object-item").first()).toBeVisible({ timeout: 15_000 });

  // --- create the table through the dialog -------------------------------
  await clickUntil(page, '.db-suite-side-actions button:has-text("Create Table")', async () => {
    await expect(page.getByRole("dialog")).toBeVisible({ timeout: 5_000 });
  });
  const dialog = page.getByRole("dialog");
  await dialog.getByPlaceholder(/^posts/).fill(table);
  // Generated keys, so a row needs no `_key` typed in.
  await dialog.locator("select").filter({ has: page.locator('option[value="ulid()"]') }).selectOption("ulid()");
  await dialog.getByPlaceholder("column_name").first().fill("title");
  await dialog.locator("select").filter({ has: page.locator('option[value="TEXT"]') }).first().selectOption("TEXT");
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  await expect(dialog).toBeHidden({ timeout: 15_000 });

  const created = page.locator(".db-suite-object-item").filter({ hasText: table });
  await expect(created).toHaveCount(1, { timeout: 15_000 });
  await created.click();
  // Sekejap lists a table under its schema, so the selection is `public.<name>`.
  await expect(page).toHaveURL(new RegExp(`[?&]table=(public\\.)?${table}$`), { timeout: 10_000 });

  // --- add a row in the grid and save it -----------------------------------
  // An empty table shows its structure until a row is added; the row's grid
  // is the table whose header names the column.
  const grid = page.locator(".db-suite-grid-editor-wrap");
  await clickUntil(page, '[title="Add row"]', async () => {
    await expect(grid.locator("thead th").filter({ hasText: /^\s*title/ }).first()).toBeVisible({ timeout: 5_000 });
  });
  const dataTable = grid.locator("table").filter({ has: page.locator("thead th", { hasText: /^\s*title/ }) }).first();
  const headers = await dataTable.locator("thead th").allTextContents();
  const titleColumn = headers.findIndex((text) => text.trim().startsWith("title"));
  expect(titleColumn, `a title column in ${JSON.stringify(headers)}`).toBeGreaterThanOrEqual(0);

  const save = page.getByTitle("Save changes");
  await expect(save).toBeEnabled({ timeout: 10_000 });
  const draftCell = dataTable.locator("tbody tr").last().locator("td").nth(titleColumn);
  await draftCell.dblclick();
  const input = draftCell.locator("input");
  await expect(input).toBeVisible({ timeout: 5_000 });
  await input.fill("Demo Album One");
  await input.press("Enter");
  await save.click();
  // Saving asks once what it will write.
  const confirm = page.getByRole("dialog").filter({ hasText: "Write changes" });
  await expect(confirm).toContainText("1 new row");
  await confirm.getByRole("button", { name: "Write", exact: true }).click();

  await expect(save).toBeDisabled({ timeout: 15_000 });
  await expect(grid).toContainText("Demo Album One", { timeout: 15_000 });

  // The row is in the store, not only in the page.
  const result = await sql(page.request, `SELECT title FROM ${table}`, false);
  expect(result.rows).toEqual([["Demo Album One"]]);

  expect(consoleErrors).toEqual([]);
});
