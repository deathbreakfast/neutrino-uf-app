# neutrino-uf-app-e2e

Leptos host that mounts Neutrino secrets pages for Playwright. Lab-only:
insecure session cookies, `POST /api/test/seed-data`, harness auth (no lepton sign-in).

## Run

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-neutrino-uf-app
export CARGO_PROFILE_DEV_DEBUG=0
# from neutrino-uf-app workspace root
cd neutrino-uf-app-e2e/end2end && npm ci && npx playwright install chromium && cd ../..
cargo leptos end-to-end --project neutrino-uf-app-e2e
```

Host listens on `127.0.0.1:3160`. Do not Ctrl-C; the run exits when Playwright finishes.

The lab host mounts the same page components as `NeutrinoRoutes`, without `Lazy`
(wasm-split Lazy under `ParentRoute` panics on hydrate in the current Leptos pin).
Production hosts keep `NeutrinoRoutes` + `--split` for code-splitting.

## Seed

`POST /api/test/seed-data` with JSON
`{ "auth": "admin" | "operator" | "requestor" | "outsider" | "unverified" | "anonymous" }`.

## Scenario catalog (implemented)

Auth: `pw-vault-auth-anonymous-sad`, `pw-vault-auth-unverified-sad`

Vault CRUD: `pw-vault-crud-happy` (create → list → reveal → rotate → delete)

Validation: `pw-vault-create-blank-sad`, `pw-vault-rotate-blank-sad`

Authz (outsider read-only): `pw-vault-write-denied-sad`, `pw-vault-reveal-denied-sad`, `pw-vault-rotate-denied-sad`, `pw-vault-delete-denied-sad`

Partial role (requestor: read+write+rotate, no reveal): `pw-vault-requestor-mutate-happy`, `pw-vault-requestor-reveal-denied-sad`

ACL manage route: `pw-vault-acl-manage-happy`

Help spotlight: `help-spotlight-skips-when-seeded`, `help-spotlight-skips-auth-gate`,
`help-spotlight-green-/secrets`, `help-spotlight-green-/secrets/acl`
(default `seedAuth` marks Neutrino tour steps seen in `localStorage`; pass
`{ help_tour: true }` to clear and walk Next→Finish)

### Deferred

| ID | Why deferred |
|----|--------------|
| ACL grant/revoke click matrix | Route + form smoke covers `/secrets/acl`; full grant→revoke Playwright flow deferred |
| Lepton MFA / OAuth / confirm | Owned by `lepton-auth-ui-e2e` / `lepton-uf-app-e2e` |
| Chronon / Boson / Photon IsolatedLab | Broader host platform composition, not Neutrino UI |
