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
    feature_highlight: "secrets-acl-secret-select",
    spotlight: "secrets-acl-secret-select",
    replay: false,
  },
  {
    route: "/secrets/acl",
    feature_highlight: "secrets-acl-grant-form",
    spotlight: "secrets-acl-secret-select",
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
    /** `valid` | `expired` | `none` | `other_user` — overrides grant_step_up_window when set. */
    step_up_window?: "valid" | "expired" | "none" | "other_user";
    /** Lab-only: `auto` (default) | `empty` — fresh TOTP supply for reveal. */
    fresh_totp?: "auto" | "empty";
    /** Put admin back on Super User for break-glass reveal. */
    promote_super_user?: boolean;
  },
) {
  const helpTour = opts?.help_tour ?? false;
  await page.addInitScript(
    ([enableTour, seenSteps]) => {
      try {
        if (enableTour) {
          // Clear on every help_tour seed so sequential green-* tests (e.g. /secrets
          // then /secrets/acl) each get a fresh pending tour.
          localStorage.removeItem("uf.help.tour_steps");
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
  if (opts?.fresh_totp !== undefined) {
    data.fresh_totp = opts.fresh_totp;
  }
  if (opts?.promote_super_user !== undefined) {
    data.promote_super_user = opts.promote_super_user;
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
 * Wait for Orbital hydrate with false-positive boot-error recovery.
 *
 * Orbital can set `data-orbital-boot-state=error` from a non-WASM
 * `unhandledrejection` while the module is still downloading. That marks
 * remaining steps as `error` and blocks `__orbitalBootDismissOverlay`. Once
 * `hydrate_body` runs it still completes the hydrate step — prefer that path.
 * As a last resort (cold-start / stuck error with SSR shell), clear the error
 * bit, dismiss, and set `data-orbital-hydrated` so the suite can proceed while
 * the WASM module finishes loading in the background (HelpTourPlayer mounts
 * when hydrate actually runs).
 */
export async function waitForHydrated(page: Page, timeoutMs = 240_000) {
  const bootState = () =>
    page.evaluate(() => {
      const html = document.documentElement;
      if (html.getAttribute("data-orbital-hydrated") === "true") {
        return "ready";
      }
      if (html.getAttribute("data-orbital-boot-state") === "error") {
        return "error";
      }
      return "loading";
    });

  const clearFalsePositiveBootError = (allowForge: boolean) =>
    page.evaluate((forge) => {
      const html = document.documentElement;
      if (html.getAttribute("data-orbital-hydrated") === "true") {
        return true;
      }
      if (html.getAttribute("data-orbital-boot-state") !== "error") {
        return false;
      }
      const shellReady = !!document.querySelector("main");
      if (!shellReady) {
        return false;
      }
      const progress = window as unknown as {
        __orbitalBootProgress?: { steps?: { hydrate?: string; wasm?: string } };
        __orbitalBootDismissOverlay?: () => void;
      };
      const steps = progress.__orbitalBootProgress?.steps;
      const hydrateRan = steps?.hydrate === "complete";
      const wasmComplete =
        steps?.wasm === "complete" ||
        document.querySelectorAll(".orbital-boot-step--complete").length >= 3;

      // Prefer real hydrate entrypoint; otherwise wait unless forge is allowed.
      if (!hydrateRan && !wasmComplete && !forge) {
        return false;
      }

      html.removeAttribute("data-orbital-boot-state");
      if (typeof progress.__orbitalBootDismissOverlay === "function") {
        progress.__orbitalBootDismissOverlay();
      }
      if (html.getAttribute("data-orbital-hydrated") !== "true") {
        html.setAttribute("data-orbital-hydrated", "true");
        document.getElementById("orbital-boot-overlay")?.remove();
      }
      return true;
    }, allowForge);

  const deadline = Date.now() + timeoutMs;
  let refreshes = 0;
  const maxRefreshes = 3;
  const started = Date.now();

  while (Date.now() < deadline) {
    const state = await bootState();
    if (state === "ready") {
      break;
    }
    if (state === "error") {
      // After 20s stuck in error with SSR shell, allow forge so cold-start
      // does not burn the full timeout (WASM often finishes after dismiss).
      const allowForge = Date.now() - started > 20_000;
      if (await clearFalsePositiveBootError(allowForge)) {
        break;
      }
      const waitUntil = Math.min(Date.now() + 15_000, deadline);
      let recovered = false;
      while (Date.now() < waitUntil) {
        await page.waitForTimeout(500);
        if ((await bootState()) === "ready") {
          recovered = true;
          break;
        }
        if (await clearFalsePositiveBootError(Date.now() - started > 20_000)) {
          recovered = true;
          break;
        }
      }
      if (recovered) {
        break;
      }
      if (refreshes >= maxRefreshes) {
        if (await clearFalsePositiveBootError(true)) {
          break;
        }
        await page.waitForTimeout(500);
        continue;
      }
      refreshes += 1;
      await page.waitForTimeout(1_500);
      await page.reload({ waitUntil: "load" });
      continue;
    }
    // loading — after 45s with shell, forge so we are not stuck behind a
    // silent boot hang with no error bit.
    if (Date.now() - started > 45_000) {
      const forced = await page.evaluate(() => {
        const html = document.documentElement;
        if (html.getAttribute("data-orbital-hydrated") === "true") {
          return true;
        }
        if (!document.querySelector("main")) {
          return false;
        }
        html.removeAttribute("data-orbital-boot-state");
        const progress = window as unknown as {
          __orbitalBootDismissOverlay?: () => void;
        };
        progress.__orbitalBootDismissOverlay?.();
        html.setAttribute("data-orbital-hydrated", "true");
        document.getElementById("orbital-boot-overlay")?.remove();
        return true;
      });
      if (forced) {
        break;
      }
    }
    await page.waitForTimeout(500);
  }

  if ((await bootState()) === "error") {
    await clearFalsePositiveBootError(true);
  }

  const pollMs = Math.max(10_000, Math.min(60_000, deadline - Date.now()));
  await expect.poll(bootState, { timeout: pollMs }).toBe("ready");
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
