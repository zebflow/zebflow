import { createHmac, randomUUID } from "node:crypto";
import { test, expect } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, STORAGE_STATE } from "../config";
import type { APIRequestContext } from "@playwright/test";

/**
 * The hmac credential form, in a throwaway project: a saved webhook
 * signature check opens with its vendor and settings filled, the test panel
 * passes a correctly signed delivery pasted into it, and choosing another
 * vendor refills the settings. The check itself is covered by
 * tests/platform/webhook_signatures.rs; this proves the form works.
 */

test.describe("hmac credential form", () => {
  let request: APIRequestContext;
  const project = `e2e-hmac-${Date.now()}`;
  const api = `/api/projects/${OWNER}/${project}`;
  // A key made for this run, never a literal in the source.
  const signingKey = `whsec_${randomUUID()}`;

  test.beforeAll(async ({ playwright }) => {
    request = await playwright.request.newContext({ baseURL: BASE_URL, storageState: STORAGE_STATE });
    const created = await request.post("/home/projects/create", { form: { project, title: "HMAC form" }, maxRedirects: 0 });
    expect(created.status(), `create ${project}`).toBeLessThan(400);
    const presets = await (await request.get(`${api}/credential-types/hmac/presets`)).json();
    const stripe = presets.presets.find((p: any) => p.id === "stripe");
    const saved = await request.post(`${api}/credentials`, {
      data: { credential_id: "stripe-hooks", title: "Stripe hooks", kind: "hmac", secret: { ...stripe.settings, preset: "stripe", secrets: { current: signingKey } } },
    });
    expect(saved.ok(), await saved.text()).toBeTruthy();
  });

  test.afterAll(async () => {
    const removed = await request.delete(`/api/users/${OWNER}/projects/${project}`, { data: { project_name: project, password: PASSWORD } });
    expect(removed.ok(), `remove ${project}`).toBeTruthy();
    await request.dispose();
  });

  test("a saved check opens filled, passes a signed delivery, and a vendor refills it", async ({ page, consoleErrors }) => {
    await page.goto(`/projects/${OWNER}/${project}/credentials`);
    const row = page.locator("tr", { hasText: "stripe-hooks" });
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "Edit" }).click();

    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("Webhook signature check")).toBeVisible();
    const vendor = dialog.locator("select").filter({ has: page.locator('option[value="stripe"]') });
    await expect(vendor).toHaveValue("stripe");
    const signatureFrom = dialog.locator('input[placeholder="header:X-Vendor-Signature"]');
    await expect(signatureFrom).toHaveValue("part:v1");
    await expect(dialog.getByText("Test with a delivery")).toBeVisible();

    // A delivery signed the Stripe way, pasted as the dashboard shows it.
    const body = '{"id":"evt_e2e"}';
    const t = "1790000000";
    const v1 = createHmac("sha256", signingKey).update(`${t}.${body}`).digest("hex");
    const areas = dialog.locator("textarea");
    await areas.nth(0).fill(`Stripe-Signature: t=${t},v1=${v1}`);
    await areas.nth(1).fill(body);
    await dialog.getByRole("button", { name: "Check" }).click();
    await expect(dialog.getByText("Passes: this delivery matches the saved check.")).toBeVisible();

    await areas.nth(1).fill('{"id":"evt_other"}');
    await dialog.getByRole("button", { name: "Check" }).click();
    await expect(dialog.getByText(/HMAC_SIGNATURE_MISMATCH/)).toBeVisible();

    // Another vendor refills what is signed and where the signature is.
    await vendor.selectOption("github");
    await expect(signatureFrom).toHaveValue("header:X-Hub-Signature-256");
    await expect(dialog.locator('input[placeholder="{timestamp}.{body}"]')).toHaveValue("{body}");
    expect(consoleErrors).toEqual([]);
  });
});
