import { chromium, type FullConfig } from "@playwright/test";

/**
 * Cold-start the Orbital WASM once before the suite. The first browser load after
 * `cargo leptos` starts can stick in boot `error` for minutes; later tests then
 * hydrate in under a second. Warming here keeps acl/auth/help off that cliff.
 */
async function globalSetup(_config: FullConfig) {
  const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3160";
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const deadline = Date.now() + 240_000;

  while (Date.now() < deadline) {
    await page.goto(`${baseURL}/secrets`, { waitUntil: "domcontentloaded" });
    const ready = await page
      .waitForFunction(
        () => {
          const html = document.documentElement;
          if (html.getAttribute("data-orbital-hydrated") === "true") {
            return true;
          }
          if (html.getAttribute("data-orbital-boot-state") !== "error") {
            return false;
          }
          const progress = window as unknown as {
            __orbitalBootProgress?: { steps?: { hydrate?: string } };
            __orbitalBootDismissOverlay?: () => void;
          };
          if (progress.__orbitalBootProgress?.steps?.hydrate !== "complete") {
            return false;
          }
          html.removeAttribute("data-orbital-boot-state");
          progress.__orbitalBootDismissOverlay?.();
          return html.getAttribute("data-orbital-hydrated") === "true";
        },
        { timeout: 90_000 },
      )
      .then(() => true)
      .catch(() => false);

    if (ready) {
      await browser.close();
      return;
    }
    await page.waitForTimeout(1_500);
  }

  await browser.close();
  throw new Error(
    "global-setup: Orbital never reached data-orbital-hydrated on /secrets",
  );
}

export default globalSetup;
