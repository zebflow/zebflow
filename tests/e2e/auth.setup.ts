import { test as setup, expect } from "@playwright/test";
import { OWNER, PASSWORD, STORAGE_STATE } from "./config";

/**
 * Logs in once and saves the session cookie for every other spec. The cookie
 * value is an opaque token minted per login, so it cannot be hand-written.
 */
setup("authenticate", async ({ page }) => {
  await page.goto("/login");

  await page.locator('input[name="identifier"]').fill(OWNER);
  await page.locator('input[name="password"]').fill(PASSWORD);
  await page.locator('button[type="submit"], input[type="submit"]').first().click();

  // Landing on /home is what proves the credentials were accepted; asserting on
  // the redirect rather than on the click is deliberate.
  await expect(page).toHaveURL(/\/home/, { timeout: 30_000 });

  await page.context().storageState({ path: STORAGE_STATE });
});
