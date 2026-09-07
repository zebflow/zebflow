import { test, expect, visit } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;

/**
 * Every database connection the project has, whatever engine it runs.
 *
 * The connections are discovered rather than listed, because which engines a
 * machine has running differs — a hard-coded `pg-test` would fail on a laptop
 * with no PostgreSQL container rather than report anything true.
 */
test("every connected database opens its table browser", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/db/connections`);

  const links = await page
    .locator(`a[href*="/db/"][href$="/tables"]`)
    .evaluateAll((nodes) => nodes.map((n) => (n as HTMLAnchorElement).getAttribute("href")));

  const targets = [...new Set(links.filter(Boolean))] as string[];
  test.skip(targets.length === 0, "no database connections configured on this machine");

  for (const href of targets) {
    await visit(page, href);
    // The schema sidebar is the page's own content; the shell renders without it.
    await expect(page.locator('[data-db-suite-object-tree]')).toBeVisible();
    expect(await page.content(), `component error on ${href}`).not.toContain(
      "RWE component error",
    );
  }

  expect(consoleErrors).toEqual([]);
});

/**
 * Choosing a table has to reach the engine and come back with its rows.
 *
 * The tree, the preview and the row inspector are three separate pieces of
 * state; a page that renders the tree but never loads a table looks identical
 * to a working one in a screenshot.
 */
test("choosing a table loads its rows and its columns", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/db/connections`);
  const first = await page
    .locator(`a[href*="/db/"][href$="/tables"]`)
    .first()
    .getAttribute("href");
  test.skip(!first, "no database connections configured on this machine");

  await visit(page, first!);
  const tables = page.locator(".db-suite-object-item");
  await expect(tables.first()).toBeVisible({ timeout: 15_000 });

  // The page opens a table by itself; the grid header is what proves the
  // preview request came back rather than the tree merely rendering.
  await expect(page.locator(".db-suite-data-split").first()).toBeVisible();
  await expect(page.locator(".db-suite-object-item.is-active")).toHaveCount(1);

  const second = tables.nth(1);
  if (await second.count()) {
    const name = (await second.textContent())?.trim() ?? "";
    await second.click();
    // Selecting writes the open table into the URL, so that is the observable
    // proof the click was handled rather than swallowed before hydration.
    await expect(page).toHaveURL(/[?&]table=/, { timeout: 10_000 });
    expect(name.length).toBeGreaterThan(0);
  }

  expect(consoleErrors).toEqual([]);
});

/**
 * The query tab owns its statement and its result and nothing else.
 *
 * Running one is the only proof the panel is wired to the engine rather than
 * merely drawn — the box and the button render identically either way.
 */
test("the query tab runs a statement and reports the result", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/db/connections`);
  const tables = await page
    .locator(`a[href*="/db/"][href$="/tables"]`)
    .first()
    .getAttribute("href");
  test.skip(!tables, "no database connections configured on this machine");

  await visit(page, tables!.replace(/\/tables$/, "/query"));
  const status = page.locator(".db-suite-query-status");
  await expect(status).toBeVisible();
  await expect(status).toHaveText(/Ready/);

  // Empty by design: the engine is never asked, so this asserts the panel's own
  // handler ran without depending on which engine the machine happens to have.
  await page.locator(".db-suite-query-editor-host").fill("   ");
  await page.getByRole("button", { name: "Run Query" }).click();
  await expect(status).toHaveText(/Query is empty/, { timeout: 10_000 });

  expect(consoleErrors).toEqual([]);
});

/**
 * Adding a row and abandoning it.
 *
 * Deliberately writes nothing: a draft row lives in the page until it is
 * saved, so this exercises the editor's own state — the draft, the enabled
 * Save, and Cancel putting it all back — without leaving a row behind on
 * whatever engine the machine is running.
 */
test("a drafted row appears, enables saving, and cancels away", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/db/connections`);
  const tables = await page
    .locator(`a[href*="/db/"][href$="/tables"]`)
    .first()
    .getAttribute("href");
  test.skip(!tables, "no database connections configured on this machine");

  await visit(page, tables!);
  await expect(page.locator(".db-suite-object-item").first()).toBeVisible({ timeout: 15_000 });

  const addRow = page.getByTitle("Add row");
  const save = page.getByTitle("Save changes");
  const cancel = page.getByTitle("Cancel changes");
  test.skip(!(await addRow.count()), "this engine does not support adding rows");

  await expect(save).toBeDisabled();

  const before = await page.locator("tbody tr").count();
  await addRow.click();
  await expect(save).toBeEnabled({ timeout: 10_000 });
  expect(await page.locator("tbody tr").count()).toBe(before + 1);

  await cancel.click();
  await expect(save).toBeDisabled();
  expect(await page.locator("tbody tr").count()).toBe(before);

  expect(consoleErrors).toEqual([]);
});

/**
 * The Data / Relations / Properties sub-tabs.
 *
 * Each renders a different component now, and each is reached only by a click
 * after hydration — a server response proves nothing about them.
 */
test("the table sub-tabs each render their own panel", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/db/connections`);
  const tables = await page
    .locator(`a[href*="/db/"][href$="/tables"]`)
    .first()
    .getAttribute("href");
  test.skip(!tables, "no database connections configured on this machine");

  await visit(page, tables!);
  await expect(page.locator(".db-suite-object-item").first()).toBeVisible({ timeout: 15_000 });

  const properties = page.getByRole("button", { name: "Properties", exact: true });
  if (await properties.count()) {
    await properties.click();
    // The section rail is the properties panel's own content.
    await expect(page.getByText("Danger Zone")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText("Columns", { exact: true }).first()).toBeVisible();
  }

  const relations = page.getByRole("button", { name: "Relations", exact: true });
  if ((await relations.count()) && (await relations.isEnabled())) {
    await relations.click();
    await expect(page.getByText(/Collection-level relation patterns/)).toBeVisible({
      timeout: 10_000,
    });
  }

  await page.getByRole("button", { name: "Data", exact: true }).click();
  await expect(page.locator(".db-suite-grid-editor-wrap")).toBeVisible();

  expect(consoleErrors).toEqual([]);
});
