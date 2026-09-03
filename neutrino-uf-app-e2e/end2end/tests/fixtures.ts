import { test as base, expect, type Page } from "@playwright/test";

export type SeedAuthKind =
  | "anonymous"
  | "admin"
  | "operator"
  | "requestor"
  | "outsider"
  | "unverified";

export type SeedFixtures = {
  outsider_secret_id: string;
  outsider_secret_name: string;
  admin_secret_id: string;
  admin_secret_name: string;
};

/** All Neutrino Help inventory keys — seed as seen so non-tour specs stay quiet. */
const NEUTRINO_HELP_STEPS_SEEN = [
  { route: "/secrets", feature_highlight: "secrets-intro", spotlight: null, replay: false },
  {
    route: "/secrets",
    feature_highlight: "secrets-page-title",
    spotlight: "secrets-page-title",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-permissions-note",
    spotlight: "secrets-permissions-note",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create",
    spotlight: "secrets-create-button",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-col-name",
    spotlight: "secrets-col-name",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-col-scope",
    spotlight: "secrets-col-scope",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-col-kind",
    spotlight: "secrets-col-kind",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-col-version",
    spotlight: "secrets-col-version",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-col-created",
    spotlight: "secrets-col-created",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-name",
    spotlight: "secrets-create-name",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-scope",
    spotlight: "secrets-create-scope",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-kind",
    spotlight: "secrets-create-kind",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-plaintext",
    spotlight: "secrets-create-plaintext",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-cancel",
    spotlight: "secrets-create-cancel",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-create-submit",
    spotlight: "secrets-create-submit",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-reveal",
    spotlight: "secrets-action-reveal",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-reveal-value",
    spotlight: "secrets-reveal-plaintext",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-reveal-close",
    spotlight: "secrets-reveal-close",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-rotate",
    spotlight: "secrets-action-rotate",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-rotate-plaintext",
    spotlight: "secrets-rotate-plaintext",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-rotate-submit",
    spotlight: "secrets-rotate-submit",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-delete",
    spotlight: "secrets-action-delete",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-delete-confirm",
    spotlight: "secrets-delete-confirm",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-nav-secrets",
    spotlight: "secrets-nav-secrets",
    replay: false,
  },
  {
    route: "/secrets",
    feature_highlight: "secrets-nav-acl",
    spotlight: "secrets-nav-acl",
    replay: false,
  },
  {
    route: "/secrets/acl",
    feature_highlight: "secrets-acl-intro",
    spotlight: null,
    replay: false,
  },
  {
    route: "/secrets/acl",
    feature_highlight: "secrets-acl-title",
    spotlight: "secrets-acl-title",
    replay: false,
  },
  {
    route: "/secrets/acl",
    feature_highlight: "secrets-acl-empty",
    spotlight: "secrets-acl-empty",
    replay: false,
  },
  {
    route: "/secrets/acl",
    feature_highlight: "secrets-acl-nav-secrets",
    spotlight: "secrets-nav-secrets",
    replay: false,
  },
] as const;

export async function seedAuth(
  page: Page,
  auth: SeedAuthKind,
  opts?: {
    help_tour?: boolean;
    grant_step_up_window?: boolean;
    /** `valid` | `expired` | `none` — overrides grant_step_up_window when set. */
    step_up_window?: "valid" | "expired" | "none";
  },
) {
  const helpTour = opts?.help_tour ?? false;
  await page.addInitScript(
    ([enableTour, seenSteps]) => {
      try {
        if (enableTour) {
          if (!sessionStorage.getItem("uf.help.e2e_tour_cleared")) {
            localStorage.removeItem("uf.help.tour_steps");
            sessionStorage.setItem("uf.help.e2e_tour_cleared", "1");
          }
          return;
        }
        localStorage.setItem("uf.help.tour_steps", JSON.stringify(seenSteps));
      } catch {
        /* ignore */
      }
    },
    [helpTour, NEUTRINO_HELP_STEPS_SEEN] as const,
  );

  const data: Record<string, unknown> = { auth };
  if (opts?.step_up_window !== undefined) {
    data.step_up_window = opts.step_up_window;
  } else if (opts?.grant_step_up_window !== undefined) {
    data.grant_step_up_window = opts.grant_step_up_window;
  }
  const res = await page.request.post("/api/test/seed-data", {
    data,
  });
  expect(res.ok()).toBeTruthy();
  return res.json() as Promise<{
    ok: boolean;
    auth: string;
    step_up_window?: boolean | string;
    totp_secret?: string;
    fixtures: SeedFixtures;
  }>;
}

/**
 * Wait for Orbital boot overlay to finish and hydrate to mark the document ready.
 */
export async function waitForHydrated(page: Page, timeoutMs = 240_000) {
  await expect
    .poll(
      async () =>
        page.evaluate(() => {
          const html = document.documentElement;
          if (html.getAttribute("data-orbital-boot-state") === "error") {
            return "error";
          }
          if (html.getAttribute("data-orbital-hydrated") === "true") {
            return "ready";
          }
          return "loading";
        }),
      { timeout: timeoutMs },
    )
    .not.toBe("error");
  await expect
    .poll(
      async () =>
        page.evaluate(
          () => document.documentElement.getAttribute("data-orbital-hydrated") === "true",
        ),
      { timeout: timeoutMs },
    )
    .toBe(true);
  await expect(page.getByTestId("orbital-boot-overlay")).toHaveCount(0, {
    timeout: 60_000,
  });
  await expect(page.getByTestId("e2e-auth-bootstrap")).toBeAttached({
    timeout: 30_000,
  });
}

/** Higgs / server-fn deny surfaces as an Orbital error MessageBar (dialog or page). */
export async function expectMutationDenied(page: Page) {
  await expect(
    page
      .locator(
        [
          ".orbital-message-bar--error",
          "[data-testid='neutrino-create-error']",
          "[data-testid='neutrino-reveal-error']",
          "[data-testid='neutrino-rotate-error']",
          "[data-testid='neutrino-delete-error']",
        ].join(", "),
      )
      .first(),
  ).toBeVisible({ timeout: 60_000 });
}

/** Click Reveal / Rotate / Delete on a vault row. */
export async function clickSecretAction(
  page: Page,
  name: string,
  action: "Reveal" | "Rotate" | "Delete",
) {
  await page
    .getByTestId(`neutrino-secret-actions-${name}`)
    .getByRole("button", { name: new RegExp(`^${action} secret$`, "i") })
    .click();
}

export const test = base;
export { expect };
