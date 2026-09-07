import { test, expect, visit } from "../fixtures";
import { BASE_URL, OWNER, PASSWORD, PROJECT } from "../config";

const base = `/projects/${OWNER}/${PROJECT}`;

/**
 * A second person, created for this spec and reused across its runs.
 *
 * Zebflow has no route to delete a user, so a fresh name per run would leave a
 * new account on the instance every time. One stable name, created if absent,
 * is the honest trade until that route exists.
 */
const GUEST = "e2e-member";
const GUEST_PASSWORD = "e2e-member-password-1";

/** Create the guest account if this instance does not have it yet. */
async function ensureGuestExists(request: any) {
  const response = await request.post(`${BASE_URL}/api/users`, {
    data: {
      owner: GUEST,
      password: GUEST_PASSWORD,
      role: "user",
      git_name: "E2E Member",
      git_email: "e2e-member@example.invalid",
    },
  });
  // 409 means a previous run already made them, which is the intended state.
  expect([200, 409]).toContain(response.status());
}

/** Remove the guest from the project, however the last run left things. */
async function ensureNotAMember(request: any) {
  await request.delete(
    `${BASE_URL}/api/projects/${OWNER}/${PROJECT}/members/${GUEST}`,
  );
  const invites = await request.get(
    `${BASE_URL}/api/projects/${OWNER}/${PROJECT}/invites`,
  );
  const body = await invites.json().catch(() => ({}));
  for (const invite of body?.items ?? []) {
    if (invite?.target_user === GUEST && invite?.status === "pending") {
      await request.delete(
        `${BASE_URL}/api/projects/${OWNER}/${PROJECT}/invites/${invite.invite_id}`,
      );
    }
  }
}

test("the members tab lists who is in the project", async ({ page, consoleErrors }) => {
  await visit(page, `${base}/settings/members`);

  // The owner is always a member, so this table is never legitimately empty.
  await expect(page.locator(`[data-member="${OWNER}"]`)).toBeVisible();
  await expect(page.getByRole("button", { name: "Send invitation" })).toBeVisible();

  expect(await page.content()).not.toContain("RWE component error");
  expect(consoleErrors).toEqual([]);
});

/**
 * Invite, accept, and confirm the role took effect.
 *
 * Two browser sessions, because the point of the flow is that one person asks
 * and a different person answers. A single-session test would prove only that
 * the buttons exist.
 */
test("someone invited can accept and then sees the project", async ({
  page,
  browser,
  request,
}) => {
  await ensureGuestExists(request);
  await ensureNotAMember(request);

  // The maintainer invites.
  await visit(page, `${base}/settings/members`);
  await page.getByRole("textbox", { name: "Invite" }).fill(GUEST);
  await page.getByRole("button", { name: "Send invitation" }).click();
  await expect(page.getByText(new RegExp(`Invited ${GUEST}`))).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText(/Waiting on an answer/i)).toBeVisible();

  // The invitee answers, in their own session.
  const guestContext = await browser.newContext();
  const guest = await guestContext.newPage();
  await guest.goto(`${BASE_URL}/login`);
  await guest.getByRole("textbox").first().fill(GUEST);
  await guest.locator('input[type="password"]').fill(GUEST_PASSWORD);
  await guest.getByRole("button", { name: /log ?in|sign ?in/i }).first().click();
  await guest.waitForURL(/\/home/, { timeout: 20_000 });

  // Before accepting: invited, and the project is not theirs to open.
  await expect(guest.getByText(/You have been invited/i)).toBeVisible({ timeout: 15_000 });
  await expect(guest.locator(`a[href*="${base}"]`)).toHaveCount(0);

  await guest.getByRole("button", { name: "Accept", exact: true }).click();

  // After accepting, the project is on their home page — a membership nobody
  // can find is not a membership.
  await expect(guest.locator(`a[href*="${base}"]`).first()).toBeVisible({ timeout: 20_000 });
  await expect(guest.getByText(/You have been invited/i)).toHaveCount(0);

  await guestContext.close();

  // And the maintainer sees them as a member.
  await visit(page, `${base}/settings/members`);
  await expect(page.locator(`[data-member="${GUEST}"]`)).toBeVisible({ timeout: 15_000 });

  await ensureNotAMember(request);
});

/** Declining leaves the project alone and clears the invitation. */
test("an invitation can be declined", async ({ page, browser, request }) => {
  await ensureGuestExists(request);
  await ensureNotAMember(request);

  await visit(page, `${base}/settings/members`);
  await page.getByRole("textbox", { name: "Invite" }).fill(GUEST);
  await page.getByRole("button", { name: "Send invitation" }).click();
  await expect(page.getByText(/Waiting on an answer/i)).toBeVisible({ timeout: 15_000 });

  const guestContext = await browser.newContext();
  const guest = await guestContext.newPage();
  await guest.goto(`${BASE_URL}/login`);
  await guest.getByRole("textbox").first().fill(GUEST);
  await guest.locator('input[type="password"]').fill(GUEST_PASSWORD);
  await guest.getByRole("button", { name: /log ?in|sign ?in/i }).first().click();
  await guest.waitForURL(/\/home/, { timeout: 20_000 });

  await expect(guest.getByText(/You have been invited/i)).toBeVisible({ timeout: 15_000 });
  await guest.getByRole("button", { name: "Decline", exact: true }).click();

  await expect(guest.getByText(/You have been invited/i)).toHaveCount(0, { timeout: 15_000 });
  await expect(guest.locator(`a[href*="${base}"]`)).toHaveCount(0);

  await guestContext.close();

  // Declining is an answer, so the invitation stops waiting for one.
  await visit(page, `${base}/settings/members`);
  await expect(page.locator(`[data-member="${GUEST}"]`)).toHaveCount(0);
});
