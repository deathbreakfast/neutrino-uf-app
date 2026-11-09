import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";
import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

const PICKER_PLACEHOLDER = "Search by name";

function unique(prefix: string) {
  return `${prefix}${Math.random().toString(36).slice(2, 8)}`;
}

async function openAclPage(page: Page) {
  const logs: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error" || m.type() === "warning") logs.push(`${m.type()}: ${m.text()}`);
  });
  page.on("pageerror", (e) => logs.push(`pageerror: ${e.message}`));
  page.on("requestfailed", (r) => logs.push(`requestfailed: ${r.url()}`));
  try {
    return await openAclPageInner(page);
  } catch (e) {
    throw new Error(`${String(e)}\n--- page logs ---\n${logs.slice(0, 40).join("\n")}`);
  }
}

async function openAclPageInner(page: Page) {
  const seeded = await seedAuth(page, "admin");
  // A full load of /secrets/acl often forges hydrate before the WASM effects run
  // (see help_spotlight.spec.ts), so hydrate on /secrets and client-navigate.
  await page.goto("/secrets", { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
  await page.getByRole("button", { name: "Expand navigation" }).click();
  await page.getByTestId("nav-secrets-acl").getByRole("link").click();
  await expect(page.getByTestId("neutrino-acl-page")).toBeVisible({
    timeout: 15_000,
  });
  // The list can open on a secret the admin can't manage; pick the one they own.
  const select = page.locator("#secrets-acl-secret-select select");
  await expect(
    select.locator(`option[value="${seeded.fixtures.admin_secret_id}"]`),
  ).toBeAttached({ timeout: 15_000 });
  await select.selectOption(seeded.fixtures.admin_secret_id);
  await expect(page.getByTestId("secrets-acl-grant-form")).toBeVisible({
    timeout: 60_000,
  });
}

async function seedPerson(page: Page, display_name: string) {
  const res = await page.request.post("/api/test/person-lab", {
    data: { action: "person", id: unique("grantee"), display_name },
  });
  expect(res.ok()).toBeTruthy();
}

async function seedServiceUser(page: Page, display_name: string) {
  const res = await page.request.post("/api/test/person-lab", {
    data: {
      action: "service_user",
      owner: "admin",
      handle: unique("bot-"),
      display_name,
    },
  });
  expect(res.ok()).toBeTruthy();
}

/** Type into the picker and click the row whose name matches. */
async function pickByName(page: Page, query: string, name: string) {
  const input = page.getByPlaceholder(PICKER_PLACEHOLDER);
  await input.fill(query);
  const option = page.getByRole("option", { name: new RegExp(name) });
  await expect(option.first()).toBeVisible({ timeout: 30_000 });
  await option.first().click();
}

const viewGrants = (page: Page) => page.getByTestId("acl-action-View");

async function grantView(page: Page) {
  await page.getByRole("button", { name: "Grant access" }).click();
}


test.describe("pw-vault-acl", () => {
  test("pw-vault-acl-manage-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    // Cold-load `/secrets` first so the WASM bundle finishes under a route that
    // hydrates reliably; direct `/secrets/acl` can stick in Orbital boot error.
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.goto("/secrets/acl", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-acl-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText("Secret access grants", { exact: true })).toBeVisible();
    await expect(page.locator("#secrets-acl-secret-select")).toBeVisible();
    // After hydrate + SecretsGrantManage, either grant form (secret selected) or
    // empty-state copy when no selectable secret is available yet.
    const grantForm = page.getByTestId("secrets-acl-grant-form");
    const emptyPick = page.getByText("Pick a secret to manage grants.", {
      exact: true,
    });
    await expect(grantForm.or(emptyPick)).toBeVisible({ timeout: 60_000 });
  });

  test("pw-vault-acl-grant-person-happy", async ({ page }) => {
    const name = `Grace ${unique("Hopper")}`;
    await openAclPage(page);
    await seedPerson(page, name);
    await pickByName(page, "Grace", name);
    await grantView(page);
    const row = viewGrants(page).getByTestId("acl-grant-row").filter({ hasText: name });
    await expect(row).toBeVisible({ timeout: 60_000 });
    await expect(row).toHaveAttribute("data-kind", "user");
  });

  test("pw-vault-acl-grant-service-user-happy", async ({ page }) => {
    const name = `Deploy ${unique("bot")}`;
    await openAclPage(page);
    await seedServiceUser(page, name);
    await pickByName(page, "Deploy", name);
    await grantView(page);
    const row = viewGrants(page).getByTestId("acl-grant-row").filter({ hasText: name });
    await expect(row).toBeVisible({ timeout: 60_000 });
    await expect(row).toHaveAttribute("data-kind", "service_user");
  });

  test("pw-vault-acl-revoke-person-and-service-user-happy", async ({ page }) => {
    const person = `Revoke ${unique("Person")}`;
    const bot = `Revoke ${unique("Bot")}`;
    await openAclPage(page);
    await seedPerson(page, person);
    await seedServiceUser(page, bot);
    for (const name of [person, bot]) {
      await pickByName(page, "Revoke", name);
      await grantView(page);
      await expect(
        viewGrants(page).getByTestId("acl-grant-row").filter({ hasText: name }),
      ).toBeVisible({ timeout: 60_000 });
    }
    for (const name of [person, bot]) {
      const row = viewGrants(page)
        .locator("div")
        .filter({ has: page.getByTestId("acl-grant-row").filter({ hasText: name }) })
        .last();
      await row.getByRole("button", { name: "Revoke" }).click();
      await expect(
        viewGrants(page).getByTestId("acl-grant-row").filter({ hasText: name }),
      ).toHaveCount(0, { timeout: 60_000 });
    }
  });

  test("pw-vault-acl-grant-nothing-picked-sad", async ({ page }) => {
    await openAclPage(page);
    const before = await viewGrants(page).getByTestId("acl-grant-row").count();
    await grantView(page);
    await expect(
      page.getByText("Pick a person or service user.", { exact: true }),
    ).toBeVisible();
    await expect(viewGrants(page).getByTestId("acl-grant-row")).toHaveCount(before);
  });

  test("pw-vault-acl-picker-short-and-unknown-query-sad", async ({ page }) => {
    await openAclPage(page);
    const input = page.getByPlaceholder(PICKER_PLACEHOLDER);
    await input.fill("G");
    await page.waitForTimeout(800);
    await expect(page.getByRole("option")).toHaveCount(0);
    await input.fill("Zzqxnobody");
    await page.waitForTimeout(800);
    await expect(page.getByRole("option")).toHaveCount(0);
    const before = await viewGrants(page).getByTestId("acl-grant-row").count();
    await grantView(page);
    await expect(viewGrants(page).getByTestId("acl-grant-row")).toHaveCount(before);
  });

  test("pw-vault-acl-no-raw-user-id-field-and-axe", async ({ page }) => {
    await openAclPage(page);
    await expect(page.getByText("User id", { exact: true })).toHaveCount(0);
    await expect(page.getByLabel("User id")).toHaveCount(0);
    await expect(page.getByPlaceholder(PICKER_PLACEHOLDER)).toBeVisible();
    const results = await new AxeBuilder({ page })
      .include("#secrets-acl-grant-form")
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();
    expect(results.violations).toEqual([]);
  });
});
