import { test, expect, seedAuth, waitForHydrated } from "./fixtures";
import type { Page } from "@playwright/test";

async function completeVisibleTour(page: Page) {
  const footer = page.locator('[data-testid="spotlight-footer"]:visible');
  const next = footer.getByTestId("spotlight-tour-next");
  await expect(footer).toBeVisible({ timeout: 60_000 });
  for (let i = 0; i < 40; i++) {
    if ((await footer.count()) === 0) {
      break;
    }
    // Spotlight panels can sit partially off-screen; DOM click avoids Playwright
    // viewport hit-testing failures that still occur with { force: true }.
    await next.evaluate((el: HTMLElement) => el.click());
    try {
      await expect(footer).toHaveCount(0, { timeout: 2_000 });
      break;
    } catch {
      /* more steps */
    }
  }
  await expect(footer).toHaveCount(0, { timeout: 30_000 });
}

const NEUTRINO_TOUR_ROUTES = [
  { path: "/secrets", firstStep: "help-step-secrets-intro" },
  { path: "/secrets/acl", firstStep: "help-step-secrets-acl-intro" },
] as const;

test.describe("help-spotlight", () => {
  test("help-spotlight-skips-when-seeded", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("neutrino-secrets-list-page")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("help-step-secrets-intro")).toHaveCount(0);
    await expect(page.locator('[data-testid="spotlight-footer"]:visible')).toHaveCount(0);
  });

  test("help-spotlight-skips-auth-gate", async ({ page }) => {
    await seedAuth(page, "anonymous", { help_tour: true });
    await page.goto("/secrets", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("auth-required-empty-state")).toBeAttached({
      timeout: 60_000,
    });
    await expect(page.getByTestId("help-step-secrets-intro")).toHaveCount(0);
    await expect(page.locator('[data-testid="spotlight-footer"]:visible')).toHaveCount(0);
  });

  for (const route of NEUTRINO_TOUR_ROUTES) {
    test(`help-spotlight-green-${route.path}`, async ({ page }) => {
      await seedAuth(page, "admin", { help_tour: true });
      if (route.path === "/secrets/acl") {
        // Full reload of /secrets/acl often forges hydrate before WASM Effects
        // run. Prove hydrate on /secrets, finish that tour, then client-nav to
        // ACL so HelpTourPlayer keeps the live WASM runtime.
        await page.goto("/secrets", { waitUntil: "domcontentloaded" });
        await waitForHydrated(page);
        await expect(page.getByTestId("help-step-secrets-intro")).toBeVisible({
          timeout: 120_000,
        });
        await completeVisibleTour(page);
        await page.getByRole("button", { name: "Expand navigation" }).click();
        await page.getByTestId("nav-secrets-acl").getByRole("link").click();
        await expect(page.getByTestId("neutrino-acl-page")).toBeVisible({
          timeout: 30_000,
        });
        await expect(page.getByTestId(route.firstStep)).toBeVisible({
          timeout: 60_000,
        });
        await completeVisibleTour(page);
        await expect(page.getByTestId(route.firstStep)).toHaveCount(0);
        return;
      }
      await page.goto(route.path, { waitUntil: "domcontentloaded" });
      await waitForHydrated(page);
      await expect(page.getByTestId(route.firstStep)).toBeVisible({
        timeout: 120_000,
      });
      await completeVisibleTour(page);
      await expect(page.getByTestId(route.firstStep)).toHaveCount(0);
    });
  }
});
