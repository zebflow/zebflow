import { test, expect, clickUntil } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";

/**
 * The whole path a new user walks: create a project, get the starter files,
 * turn a sample pipeline on, and see the sample page render and respond.
 *
 * This is the one spec that would catch a broken scaffold, a broken pipeline
 * activation, a broken RWE compile, or dead hydration — none of which the Rust
 * suite can see, because it only proves templates compile.
 */

const PROJECT = `e2e-lifecycle-${Date.now()}`;

test.afterAll(async ({ playwright }) => {
  // Clean up even when an assertion above failed, so a bad run leaves no
  // project behind on the shared dev server.
  const ctx = await playwright.request.newContext({
    baseURL: BASE_URL,
    storageState: STORAGE_STATE,
  });
  await ctx.delete(`/api/users/${OWNER}/projects/${PROJECT}`, {
    data: { project_name: PROJECT, password: PASSWORD },
  });
  await ctx.dispose();
});

test("a new project scaffolds a flat repository with runnable samples", async ({
  page,
  request,
  consoleErrors,
}) => {
  // --- create ------------------------------------------------------------
  const created = await request.post("/home/projects/create", {
    form: { project: PROJECT, title: "E2E Lifecycle" },
    maxRedirects: 0,
  });
  expect(created.status(), "create project").toBe(303);

  // --- the scaffold is flat ---------------------------------------------
  const repo = await request.get(`/api/projects/${OWNER}/${PROJECT}/repo`);
  expect(repo.ok(), "read repository tree").toBeTruthy();
  const items: Array<{ rel_path: string; kind: string }> = (await repo.json()).items;
  const paths = items.map((i) => i.rel_path).sort();

  expect(paths).toContain("README.md");
  expect(paths).toContain("globals.css");
  expect(paths).toContain("sample_web_page.tsx");
  expect(paths).toContain("sample_api_pipeline.zf.json");
  expect(paths).toContain("sample_web_page_pipeline.zf.json");

  // A new project is an empty repository plus samples — no scaffolded folders.
  const folders = items.filter((i) => i.kind === "folder").map((i) => i.rel_path);
  expect(folders, "a fresh project scaffolds no folders").toEqual([]);

  // --- the API sample answers once activated -----------------------------
  const dsl = (line: string) =>
    request.post(`/api/projects/${OWNER}/${PROJECT}/pipelines/dsl`, {
      data: { dsl: line },
    });

  // Inactive is the shipped state, so the webhook must 404 before activation.
  expect((await request.get(`/wh/${OWNER}/${PROJECT}/sample`)).status()).toBe(404);

  expect((await dsl("activate pipeline sample_api_pipeline.zf.json")).ok()).toBeTruthy();
  const api = await request.get(`/wh/${OWNER}/${PROJECT}/sample`);
  expect(api.status(), "sample API after activation").toBe(200);
  expect(await api.json()).toMatchObject({ method: "GET", path: "/sample" });

  // --- the web sample renders and hydrates -------------------------------
  expect((await dsl("activate pipeline sample_web_page_pipeline.zf.json")).ok()).toBeTruthy();

  await page.goto(`/wh/${OWNER}/${PROJECT}/p/sample`);
  await expect(page.getByRole("heading", { name: /Hello from Zebflow/i })).toBeVisible();

  // globals.css is imported explicitly by the page; if the compiler failed to
  // inline it, this custom property would resolve to an empty string.
  const brand = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--color-brand").trim(),
  );
  expect(brand, "globals.css reached the browser").not.toBe("");

  // Hydration: the counter only moves if the runtime attached the handler.
  const button = page.getByRole("button", { name: /Clicked/i });
  await expect(button).toHaveText(/Clicked 0 times/);
  await clickUntil(page, "button", async () => {
    await expect(button).toHaveText(/Clicked [1-9]\d* times/, { timeout: 5_000 });
  });

  expect(consoleErrors).toEqual([]);
});
