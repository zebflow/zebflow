import { test, expect, visit } from "../fixtures";
import { OWNER, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;

/** The detail pane's action button, whatever verb it currently shows. */
const action = (page: any, label: RegExp) =>
  page.getByRole("button", { name: label, exact: true });

/**
 * Open the shelf showing everything, and select an installable library.
 *
 * Discovered rather than named. The state filter defaults to "Available", so a
 * hard-coded package disappears the moment a previous run leaves it installed —
 * which is how the first version of this spec passed on one machine and failed
 * on the next. "All" plus the Libraries chip is a question about the shelf, not
 * about what happened here yesterday.
 */
async function openALibrary(page: any, packageId?: string) {
  await visit(page, `${base}/hub`);
  await page.getByRole("button", { name: "All", exact: true }).click();
  await page.getByRole("button", { name: "Libraries", exact: true }).click();

  // Re-opening a named package rather than "whichever is first". Installing one
  // moves it between state filters, so `first()` after a reload is a different
  // library and the assertion that follows is about the wrong row.
  //
  // The state has to be hunted for: "All" is a *kind* chip, not a state one —
  // the states are In project / Available / Updatable / Problems and there is
  // no "everything" among them. An installed package is simply not under
  // Available, which is where the page opens.
  if (packageId) {
    const named = page.locator(`[data-hub-item="${packageId}"]`);
    for (const state of ["In project", "Available", "Updatable", "Problems"]) {
      await page.getByRole("button", { name: state, exact: true }).click();
      if (await named.count()) break;
    }
    await expect(named).toBeVisible({ timeout: 20_000 });
    await named.click();
    await expect(action(page, /^(Install|Reinstall|Update)$/).first()).toBeVisible({
      timeout: 20_000,
    });
    return packageId;
  }

  // `data-hub-item` and `data-hub-kind` are the row's own hooks. Matching
  // rendered text failed here: the label starts with a kind glyph, so `^zeb/`
  // never matched and all three tests skipped — silently, which is worse than
  // failing.
  const rows = page.locator('[data-hub-kind="rwe_library"][data-hub-item]');
  const count = await rows.count();
  test.skip(count === 0, "this build carries no rwe libraries");
  const row = rows.first();
  const name = (await row.getAttribute("data-hub-item")) || "";
  await row.click();
  // The detail pane is what every assertion below reads; returning before it
  // has rendered is what made this spec race.
  await expect(action(page, /^(Install|Reinstall|Update)$/).first()).toBeVisible({
    timeout: 20_000,
  });
  return name;
}

/**
 * Leave the project as the spec found it.
 *
 * Waits for the pane to settle before deciding. An instant `isVisible` check
 * raced the render: it saw no Remove on a package that was in fact installed,
 * clicked Install, got the overwrite dialog instead, and then waited for a
 * button that a modal was covering. That failed one run in three and passed the
 * rest, which is the worst kind of test.
 */
async function ensureNotInstalled(page: any) {
  await expect(
    action(page, /^(Install|Reinstall|Update)$/).first(),
  ).toBeVisible({ timeout: 20_000 });

  if (await action(page, /^Remove$/).isVisible()) {
    await action(page, /^Remove$/).click();
    await page.getByRole("button", { name: "Remove it" }).click();
    await expect(action(page, /^Install$/)).toBeVisible({ timeout: 25_000 });
  }
  await expect(action(page, /^Remove$/)).toHaveCount(0);
}

test("the hub lists packages and says what each verb costs", async ({ page, consoleErrors }) => {
  const name = await openALibrary(page);
  expect(name).toMatch(/^zebflow\./);

  // The pane explains the verb rather than only naming it: `install` and `add`
  // differ in whether the project can ever undo them.
  await expect(page.getByText(/Installing keeps this managed/i)).toBeVisible();
  expect(await page.content()).not.toContain("RWE component error");
  expect(consoleErrors).toEqual([]);
});

/**
 * Install, then remove, in one pass.
 *
 * Both directions were broken in ways only a click revealed: the install sent a
 * version that does not exist, and afterwards the row never learned it had been
 * installed — so Remove, which had just become available, appeared to be
 * missing.
 */
test("a library can be installed and removed again from the browser", async ({
  page,
  consoleErrors,
}) => {
  const packageId = await openALibrary(page);
  await ensureNotInstalled(page);

  await action(page, /^Install$/).click();

  // The row learns it is installed without a reload. Both of these come from
  // re-reading the project, not the catalogue.
  await expect(action(page, /^Reinstall$/)).toBeVisible({ timeout: 25_000 });
  await expect(action(page, /^Remove$/)).toBeVisible();

  // And it still knows after a reload, when the state comes from the
  // server-rendered payload rather than a refetch. Re-opening the same package
  // by name: installing changed where it sorts.
  await openALibrary(page, packageId);
  await expect(action(page, /^Remove$/)).toBeVisible();

  // Removing deletes files, so it asks first.
  await action(page, /^Remove$/).click();
  await expect(page.getByText(/Remove this package from the project\?/i)).toBeVisible();
  await page.getByRole("button", { name: "Remove it" }).click();

  await expect(action(page, /^Install$/)).toBeVisible({ timeout: 25_000 });
  await expect(action(page, /^Remove$/)).toHaveCount(0);

  expect(await page.content()).not.toContain("RWE component error");
  expect(consoleErrors).toEqual([]);
});

/**
 * Writing over a file that already exists is the one step that cannot be
 * undone, so it is the one step that stops and asks — and it must not write
 * first.
 */
test("re-adding an installed package asks before it overwrites", async ({ page }) => {
  await openALibrary(page);
  await ensureNotInstalled(page);

  await action(page, /^Install$/).click();
  await expect(action(page, /^Reinstall$/)).toBeVisible({ timeout: 25_000 });

  await action(page, /^Reinstall$/).click();
  await expect(page.getByText(/Replace files already in this project\?/i)).toBeVisible({
    timeout: 20_000,
  });
  // Every file it would replace is named, not counted.
  await expect(page.getByText(/rwe-libraries\//).first()).toBeVisible();

  // Declining leaves the project alone.
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.getByText(/Replace files already in this project\?/i)).toHaveCount(0);
  await expect(action(page, /^Remove$/)).toBeVisible();

  await ensureNotInstalled(page);
});
