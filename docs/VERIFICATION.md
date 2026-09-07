# neutrino-uf-app verification

Re-run after code or doc changes. This workspace is the **Neutrino ops UI**
(`neutrino-app` / `NeutrinoRoutes`). Domain contracts live in the sibling
[neutrino](https://github.com/unified-field-dev/neutrino) repo.

## PR CI parity (run before push / opening a PR)

Matches [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) required jobs.
Toolchain: **nightly-2026-08-07**. Global env includes `RUSTFLAGS=-D warnings`.

| CI job | Local command |
|--------|----------------|
| fmt | `cargo fmt -p neutrino-app -p neutrino-uf-app-e2e -- --check` |
| clippy | `cargo clippy -p neutrino-app --features ssr --all-targets -- -D warnings` |
| test | `cargo check` + `cargo test -p neutrino-app --features ssr` below |
| docs | `RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p neutrino-app --features ssr --no-deps` |
| leptos-lints | dylint hydrate on `neutrino-app` (`nightly-2025-05-14`) |
| e2e | `cargo leptos end-to-end --project neutrino-uf-app-e2e` |

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-neutrino-uf-app
export CARGO_PROFILE_DEV_DEBUG=0
export RUSTFLAGS="-D warnings"
```

## Layer 1 — fmt / clippy / compile / test / rustdoc

```bash
cargo fmt -p neutrino-app -p neutrino-uf-app-e2e -- --check
cargo clippy -p neutrino-app --features ssr --all-targets -- -D warnings
cargo check -p neutrino-app --features ssr
cargo check -p neutrino-uf-app-e2e --features ssr
cargo check -p neutrino-app --target wasm32-unknown-unknown --features hydrate
cargo test -p neutrino-app --features ssr
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p neutrino-app --features ssr --no-deps
```

UI compile and rustdoc are pin-dependent on Orbital / `uf-product`. When those
graphs fail, prefer domain gates in neutrino over treating this as a vault API
regression.

### leptos-lints (CI job `leptos-lints`)

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"
cargo dylint --all -p neutrino-app --no-deps -- --features hydrate
```

## Layer 2 — E2E (Playwright)

Lab host on `127.0.0.1:3160` mounts eager `NeutrinoRoutes` pages (no `Lazy`,
no `--split`). Harness auth via `POST /api/test/seed-data`.

```bash
cd neutrino-uf-app-e2e/end2end && npm ci && npx playwright install chromium && cd ../..
cargo leptos end-to-end --project neutrino-uf-app-e2e
```

Do not Ctrl-C; the process exits when Playwright finishes. Scenario catalog:
[`neutrino-uf-app-e2e/README.md`](../neutrino-uf-app-e2e/README.md).

## Guide-contract audit

Deferred — no guide-contract workspace ships with this repository yet.
