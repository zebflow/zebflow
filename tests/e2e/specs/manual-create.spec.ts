import { test, expect, visit } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;
const api = `/api/projects/${OWNER}/${PROJECT}`;

async function openNew(page: any, kind: string) {
  await visit(page, `${base}/pipelines/registry`);
  await page.getByRole("button", { name: "New" }).click();
  await page.locator("div").filter({ hasText: new RegExp(`^${kind}$`) }).first().click();
}

/**
 * A person creating by hand, pressing Create without touching the dropdowns.
 *
 * The create dialogs' selects carry no preset value. The shared Select used to
 * mark every option not-selected, which leaves the box blank: Create Pipeline's
 * required Trigger then refused the form ("Please select an item"). Uncontrolled
 * selects keep the browser's default, the first option.
 */
test("a pipeline created by hand with the default trigger saves", async ({ page, consoleErrors }) => {
  const name = `e2e-manual-${Date.now()}`;
  await openNew(page, "Pipeline");
  const trigger = page.locator('select[name="trigger_kind"]:visible');
  await expect(trigger).toHaveValue("webhook");

  await page.locator('input[placeholder="my-pipeline"]:visible').fill(name);
  const created = page.waitForResponse((r: any) => r.url().endsWith("/pipelines/definition") && r.request().method() === "POST");
  await page.getByRole("button", { name: "Create" }).first().click();
  expect((await created).status()).toBe(200);

  const read = await page.request.get(`${api}/pipelines/by-id?id=${name}.zf.json`);
  expect(read.status()).toBe(200);

  await page.request.delete(`${api}/pipelines/definition`, { data: { file_rel_path: `${name}.zf.json` } });
  expect(consoleErrors).toEqual([]);
});

test("the template dialog shows its default kind and creates the page starter", async ({ page, consoleErrors }) => {
  const name = `e2e-manual-page-${Date.now()}`;
  await openNew(page, "Template file");
  await expect(page.locator('select[name="kind"]:visible')).toHaveValue("page");

  await page.locator('input[placeholder="my-page"]:visible').fill(name);
  const created = page.waitForResponse((r: any) => r.url().includes("/repo/file") && r.request().method() === "PUT");
  await page.getByRole("button", { name: "Create" }).first().click();
  const answer = await (await created).json();
  // The kind picks the starter; `file_kind` is the server's label by folder.
  expect(answer.file.content).toContain('className="p-8"');

  await page.request.delete(`${api}/repo/file?path=${encodeURIComponent(answer.file.rel_path)}`);
  expect(consoleErrors).toEqual([]);
});
