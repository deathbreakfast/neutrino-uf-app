import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  expectMutationDenied,
  clickSecretAction,
} from "./fixtures";

test.describe("pw-vault-crud", () => {
  test("pw-vault-crud-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

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
    await expect(page.getByTestId("neutrino-reveal-plaintext").locator("input")).toHaveValue(
      Buffer.from("v1-plaintext").toString("base64"),
      { timeout: 60_000 },
    );
    await page.getByTestId("neutrino-reveal-close").click();

    await clickSecretAction(page, name, "Rotate");
    await page.getByLabel("New plaintext").fill("v2-plaintext");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "2",
      { timeout: 60_000 },
    );

    await clickSecretAction(page, name, "Reveal");
    await expect(page.getByTestId("neutrino-reveal-plaintext").locator("input")).toHaveValue(
      Buffer.from("v2-plaintext").toString("base64"),
      { timeout: 60_000 },
    );
    await page.getByTestId("neutrino-reveal-close").click();

    await clickSecretAction(page, name, "Delete");
    await page.getByTestId("neutrino-delete-confirm").click();
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
    await expect(page.getByTestId("neutrino-rotate-error")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("neutrino-rotate-error")).toContainText(/required/i);
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText(
      "1",
    );
  });
});

test.describe("pw-vault-authz", () => {
  test("pw-vault-write-denied-sad", async ({ page }) => {
    await seedAuth(page, "outsider");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });

    await page.getByTestId("neutrino-create-secret-btn").click();
    await expect(page.getByTestId("neutrino-create-secret-dialog")).toBeVisible();
    await page
      .getByTestId("neutrino-create-name")
      .locator("input")
      .fill("outsider-denied");
    await page
      .getByTestId("neutrino-create-scope")
      .locator("input")
      .fill("/e2e/denied");
    await page.getByTestId("neutrino-create-kind").locator("input").fill("token");
    await page
      .getByTestId("neutrino-create-plaintext")
      .locator("input")
      .fill("nope");
    await page.getByTestId("neutrino-create-submit").click();
    await expectMutationDenied(page);
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
    await page.getByLabel("New plaintext").fill("rotate-denied");
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
    await page.getByTestId("neutrino-delete-confirm").click();
    await expectMutationDenied(page);
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

  test("pw-vault-rotate-delete-without-step-up-sad", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", { step_up_window: "none" });
    const name = seeded.fixtures.admin_secret_name;
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId(`neutrino-secret-row-${name}`)).toBeVisible({
      timeout: 60_000,
    });

    await clickSecretAction(page, name, "Rotate");
    await page.getByLabel("New plaintext").fill("should-fail");
    await page.getByTestId("neutrino-rotate-submit").click();
    await expectMutationDenied(page);
    await expect(page.getByTestId(`neutrino-secret-version-${name}`)).toHaveText("1");

    await clickSecretAction(page, name, "Delete");
    await page.getByTestId("neutrino-delete-confirm").click();
    await expectMutationDenied(page);
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
    await expect(page.getByTestId("neutrino-reveal-plaintext").locator("input")).toHaveValue(
      Buffer.from("admin-seed-pt").toString("base64"),
      { timeout: 60_000 },
    );
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
    await page.getByLabel("New plaintext").fill("requestor-v2");
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
