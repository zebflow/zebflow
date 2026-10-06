import { test as base, expect, Page } from "@playwright/test";

/**
 * Console noise that is not a defect. Chrome emits the autocomplete hint on any
 * password field, and a missing favicon is not a broken page.
 */
const IGNORED = [/autocomplete attributes/i, /favicon/i];

export const test = base.extend<{ consoleErrors: string[] }>({
  consoleErrors: async ({ page }, use) => {
    const errors: string[] = [];

    page.on("console", (msg) => {
      if (msg.type() !== "error") return;
      const text = msg.text();
      if (IGNORED.some((re) => re.test(text))) return;
      errors.push(`console.error: ${text}`);
    });

    // An uncaught exception kills hydration silently — it must fail the test.
    page.on("pageerror", (err) => {
      errors.push(`pageerror: ${err.message}`);
    });

    await use(errors);
  },
});

export { expect };

/**
 * Opens a Studio page and waits for the server-rendered markup to settle.
 */
export async function visit(page: Page, path: string) {
  const response = await page.goto(path);
  expect(response?.status(), `GET ${path}`).toBeLessThan(400);
  await page.waitForLoadState("domcontentloaded");
  return response;
}

/**
 * Clicks until the expected state appears.
 *
 * Zebflow pages hydrate after the server HTML arrives, so a click that lands
 * before the handler is attached is swallowed with no error at all. Asserting
 * "the click succeeded" would pass while the app did nothing; only the observed
 * state change proves the page is alive.
 */
export async function clickUntil(
  page: Page,
  selector: string,
  assertion: () => Promise<void>,
) {
  await expect(async () => {
    await page.locator(selector).first().click({ timeout: 5_000 });
    await assertion();
  }).toPass({ timeout: 30_000 });
}

/** Pixels of the element's screenshot within `tol` of the RGB colour. */
export async function countColour(page: Page, selector: string, rgb: [number, number, number], tol = 40) {
  const png = (await page.locator(selector).screenshot()).toString("base64");
  return page.evaluate(async ({ png, rgb, tol }) => {
    const img = new Image();
    img.src = `data:image/png;base64,${png}`;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.width; c.height = img.height;
    const ctx = c.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (Math.abs(d[i] - rgb[0]) < tol && Math.abs(d[i + 1] - rgb[1]) < tol && Math.abs(d[i + 2] - rgb[2]) < tol) n++;
    }
    return n;
  }, { png, rgb, tol });
}
