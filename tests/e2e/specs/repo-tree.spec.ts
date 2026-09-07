import { test, expect, visit } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;

async function openTree(page: any) {
  await visit(page, `${base}/pipelines/registry`);
  await expect(page.locator("[data-repo-tree]")).toBeVisible({ timeout: 15_000 });
}

/** The ⋯ belonging to one row, found by that row's path. */
function menuButtonFor(page: any, path: string) {
  return page
    .locator(`[data-repo-tree-row="${path}"]`)
    .locator("xpath=following-sibling::button[@data-context-menu-trigger]");
}

/**
 * The tree opens one folder at a time.
 *
 * What this guards is the cost model, not the drawing: expanding asks for that
 * folder, and reopening asks for nothing. The screen looks identical either
 * way, which is why it needs a test.
 */
test("expanding a folder fetches only that folder, and only once", async ({ page, consoleErrors }) => {
  const treeCalls: string[] = [];
  await page.route("**/api/projects/*/*/repo?*", async (route) => {
    treeCalls.push(new URL(route.request().url()).search);
    await route.continue();
  });

  await openTree(page);
  await expect
    .poll(() => treeCalls.some((s) => s.includes("depth=1")), { timeout: 15_000 })
    .toBe(true);
  expect(
    treeCalls.every((s) => s.includes("depth=1") || s.includes("fields=path")),
    `no unscoped tree read: ${treeCalls.join(" | ")}`,
  ).toBe(true);

  const folder = page.locator('[data-repo-tree-row="pipelines"]');
  test.skip(!(await folder.count()), "this project has no folder to open");

  const before = treeCalls.length;
  await folder.click();
  await expect.poll(() => treeCalls.length, { timeout: 10_000 }).toBeGreaterThan(before);
  expect(treeCalls[treeCalls.length - 1]).toContain("path=pipelines");

  const afterOpen = treeCalls.length;
  await folder.click();
  await folder.click();
  await page.waitForTimeout(500);
  expect(treeCalls.length, "a reopen must not refetch").toBe(afterOpen);

  expect(consoleErrors).toEqual([]);
});

/**
 * The repository itself is a row.
 *
 * Without it there is nowhere to hang "create at the root" — the gap that made
 * the first version of this tree unusable for the one thing people do most.
 */
test("the root is a row with its own actions", async ({ page, consoleErrors }) => {
  await openTree(page);

  const root = page.locator('[data-repo-tree-kind="root"]');
  await expect(root).toHaveCount(1);

  await menuButtonFor(page, "").click();
  const menu = page.locator("[data-context-menu]");
  await expect(menu).toBeVisible({ timeout: 5_000 });
  await expect(menu).toContainText("New file");
  await expect(menu).toContainText("New folder");
  await expect(menu).toContainText("Collapse all");

  expect(consoleErrors).toEqual([]);
});

/**
 * Every row carries the same ⋯, and it offers what applies to that row.
 *
 * A folder is where things are made; a file is a thing. Offering "New file
 * here" on a file would create somewhere the reader never pointed at.
 */
test("every row has a menu button offering what fits that row", async ({ page, consoleErrors }) => {
  await openTree(page);

  const rows = await page.locator("[data-repo-tree-row]").count();
  const triggers = await page.locator("[data-context-menu-trigger]").count();
  expect(triggers, "one menu button per row").toBe(rows);

  await menuButtonFor(page, "pipelines").click();
  const folderMenu = page.locator("[data-context-menu]");
  await expect(folderMenu).toContainText("New file here");
  await expect(folderMenu).toContainText("Delete folder");
  await page.locator("[data-context-menu-backdrop]").click();

  await menuButtonFor(page, "README.md").click();
  const fileMenu = page.locator("[data-context-menu]");
  await expect(fileMenu).toContainText("Duplicate");
  await expect(fileMenu).not.toContainText("New file here");
  await page.locator("[data-context-menu-backdrop]").click();

  expect(consoleErrors).toEqual([]);
});

/**
 * Clicking a folder says where the next thing goes.
 *
 * Selection is the answer to "create it *where*" for the toolbar's New button,
 * so exactly one row is ever current.
 */
test("clicking a folder selects it and opens it", async ({ page, consoleErrors }) => {
  await openTree(page);

  const folder = page.locator('[data-repo-tree-row="schemas"]');
  test.skip(!(await folder.count()), "this project has no folder to select");

  await folder.click();
  await expect(page.locator("[data-selected]")).toHaveCount(1);
  await expect(folder).toHaveAttribute("data-selected", "true");
  // Opening it shows what is inside, so a new file there will be visible.
  await expect(page.locator('[data-repo-tree-row^="schemas/"]').first()).toBeVisible({
    timeout: 10_000,
  });

  expect(consoleErrors).toEqual([]);
});

/**
 * Creating from a folder's menu must aim at that folder, not at whatever the
 * page happened to be scoped to.
 */
test("creating from a folder menu targets that folder", async ({ page, consoleErrors }) => {
  await openTree(page);

  const folder = page.locator('[data-repo-tree-row="pipelines"]');
  test.skip(!(await folder.count()), "this project has no folder to create in");

  await menuButtonFor(page, "pipelines").click();
  await page.getByText("New file here", { exact: true }).click();

  await expect(folder).toHaveAttribute("data-selected", "true");
  await expect(page.locator("dialog[open]")).toBeVisible({ timeout: 5_000 });

  expect(consoleErrors).toEqual([]);
});

/**
 * Opening a folder shows what is in it.
 *
 * The panel beside the tree already lists a folder's folders, pipelines and
 * files — that is what the root shows on arrival. Opening a folder is the same
 * view pointed one level down, so there is one way to look at a folder rather
 * than two.
 */
test("open folder shows that folder's contents in the panel", async ({ page, consoleErrors }) => {
  await openTree(page);

  const folder = page.locator('[data-repo-tree-row="pipelines"]');
  test.skip(!(await folder.count()), "this project has no folder to open");

  await menuButtonFor(page, "pipelines").click();
  await page.getByText("Open folder", { exact: true }).click();

  await expect(page).toHaveURL(/[?&]path=%2Fpipelines/, { timeout: 10_000 });
  // The panel names the folder it is listing, not the repository root.
  await expect(page.getByText("/pipelines", { exact: true }).first()).toBeVisible({
    timeout: 10_000,
  });

  expect(consoleErrors).toEqual([]);
});

/** Adding from the hub is offered where things are made, and nowhere else. */
test("add from hub is offered on folders and the root, not on files", async ({ page, consoleErrors }) => {
  await openTree(page);

  await menuButtonFor(page, "").click();
  await expect(page.locator("[data-context-menu]")).toContainText("Add from hub…");
  await page.locator("[data-context-menu-backdrop]").click();

  await menuButtonFor(page, "pipelines").click();
  await expect(page.locator("[data-context-menu]")).toContainText("Add from hub…");
  await page.locator("[data-context-menu-backdrop]").click();

  await menuButtonFor(page, "README.md").click();
  await expect(page.locator("[data-context-menu]")).not.toContainText("Add from hub…");
  await page.locator("[data-context-menu-backdrop]").click();

  expect(consoleErrors).toEqual([]);
});
