import type { Page, Route } from "@playwright/test";
import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  expectMutationDenied,
  clickSecretAction,
} from "./fixtures";

/** Server fn URLs end in the fn name plus a hash. */
const serverFn = (name: string) => new RegExp(`/${name}\\w*$`);

/** Answer a server fn call the way Leptos encodes a `ServerFnError`. */
async function fulfillServerError(route: Route, message: string) {
  await route.fulfill({
    status: 500,
    headers: { serverfnerror: new URL(route.request().url()).pathname },
    contentType: "text/plain",
    body: `ServerError|${message}`,
  });
}

/** No raw server text reaches the screen. */
async function expectNoRawErrors(page: Page) {
  await expect(
    page.getByText(/error running server function|STEP_UP:|permission_denied::/),
  ).toHaveCount(0);
}

async function fillRotate(page: Page, value: string) {
  await page.getByTestId("neutrino-rotate-plaintext").locator("input").fill(value);
}

async function confirmDelete(page: Page, name: string) {
  await page.getByTestId("neutrino-delete-type").locator("input").fill(name);
  await page.getByTestId("neutrino-delete-confirm").getByRole("button").click();
}

/** Lab codes come from the clock; start a two-read check early in a 30 s window. */
async function waitForFreshTotpWindow(page: Page) {
  const intoWindow = Math.floor(Date.now() / 1000) % 30;
  if (intoWindow > 20) {
    await page.waitForTimeout((30 - intoWindow) * 1000 + 500);
  }
}

test.describe("pw-vault-crud", () => {
  test("pw-vault-crud-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    await expect(page.getByText("Neutrino", { exact: true }).first()).toBeVisible();

    const name = `e2e-crud-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await expect(page.getByTestId("neutrino-create-secret-dialog")).toBeVisible();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/crud");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("v1-plaintext");
    await page.getByTestId("neutrino-create-submit").click();

    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "1",
    );

    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-secret-dialog")).toBeVisible();
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveText("v1-plaintext", {
      timeout: 60_000,
    });
    await expect(
      page.getByTestId("neutrino-reveal-copy").getByRole("button", { name: "Copy" }),
    ).toBeVisible();
    await expect(page.getByTestId("neutrino-reveal-timer")).toContainText(
      /Hides in \d+ seconds?\. Closing clears it\./,
    );
    await expect(page.getByTestId("neutrino-reveal-base64")).toHaveCount(0);
    await page.getByTestId("neutrino-reveal-close").click();

    await clickSecretAction(page, name, "Rotate");
    await expect(
      page.getByText("Version 2 replaces version 1. Apps get the new value the next time they read it."),
    ).toBeVisible();
    await fillRotate(page, "v2-plaintext");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "2",
      { timeout: 60_000 },
    );

    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveText("v2-plaintext", {
      timeout: 60_000,
    });
    await page.getByTestId("neutrino-reveal-close").click();

    await clickSecretAction(page, name, "Delete");
    await expect(page.getByText(`Delete ${name}?`)).toBeVisible();
    const confirm = page.getByTestId("neutrino-delete-confirm").getByRole("button");
    await expect(confirm).toHaveClass(/orbital-button--danger/);
    await expect(confirm).toBeDisabled();
    await page.getByTestId("neutrino-delete-type").locator("input").fill(name.slice(0, -1));
    await expect(confirm).toBeDisabled();
    await page.getByTestId("neutrino-delete-type").locator("input").fill(name);
    await expect(confirm).toBeEnabled();
    await confirm.click();
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toHaveCount(0, {
      timeout: 60_000,
    });
  });
});

test.describe("pw-vault-validation", () => {
  test("pw-vault-create-blank-sad", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    await page.getByTestId("neutrino-create-secret-btn").click();
    await expect(page.getByTestId("neutrino-create-secret-dialog")).toBeVisible();
    await page.getByTestId("neutrino-create-submit").click();
    await expect(page.getByTestId("neutrino-create-error")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("neutrino-create-error")).toContainText(/required/i);
    await expect(page.getByTestId("neutrino-create-secret-dialog")).toBeVisible();
    await expectNoRawErrors(page);
  });

  test("pw-vault-rotate-blank-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Rotate");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(page.getByTestId("neutrino-rotate-validation")).toHaveText(
      "Paste the new value.",
    );
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "1",
    );
  });
});

test.describe("pw-vault-authz", () => {
  test("pw-vault-read-only-no-create-sad", async ({ page }) => {
    // Outsider holds SecretsRead only: no Create button, a Request access bar instead.
    await seedAuth(page, "outsider");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    const bar = page.getByTestId("neutrino-ask-create-bar");
    await expect(bar).toBeVisible({ timeout: 60_000 });
    await expect(bar).toContainText("Adding secrets needs SecretsWrite");
    await expect(bar).toContainText(
      "Request it, or ask someone who can add secrets to store this one for you.",
    );
    await expect(page.getByTestId("neutrino-create-secret-btn")).toHaveCount(0);
    await expect(page.getByText("permission-gated server-side")).toHaveCount(0);
    await bar.getByRole("button", { name: "Request access" }).click();
    await expect(
      bar.getByTestId("neutrino-request-access-sent").or(bar.getByText(/already/)),
    ).toBeVisible({ timeout: 60_000 });
    await expectNoRawErrors(page);
  });

  test("pw-vault-reveal-denied-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    const name = seeded.fixtures.outsider_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Reveal");
    await expectMutationDenied(page);
    await expectNoRawErrors(page);
  });

  test("pw-vault-rotate-denied-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    const name = seeded.fixtures.outsider_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Rotate");
    await fillRotate(page, "rotate-denied");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expectMutationDenied(page);
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "1",
    );
  });

  test("pw-vault-delete-denied-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "outsider");
    const name = seeded.fixtures.outsider_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Delete");
    await confirmDelete(page, name);
    await expectMutationDenied(page);
    await expectNoRawErrors(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible();
  });

  test("pw-vault-step-up-required-sad", async ({ page }) => {
    await seedAuth(page, "admin", { step_up_window: "none" });
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    const name = `e2e-stepup-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/stepup");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("blocked");
    await page.getByTestId("neutrino-create-submit").click();
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error").first())
        .or(page.getByTestId("neutrino-create-error")),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toHaveCount(0);
  });

  test("pw-vault-step-up-expired-sad", async ({ page }) => {
    await seedAuth(page, "admin", { step_up_window: "expired" });
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    const name = `e2e-expired-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/expired");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("blocked");
    await page.getByTestId("neutrino-create-submit").click();
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error").first())
        .or(page.getByTestId("neutrino-create-error")),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toHaveCount(0);
  });

  test("pw-vault-step-up-other-user-sad", async ({ page }) => {
    // TM-3: valid window bound to a different user than the session must deny create.
    await seedAuth(page, "admin", { step_up_window: "other_user" });
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    const name = `e2e-other-user-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/other-user");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("blocked");
    await page.getByTestId("neutrino-create-submit").click();
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error").first())
        .or(page.getByTestId("neutrino-create-error")),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toHaveCount(0);
  });

  test("pw-vault-break-glass-window-only-deny-sad", async ({ page }) => {
    // TM-12: Super User + valid window + empty fresh TOTP must deny outsider reveal.
    const seeded = await seedAuth(page, "admin", {
      step_up_window: "valid",
      fresh_totp: "empty",
      promote_super_user: true,
    });
    const name = seeded.fixtures.outsider_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Reveal");
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error"))
        .or(page.getByTestId("neutrino-reveal-error"))
        .first(),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveCount(0);
  });

  test("pw-vault-break-glass-fresh-happy", async ({ page }) => {
    // TM-12: Super User + auto fresh TOTP can reveal outsider secret (break-glass).
    const seeded = await seedAuth(page, "admin", {
      step_up_window: "valid",
      fresh_totp: "auto",
      promote_super_user: true,
    });
    const name = seeded.fixtures.outsider_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-secret-dialog")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveText("outsider-seed-pt", {
      timeout: 60_000,
    });
    await page.getByTestId("neutrino-reveal-close").click();
  });

  test("pw-vault-list-without-step-up-happy", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", { step_up_window: "none" });
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-vault-rotate-without-step-up-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", { step_up_window: "none" });
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Rotate");
    await fillRotate(page, "should-fail");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error").first())
        .or(page.getByTestId("neutrino-rotate-error")),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText("1");
  });

  test("pw-vault-delete-without-step-up-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", { step_up_window: "none" });
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Delete");
    await confirmDelete(page, name);
    await expect(
      page
        .getByText(/Confirm it's you/i)
        .or(page.locator(".orbital-message-bar--error").first())
        .or(page.getByTestId("neutrino-delete-error")),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible();
  });

  test("pw-vault-reveal-fresh-without-window-happy", async ({ page }) => {
    // Fresh reveal ignores the sudo window (TM-3 / TM-12); e2e-lab supplies a code.
    const seeded = await seedAuth(page, "admin", { step_up_window: "none" });
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-secret-dialog")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveText("admin-seed-pt", {
      timeout: 60_000,
    });
    await page.getByTestId("neutrino-reveal-close").click();
  });
});

test.describe("pw-vault-requestor", () => {
  test("pw-vault-requestor-mutate-happy", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    const name = `e2e-requestor-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/requestor");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("requestor-v1");
    await page.getByTestId("neutrino-create-submit").click();

    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Rotate");
    await fillRotate(page, "requestor-v2");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "2",
      { timeout: 60_000 },
    );
  });

  test("pw-vault-requestor-reveal-denied-sad", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);

    const name = `e2e-requestor-deny-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/requestor-deny");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("hidden");
    await page.getByTestId("neutrino-create-submit").click();
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Reveal");
    await expectMutationDenied(page);
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveCount(0);
  });
});

test.describe("pw-vault-dialogs", () => {
  test("pw-vault-read-value-reused-code-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await waitForFreshTotpWindow(page);
    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveText("admin-seed-pt", {
      timeout: 60_000,
    });
    await page.getByTestId("neutrino-reveal-close").click();

    // Same 30 s window, so the lab hands out the same code; the server refuses it.
    await page.route(serverFn("reveal_vault_secret"), (route) =>
      fulfillServerError(
        route,
        "STEP_UP:step_up_invalid: reason_class=step_up_invalid: invalid or reused totp code",
      ),
    );
    await clickSecretAction(page, name, "Reveal");
    const error = page.getByTestId("neutrino-reveal-error");
    await expect(error).toContainText("That code was already used", { timeout: 60_000 });
    await expect(error).toContainText(
      "Each code works once. Wait for the next one in your authenticator app, then type it here.",
    );
    await expect(page.getByTestId("neutrino-reveal-plaintext")).toHaveCount(0);
    await expectNoRawErrors(page);
  });

  test("pw-vault-rotate-disables-both-buttons-while-running-happy", async ({ page }) => {
    // This rotate really runs, so it uses its own secret: rotating the shared
    // seed would change what later reads in the same lane see.
    await seedAuth(page, "admin");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `e2e-rotate-busy-${Date.now()}`;
    await page.getByTestId("neutrino-create-secret-btn").click();
    await page.getByTestId("neutrino-create-name").locator("input").fill(name);
    await page.getByTestId("neutrino-create-scope").locator("input").fill("/e2e/rotate-busy");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page.getByTestId("neutrino-create-plaintext").locator("input").fill("busy-v1");
    await page.getByTestId("neutrino-create-submit").click();
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await page.route(serverFn("rotate_vault_secret"), async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 2_000));
      await route.continue();
    });
    await clickSecretAction(page, name, "Rotate");
    await fillRotate(page, "slow-v2");
    const submit = page.getByTestId("neutrino-rotate-submit").getByRole("button");
    await expect(submit).toHaveText("Rotate to version 2");
    await submit.click();
    await expect(submit).toBeDisabled();
    await expect(page.getByTestId("neutrino-rotate-cancel").getByRole("button")).toBeDisabled();
    await expect(page.getByTestId("neutrino-rotate-secret-dialog")).toBeHidden({
      timeout: 60_000,
    });
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText("2", {
      timeout: 60_000,
    });
  });

  test("pw-vault-rotate-notify-failed-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin");
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    // The version saved but the notice to Nucleus didn't go out.
    await page.route(serverFn("rotate_vault_secret"), (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          version: 2,
          notify_error: "vault_internal::Nucleus wasn't told about version 2",
        }),
      }),
    );
    await clickSecretAction(page, name, "Rotate");
    await fillRotate(page, "notify-v2");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(page.getByTestId("neutrino-rotate-secret-dialog")).toBeHidden({
      timeout: 60_000,
    });
    const bar = page.getByTestId("neutrino-notify-failed");
    await expect(bar).toContainText("Version 2 is saved, but Nucleus hasn't been told");
    const resend = bar.getByRole("button", { name: "Send notice again" });
    await expect(resend).toBeVisible();
    await resend.click();
    await expect(bar).toHaveCount(0, { timeout: 60_000 });
  });
});
