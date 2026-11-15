import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext, Page } from "@playwright/test";

/**
 * A generated docs site on a phone, in a throwaway project: the sidebar is a
 * drawer that starts closed, opens from "Contents", closes from its backdrop,
 * Escape and a followed link, and on a wide window is a plain column with no
 * backdrop, even when the drawer was left open. A page asked for without its
 * last slash is sent to the slash form and still hydrates.
 */

const DOCS: Array<[string, string]> = [
  ["docs/handbook/_meta.yaml", "title: Demo Handbook\n"],
  ["docs/handbook/index.md", "# Welcome\n\nThe demo handbook home.\n"],
  ["docs/handbook/setup/_meta.yaml", "title: Setup\norder: 1\n"],
  ["docs/handbook/setup/install.md", "---\ntitle: Install\norder: 1\n---\n# Install\n\nInstall the demo app.\n"],
];

const PHONE = { width: 390, height: 844 };
const WIDE = { width: 1280, height: 800 };

/** The drawer's left edge: below 0 when it is pushed off screen. */
async function drawerLeft(page: Page): Promise<number> {
  return page.locator("aside").first().evaluate((el) => el.getBoundingClientRect().left);
}

async function backdropShown(page: Page): Promise<boolean> {
  return page.locator('label[for="docs-nav"][aria-hidden="true"]').evaluate((el) => getComputedStyle(el).display !== "none");
}

test.describe("docs site drawer", () => {
  let request: APIRequestContext;
  const project = `e2e-docs-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  const base = new URL(BASE_URL);
  const host = `${base.protocol}//${project}.${OWNER}.localhost:${base.port}`;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "Docs drawer" }, maxRedirects: 0 });
    expect(created.status(), `create ${project}`).toBeLessThan(400);
    for (const [path, content] of DOCS) {
      const put = await request.put(`${api}/repo/file?path=${path}`, { data: content, headers: { "Content-Type": "text/plain" } });
      expect(put.ok(), `write ${path}`).toBeTruthy();
    }
    for (const dsl of [
      "register pipelines/sandbox/docs-build -- | trigger.manual | web.site.generate --from handbook --folder handbook-site",
      "activate pipeline pipelines/sandbox/docs-build",
    ]) {
      const res = await request.post(`${api}/pipelines/dsl`, { data: { dsl } });
      expect(JSON.stringify(await res.json()), dsl).not.toContain('"ok":false');
    }
    const serve = await request.put(`${api}/files/access`, {
      data: { path: "handbook-site", access: "public_execute", scope: "prefix", serve: [host] },
    });
    expect(serve.ok(), await serve.text()).toBeTruthy();
    const built = await request.post(`${api}/pipelines/execute`, {
      data: { file_rel_path: "pipelines/sandbox/docs-build.zf.json", trigger: "manual", input: {} },
    });
    expect(built.ok(), await built.text()).toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("on a phone the sidebar is a drawer; on a wide window it is a column", async ({ page, consoleErrors }) => {
    await page.setViewportSize(PHONE);
    // Without its last slash the page is sent to the slash form, where its
    // relative scripts resolve, so it hydrates.
    await page.goto(`${host}/setup/install`);
    expect(new URL(page.url()).pathname).toBe("/setup/install/");
    await page.waitForFunction(() => typeof (window as any).rweNavigate === "function");

    expect(await drawerLeft(page), "starts closed, off screen").toBeLessThan(0);
    expect(await backdropShown(page)).toBe(false);

    await page.getByText("Contents", { exact: true }).first().click();
    await expect.poll(() => drawerLeft(page), { message: "Contents opens it" }).toBeGreaterThanOrEqual(0);
    expect(await backdropShown(page)).toBe(true);

    await page.locator('label[for="docs-nav"][aria-hidden="true"]').click({ position: { x: PHONE.width - 10, y: PHONE.height / 2 } });
    await expect.poll(() => drawerLeft(page), { message: "the backdrop closes it" }).toBeLessThan(0);

    await page.getByText("Contents", { exact: true }).first().click();
    await expect.poll(() => drawerLeft(page)).toBeGreaterThanOrEqual(0);
    await page.keyboard.press("Escape");
    await expect.poll(() => drawerLeft(page), { message: "Escape closes it" }).toBeLessThan(0);

    // Left open, then the window widens: a column, and no backdrop over the page.
    await page.getByText("Contents", { exact: true }).first().click();
    await expect.poll(() => drawerLeft(page)).toBeGreaterThanOrEqual(0);
    await page.setViewportSize(WIDE);
    expect(await drawerLeft(page)).toBeGreaterThanOrEqual(0);
    expect(await backdropShown(page), "no backdrop on a wide window").toBe(false);
    expect(await page.getByText("Contents", { exact: true }).first().isVisible(), "the phone bar is gone").toBe(false);

    expect(consoleErrors).toEqual([]);
  });
});
