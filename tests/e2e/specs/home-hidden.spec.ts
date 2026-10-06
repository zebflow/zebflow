import { test, expect, clickUntil, visit } from "../fixtures";
import { OWNER, PASSWORD } from "../config";

/**
 * Hide from home (`project.md` § Home listing): ticked in the project's
 * settings, the card leaves the home list and waits under Hidden projects;
 * shown again from that card's menu, it is back on the list.
 */
test("a project hidden in settings leaves home, waits under Hidden projects, and comes back", async ({ page, request, consoleErrors }) => {
  const project = `e2e-hidden-${Date.now()}`;
  const href = `/projects/${OWNER}/${project}`;
  const listed = () => page.locator(`section:not([data-hidden-projects]) a[href="${href}"]`);
  const hiddenSection = page.locator("[data-hidden-projects]");
  try {
    const created = await request.post("/home/projects/create", { form: { project, title: "Hidden Probe" }, maxRedirects: 0 });
    expect(created.status()).toBe(303);

    await visit(page, "/home");
    await expect(listed()).toHaveCount(1);

    // Hide it in Settings → General.
    await visit(page, `${href}/settings`);
    const box = page.locator("[data-home-visibility] input[type=checkbox]");
    await expect(box).not.toBeChecked();
    await clickUntil(page, "[data-home-visibility] input[type=checkbox]", async () => {
      await expect(page.locator("[data-home-visibility]")).toContainText("Hidden from home.", { timeout: 3000 });
    });
    await expect(box).toBeChecked();
    const stored = await (await request.get(`/api/projects/${OWNER}/${project}/settings/home`)).json();
    expect(stored.data.hidden).toBe(true);

    // Off the list; under Hidden projects once opened.
    await visit(page, "/home");
    await expect(listed()).toHaveCount(0);
    await expect(hiddenSection).toBeVisible();
    await clickUntil(page, "[data-hidden-projects] button[aria-expanded]", async () => {
      await expect(hiddenSection.locator(`a[href="${href}"]`)).toBeVisible({ timeout: 3000 });
    });

    // Reachable by address all along.
    await visit(page, href);
    await visit(page, "/home");

    // Shown again from the card's menu in the hidden section.
    await clickUntil(page, "[data-hidden-projects] button[aria-expanded]", async () => {
      await expect(hiddenSection.locator(`a[href="${href}"]`)).toBeVisible({ timeout: 3000 });
    });
    const row = hiddenSection.locator("div.flex.items-center.justify-between").filter({ has: page.locator(`a[href="${href}"]`) });
    await row.getByRole("button", { name: "Project menu" }).click();
    // The home list is the server's; the menu reloads the page to show it.
    await Promise.all([
      page.waitForEvent("load"),
      page.getByText("Show on home", { exact: true }).click(),
    ]);
    await expect(listed()).toHaveCount(1);
    const shown = await (await request.get(`/api/projects/${OWNER}/${project}/settings/home`)).json();
    expect(shown.data.hidden).toBe(false);
    expect(consoleErrors).toEqual([]);
  } finally {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, {
      data: { project_name: project, password: PASSWORD },
    });
    expect(removed.ok(), "remove hidden-project test project").toBeTruthy();
  }
});
