import AxeBuilder from "@axe-core/playwright";
import type { Page, Route } from "@playwright/test";
import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

const PICKER_PLACEHOLDER = "Search by name";

function unique(prefix: string) {
  return `${prefix}${Math.random().toString(36).slice(2, 8)}`;
}

/** Server fn URLs end in the fn name plus a hash. */
const serverFn = (name: string) => new RegExp(`/${name}\\w*$`);

async function fulfillServerError(route: Route, message: string) {
  await route.fulfill({
    status: 500,
    headers: { serverfnerror: new URL(route.request().url()).pathname },
    contentType: "text/plain",
    body: `ServerError|${message}`,
  });
}

/** Open Access, check nothing is picked, then pick the secret the admin owns. */
async function openAclPage(page: Page) {
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
  await expect(page.getByTestId("neutrino-acl-choose")).toBeVisible();
  const select = page.locator("#secrets-acl-secret-select select");
  await expect(
    select.locator(`option[value="${seeded.fixtures.admin_secret_id}"]`),
  ).toBeAttached({ timeout: 15_000 });
  await select.selectOption(seeded.fixtures.admin_secret_id);
  await expect(page.getByTestId("neutrino-access-card")).toBeVisible({
    timeout: 60_000,
  });
  return seeded;
}

async function seedPerson(page: Page, display_name: string) {
  const id = unique("grantee");
  const res = await page.request.post("/api/test/person-lab", {
    data: { action: "person", id, display_name },
  });
  expect(res.ok()).toBeTruthy();
  return id;
}

async function seedServiceUser(page: Page, display_name: string) {
  const handle = unique("bot-");
  const res = await page.request.post("/api/test/person-lab", {
    data: { action: "service_user", owner: "admin", handle, display_name },
  });
  expect(res.ok()).toBeTruthy();
  return handle;
}

const giveDialog = (page: Page) => page.getByTestId("neutrino-give-access-dialog");
const giveButton = (page: Page) =>
  page.getByTestId("neutrino-give-submit").getByRole("button");

async function openGive(page: Page) {
  await page.getByTestId("secrets-acl-grant-form").getByRole("button").click();
  await expect(giveDialog(page)).toBeVisible();
}

/** Type into the picker and click the row whose name matches. */
async function pickByName(page: Page, query: string, name: string) {
  const input = page.getByPlaceholder(PICKER_PLACEHOLDER);
  await input.fill(query);
  const option = page.getByRole("option", { name: new RegExp(name) });
  await expect(option.first()).toBeVisible({ timeout: 30_000 });
  await option.first().click();
}

/** Plain-word label for each Gauge action, as the Give access checkboxes show it. */
const ACTION_LABELS: Record<string, string> = {
  View: "See it",
  Reveal: "Read the value",
  Edit: "Change it",
  Delete: "Delete it",
  Maintain: "Manage access",
};

/** Tick an action by clicking its visible label, then confirm it took. */
async function tick(page: Page, action: string) {
  const box = page.getByTestId(`neutrino-give-action-${action}`);
  await box.getByText(ACTION_LABELS[action], { exact: true }).click();
  await expect(box.getByRole("checkbox")).toBeChecked();
}

/** Give `actions` (Gauge names) to the person named `name`. */
async function give(page: Page, query: string, name: string, actions: string[]) {
  await openGive(page);
  await pickByName(page, query, name);
  for (const action of actions) {
    await tick(page, action);
  }
  await giveButton(page).click();
  await expect(giveDialog(page)).toBeHidden({ timeout: 60_000 });
}

/** The By person table row for `name`. */
const personRow = (page: Page, name: string) =>
  page
    .getByTestId("acl-by-person")
    .locator('[data-table-role="row"]')
    .filter({ hasText: name });

test.describe("pw-vault-acl", () => {
  test("pw-vault-acl-manage-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    // Cold-load `/secrets` first so the WASM bundle finishes under a route that
    // hydrates reliably; direct `/secrets/acl` can stick in Orbital boot error.
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("Neutrino", { exact: true }).first()).toBeVisible();
    await page.goto("/secrets/acl", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-acl-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.locator("#secrets-acl-title")).toContainText("Access");
    await expect(
      page.getByText(
        "Who can do what with each secret. You can also open a secret and use its Access tab.",
      ),
    ).toBeVisible();
    // Nothing is picked for you.
    await expect(page.locator("#secrets-acl-secret-select select")).toHaveValue("");
    await expect(page.getByTestId("neutrino-acl-choose")).toContainText("Choose a secret");
    await expect(page.getByTestId("neutrino-access-card")).toHaveCount(0);
  });

  test("pw-vault-acl-grant-person-happy", async ({ page }) => {
    const name = `Grace ${unique("Hopper")}`;
    const seeded = await openAclPage(page);
    await expect(page.getByTestId("neutrino-acl-open-secret").getByRole("link")).toHaveAttribute(
      "href",
      `/secrets/${seeded.fixtures.admin_secret_id}`,
    );
    const id = await seedPerson(page, name);
    await give(page, "Grace", name, ["View"]);
    const row = personRow(page, name);
    await expect(row).toBeVisible({ timeout: 60_000 });
    await expect(row.getByTestId("acl-person-row")).toHaveAttribute("data-kind", "user");
    await expect(row).toContainText("Person");
    await expect(row).toContainText("See it");
    // Display names only: no raw ids or permission names.
    const table = page.getByTestId("acl-by-person");
    await expect(table).not.toContainText(id);
    await expect(table).not.toContainText(/\bu-[0-9a-z]/);
    await expect(page.getByTestId("neutrino-access-card")).not.toContainText("neutrino_secret.");
  });

  test("pw-vault-acl-grant-service-user-happy", async ({ page }) => {
    const name = `Deploy ${unique("bot")}`;
    await openAclPage(page);
    const handle = await seedServiceUser(page, name);
    await give(page, "Deploy", name, ["View"]);
    const row = personRow(page, name);
    await expect(row).toBeVisible({ timeout: 60_000 });
    await expect(row.getByTestId("acl-person-row")).toHaveAttribute("data-kind", "service_user");
    await expect(row).toContainText("Service user");
    await expect(row).not.toContainText(handle);
  });

  test("pw-vault-acl-give-two-actions-happy", async ({ page }) => {
    const name = `Two ${unique("Actions")}`;
    await openAclPage(page);
    await seedPerson(page, name);
    await give(page, "Two", name, ["View", "Edit"]);
    const row = personRow(page, name);
    await expect(row).toContainText("See it", { timeout: 60_000 });
    await expect(row).toContainText("Change it");

    // By action lists them under each action; switching keeps the loaded grants.
    await page.getByText("By action", { exact: true }).click();
    await expect(page.getByTestId("acl-action-View")).toContainText(name);
    await expect(page.getByTestId("acl-action-Edit")).toContainText(name);
    await expect(page.getByTestId("acl-action-Delete")).toContainText(
      "No one has this directly.",
    );
    await expect(page.getByTestId("acl-action-View")).toContainText("Permission id");
  });

  test("pw-vault-acl-revoke-person-and-service-user-happy", async ({ page }) => {
    const person = `Revoke ${unique("Person")}`;
    const bot = `Revoke ${unique("Bot")}`;
    await openAclPage(page);
    await seedPerson(page, person);
    await seedServiceUser(page, bot);
    for (const name of [person, bot]) {
      await give(page, name, name, ["View"]);
      await expect(personRow(page, name)).toBeVisible({ timeout: 60_000 });
    }
    for (const name of [person, bot]) {
      // The table rebuilds its rows (and their menus) after each save, so retry
      // until the menu stays open long enough to pick Remove access.
      await expect(async () => {
        await personRow(page, name)
          .getByRole("button", { name: `More for ${name}` })
          .first()
          .click({ timeout: 3_000 });
        await page
          .getByRole("menuitem", { name: "Remove access" })
          .first()
          .click({ timeout: 3_000 });
      }).toPass({ timeout: 30_000 });
      const dialog = page.getByTestId("neutrino-remove-access-dialog");
      await expect(dialog).toContainText(`${name.split(" ")[0]} loses`);
      await expect(dialog).toContainText("Only grants on this secret go.");
      await dialog.getByTestId("neutrino-remove-submit").getByRole("button").click();
      await expect(personRow(page, name)).toHaveCount(0, { timeout: 60_000 });
    }
  });

  test("pw-vault-acl-grant-nothing-picked-sad", async ({ page }) => {
    await openAclPage(page);
    await openGive(page);
    await tick(page, "View");
    await giveButton(page).click();
    await expect(page.getByTestId("neutrino-give-hint")).toHaveText(
      "Pick a person or service user.",
    );
    await expect(giveDialog(page)).toBeVisible();
  });

  test("pw-vault-acl-picker-short-and-unknown-query-sad", async ({ page }) => {
    await openAclPage(page);
    await openGive(page);
    const input = page.getByPlaceholder(PICKER_PLACEHOLDER);
    await input.fill("G");
    const listbox = page.getByRole("listbox");
    await expect(listbox.getByText("Type 2 or more letters to search.")).toBeVisible();
    // Native selects also expose options; the picker's list is the listbox.
    const pickable = page.getByRole("listbox").getByRole("option", { disabled: false });
    await expect(pickable).toHaveCount(0);
    await input.fill("Zzqxnobody");
    // "No one matches." only shows once the search answered ("Searching…" before).
    await expect(listbox.getByText("No one matches.")).toBeVisible();
    await expect(pickable).toHaveCount(0);
  });

  test("pw-vault-acl-failed-grant-keeps-dialog-sad", async ({ page }) => {
    const name = `Fail ${unique("Grant")}`;
    await openAclPage(page);
    await seedPerson(page, name);
    await page.route(serverFn("grant_secret_actions"), (route) =>
      fulfillServerError(route, "gauge_failed::grant secret action"),
    );
    await openGive(page);
    await pickByName(page, "Fail", name);
    await tick(page, "View");
    await giveButton(page).click();
    const error = page.getByTestId("neutrino-give-error");
    await expect(error).toContainText("Access wasn't given", { timeout: 60_000 });
    await expect(error).toContainText("Gauge didn't save the grant. Nothing changed; try again.");
    await expect(giveDialog(page)).toBeVisible();
    await expect(giveButton(page)).toBeEnabled();
    await expect(page.getByText(/error running server function|gauge_failed::/)).toHaveCount(0);
  });

  test("pw-vault-acl-no-raw-user-id-field-and-axe", async ({ page }) => {
    await openAclPage(page);
    await openGive(page);
    await expect(page.getByText("User id", { exact: true })).toHaveCount(0);
    await expect(page.getByLabel("User id")).toHaveCount(0);
    await expect(page.getByPlaceholder(PICKER_PLACEHOLDER)).toBeVisible();
    const results = await new AxeBuilder({ page })
      .include('[data-testid="neutrino-give-access-dialog"]')
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();
    expect(results.violations).toEqual([]);
  });
});
