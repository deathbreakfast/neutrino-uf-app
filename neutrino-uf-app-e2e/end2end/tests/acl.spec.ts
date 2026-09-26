import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

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
});
