import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * Every Studio write that names a file reaches the file, in a throwaway
 * project: the file editor's Save, the registry's delete of a page, and the
 * pipeline editor's Save Draft and Activate.
 *
 * The file editor's Save still sent the body of the `/templates/file` API the
 * repository API replaced in v0.9.0 -- `{ rel_path, content }` with no
 * `?path=` -- and every save was refused. Nothing failed at build time and the
 * page still rendered, so only a browser that saves and reads the file back
 * shows it.
 */

test.describe("studio writes", () => {
  let request: APIRequestContext;
  const project = `e2e-save-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const studio = `/projects/${OWNER}/${project}/pipelines/registry`;

  const read = async (path: string) => request.get(`${api}/repo/file?path=${encodeURIComponent(path)}`);

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Studio writes" }, maxRedirects: 0 });
    expect(created.status(), `create ${project}`).toBeLessThan(400);
    const page = await request.put(`${api}/repo/file?path=delete-me.tsx`, {
      data: "export default function DeleteMe() {\n  return <div />;\n}\n",
      headers: { "Content-Type": "text/plain" },
    });
    expect(page.ok(), "seed delete-me.tsx").toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("the file editor saves what was typed", async ({ page, consoleErrors }) => {
    const marker = `edited in the browser ${Date.now()}`;
    await page.goto(`${studio}?type=file&path=${encodeURIComponent("/")}&file=README.md`);
    const editor = page.locator(".cm-content").first();
    await expect(editor).toBeVisible();
    const indicator = page.locator(".pipeline-editor-indicator").first();
    await expect(indicator).toHaveText("Saved");
    await editor.click();
    await page.keyboard.press("ControlOrMeta+End");
    await page.keyboard.type(`\n${marker}\n`);
    await expect(indicator).toHaveText("Unsaved");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(indicator).toHaveText("Saved");
    const stored = await read("README.md");
    expect(stored.ok()).toBeTruthy();
    expect((await stored.json()).file.content).toContain(marker);
    expect(consoleErrors).toEqual([]);
  });

  test("the registry deletes a page", async ({ page, consoleErrors }) => {
    await page.goto(`${studio}?path=${encodeURIComponent("/")}`);
    const row = page.locator('[data-pipeline-row][data-rel-path="delete-me.tsx"]');
    await expect(row).toBeVisible();
    const dialog = page.locator(".pipeline-delete-box");
    // A click before hydration is swallowed: retry until the dialog opens.
    await expect(async () => {
      await row.locator(".pipeline-registry-row-del").click({ timeout: 5_000 });
      await expect(dialog).toBeVisible({ timeout: 2_000 });
    }).toPass({ timeout: 30_000 });
    await dialog.locator(".pipeline-delete-input").fill("delete-me.tsx");
    await dialog.getByRole("button", { name: "Delete", exact: true }).click();
    await expect(dialog).toHaveCount(0);
    expect((await read("delete-me.tsx")).status()).toBe(404);
    expect(consoleErrors).toEqual([]);
  });

  test("the pipeline editor saves and activates", async ({ page, consoleErrors }) => {
    const file = "sample_api_pipeline.zf.json";
    await page.goto(`${studio}?type=pipeline&path=${encodeURIComponent("/")}&file=${file}`);
    const save = page.getByRole("button", { name: "Save Draft", exact: true });
    await expect(save).toBeEnabled();
    await save.click();
    await page.getByRole("button", { name: "Activate", exact: true }).click();
    await expect(async () => {
      const listed = await request.get(`${api}/pipelines?recursive=true`);
      const meta = (await listed.json()).items.find((i: any) => i.meta.file_rel_path === file)?.meta;
      expect(meta?.active_hash, "activated").toBeTruthy();
      expect(meta.active_hash).toBe(meta.hash);
    }).toPass({ timeout: 15_000 });
    expect(consoleErrors).toEqual([]);
  });
});
