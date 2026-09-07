import { test, expect, visit, clickUntil } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;

/**
 * Every Studio surface, with the heading that proves the page actually
 * rendered its own content rather than an empty shell or an error card.
 */
const PAGES = [
  { name: "dashboard", path: `${base}/dashboard`, title: /Dashboard/i },
  { name: "pipelines registry", path: `${base}/pipelines/registry?path=/`, title: /Pipelines/i },
  { name: "database connections", path: `${base}/db/connections`, title: /Databases/i },
  { name: "credentials", path: `${base}/credentials`, title: /Credentials/i },
  { name: "hub", path: `${base}/hub`, title: /Hub/i },
  { name: "settings", path: `${base}/settings`, title: /Settings/i },
  { name: "files", path: `${base}/files`, title: /Files/i },
  { name: "editor", path: `${base}/editor`, title: /Editor/i },
  { name: "infrastructure", path: `${base}/infrastructure`, title: /Infrastructure/i },
];

for (const p of PAGES) {
  test(`${p.name} renders without console errors`, async ({ page, consoleErrors }) => {
    await visit(page, p.path);

    await expect(page).toHaveTitle(p.title);
    // A page that rendered has visible body text; a blank hydration failure
    // leaves the shell with nothing in it.
    await expect(page.locator("body")).not.toBeEmpty();

    expect(consoleErrors, `console errors on ${p.path}`).toEqual([]);
  });
}

test("infrastructure reports the office topology", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/infrastructure`);

  // Offices and runtime placement are the two panels this page exists for.
  await expect(page.getByText("Offices", { exact: false }).first()).toBeVisible();
  await expect(page.getByText(/Project runtime/i).first()).toBeVisible();

  expect(consoleErrors).toEqual([]);
});

test("settings tabs navigate", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/settings`);

  // Asserting on the resulting URL, not on the click: a click that lands before
  // hydration reports success while doing nothing at all.
  await clickUntil(page, 'a[href$="/settings/policy"]', async () => {
    await expect(page).toHaveURL(/\/settings\/policy$/, { timeout: 5_000 });
  });

  expect(consoleErrors).toEqual([]);
});
