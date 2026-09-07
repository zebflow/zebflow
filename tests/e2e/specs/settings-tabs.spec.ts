import { test, expect, visit } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}/settings`;

/**
 * Each settings tab and the panel that proves it rendered its own content.
 *
 * The page is composed of ~20 panels in sibling files. A panel that fails
 * leaves the rest of the tab intact and reports only a console error and a
 * missing block, so asserting on the tab's own text is what catches it.
 */
const TABS = [
  { name: "general", path: base, shows: /Project Export/i },
  { name: "git", path: `${base}/git`, shows: /Git Remote/i },
  { name: "policy", path: `${base}/policy`, shows: /Reactive Web Engine/i },
  { name: "logs", path: `${base}/logs`, shows: /Pipeline Logging/i },
  { name: "automatons", path: `${base}/automatons`, shows: /Assistant/i },
];

/**
 * Libraries, nodes and dependencies moved to the Hub — the place they are
 * installed from. Settings kept signposts, and the old addresses redirect
 * rather than quietly rendering General.
 */
const MOVED_TO_HUB = [
  { name: "libraries", shows: /zeb\/codemirror/ },
  { name: "nodes", shows: /Node/i },
  { name: "dependencies", shows: /Dependency Lock/i },
];

for (const tab of TABS) {
  test(`settings ${tab.name} renders its panels`, async ({ page, consoleErrors }) => {
    await visit(page, tab.path);

    await expect(page.getByText(tab.shows).first()).toBeVisible();
    // A panel that threw is replaced by this comment in the server markup and
    // by a gap on screen; neither shows up as a bad status code.
    expect(await page.content()).not.toContain("RWE component error");

    expect(consoleErrors, `console errors on ${tab.path}`).toEqual([]);
  });
}

test("the general tab shows every panel it composes", async ({ page }) => {
  await visit(page, base);

  for (const heading of [/Project Export/i, /Runtime/i, /Danger/i]) {
    await expect(page.getByText(heading).first()).toBeVisible();
  }
});

/**
 * Git is its own subject.
 *
 * It used to be three sections buried at the bottom of a ten-section General
 * tab. The `GIT` button in the studio chrome is the quick view; its `settings`
 * link is how you reach the configuration, and it must land on the tab that
 * actually holds it.
 */
test("git has its own tab, and the chrome links to it", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/git`);
  for (const heading of [/Repository Health/i, /Git Remote/i, /Git Branches/i]) {
    await expect(page.getByText(heading).first()).toBeVisible();
  }

  // General must no longer carry them.
  await visit(page, base);
  await expect(page.getByText(/Git Remote/i)).toHaveCount(0);

  expect(consoleErrors).toEqual([]);
});

for (const moved of MOVED_TO_HUB) {
  test(`${moved.name} lives in the hub now`, async ({ page, consoleErrors }) => {
    await visit(page, `/projects/${OWNER}/${PROJECT}/hub/${moved.name}`);
    await expect(page.getByText(moved.shows).first()).toBeVisible();
    expect(await page.content()).not.toContain("RWE component error");
    expect(consoleErrors, `console errors on hub/${moved.name}`).toEqual([]);
  });

  test(`the old settings/${moved.name} address redirects to the hub`, async ({ page }) => {
    await page.goto(`/projects/${OWNER}/${PROJECT}/settings/${moved.name}`);
    // Landing on General with no explanation would be worse than either a
    // redirect or a refusal, so assert we actually arrive at the new home.
    await expect(page).toHaveURL(new RegExp(`/hub/${moved.name}$`), { timeout: 10_000 });
  });
}

/**
 * The storage backend moved to the Files page.
 *
 * It was read-only in Settings — a fact you could see and not act on. It now
 * sits with the files it describes, and shows the configured backend rather
 * than the hardcoded string the storages table used to print.
 */
test("the storage backend is configured with the files", async ({ page, consoleErrors }) => {
  await visit(page, `/projects/${OWNER}/${PROJECT}/files`);
  await expect(page.getByText("File Storage Backend")).toBeVisible();
  expect(consoleErrors).toEqual([]);
});

test("the old settings/files address redirects to the files page", async ({ page }) => {
  await page.goto(`/projects/${OWNER}/${PROJECT}/settings/files`);
  await expect(page).toHaveURL(new RegExp(`/projects/${OWNER}/${PROJECT}/files$`), {
    timeout: 10_000,
  });
});

/** Settings still says where the moved things went. */
test("general signposts the configuration that lives elsewhere", async ({ page }) => {
  await visit(page, `/projects/${OWNER}/${PROJECT}/settings`);
  // Scoped to the block itself: "Databases" also appears in the collapsed
  // sidebar nav, and matching loose text finds that hidden copy instead.
  const signposts = page.locator("[data-settings-signposts]");
  await expect(signposts).toBeVisible();
  for (const label of ["Libraries", "Nodes", "Dependencies", "File storage", "Databases"]) {
    await expect(signposts.getByText(label, { exact: true })).toBeVisible();
  }
});
