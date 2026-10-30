# AGENTS.md

Rust workspace for **Kavio**: independent desktop products that share plumbing, plus one mobile companion and an optional LAN hub. Each product is installed, versioned and released separately; each install owns its own database.

Read `docs/architecture.md` for rationale. This file is the rules.

## Layout

```
crates/           shared plumbing — no product domain, ever
  ui-kit          tokens + theme; widgets behind off-by-default feature
  ui-shell        header / sidebar / content, Screen<S>, Shell<S>
  storage         Store, Migration, user_version runner, pragmas
  config          JSON settings, atomic tmp+rename save
  app-boot        init, Context, Dirs, logging, crash hook
apps/             one binary crate per product
  pos/            retail till
  school/         school admin
docs/architecture.md
```

Dependency direction is one-way: `apps/*` → `crates/*`. Never app → app. Never `ui-*` → `storage`/`sync` (portable crates stay UI-free).

## Commands

```bash
cargo build --offline            # must always work; see Offline below
cargo check                      # the default local loop — nothing heavier
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --locked

cargo build --release --locked -p pos     # CI only, never needed locally
```

Release builds happen in CI. Do not run them locally — `target/release` costs 2–3 GB and the machine is disk-constrained.

## Hard rules

- **No `crates/core`.** Shared crates must be narrow and purpose-specific.
- **No brand prefixes on crate names.** `ui-kit`, not `kavio-ui-kit`. Package names are brand-free on purpose; a rename must not touch them.
- **No shared domain model.** If two apps need it, it is plumbing. If it is domain, it lives in the app.
- **Compile-time composition.** Screens are listed in `main()`. No runtime `.so` plugin loading.
- **`unsafe` is forbidden** workspace-wide (lint, not convention).
- **`Cargo.lock` is committed.** CI uses `--locked`.

## Offline

Dependencies are pinned to what is already in `~/.cargo/registry`; a clean build must need no network.

- Do not bump a version or add a crate without verifying `cargo build --offline` still passes.
- Do not set `net.offline = true` in `.cargo/config.toml`. Pass `--offline` per invocation.
- If a new dependency is genuinely required, fetch it once deliberately, commit `Cargo.lock`, and note it in `docs/architecture.md`.

## Data

- SQLite, WAL, `busy_timeout`, `foreign_keys=ON`, `synchronous=NORMAL`, `PRAGMA application_id` per app (a random stable value — never derived from the brand).
- Open databases **only** through `storage::open_migrated(path, MIGRATIONS)`.
- Migrations: forward-only, `PRAGMA user_version`, one transaction each, embedded with `include_str!` from `apps/<app>/migrations/`.
- **Never edit a shipped migration.** Add a new one.
- A database newer than the binary's max version must refuse to open.
- Version bands (one counter per file): app `1–999`, `finance` `1000–1099`, `party`/`catalog` `2000–2999`, later app tables `10000+`.

Money and stock:

- Money is **integer minor units**. Never float.
- `finance`: `sum(postings.amount) = 0` per entry, enforced inside `post_entry()` in one transaction. Entries are never edited; corrections are reversing entries.
- Inventory: `stock_movements(delta)` only. Stock is `SUM(delta)`, never an absolute quantity column.

## Sync

Local SQLite is the source of truth. The app must be fully functional with zero peers and zero internet.

- One sequencer per tier: hub on the LAN, D1 in the cloud. Devices keep one cursor per authority (`peer_state`).
- Ordering is **Lamport + origin**, never wall-clock time. Merge: incoming wins if `(lamport, origin) > (local.lamport, local.origin)`.
- Append-only tables (`sales`, `payments`, `postings`, `stock_movements`): insert-or-ignore by `id`, no merge logic.
- Ids are UUIDv7, generated per install. Push is idempotent by row `id`.
- Every synced table carries `id`, `tenant_id`, `origin`, `lamport`, `deleted_at`.
- Endpoint paths are brand-free and versioned: `/v1/push`, `/v1/pull`, `/v1/snapshot`, `/v1/reports/*`.
- Do not build: CRDTs, dynamic schema engines, conflict-resolution UI, user accounts, per-tenant databases, realtime sync.

## UI

- `ui-kit` = tokens and theme. `ui-shell` = visible chrome and `Screen<S>`.
- `egui`/`eframe` are pinned at 0.33.3 and only `ui-kit`/`ui-shell` may use them — this keeps the portable crates reusable by the mobile companion.
- Theme is installed once at startup; screens never set global visuals.

## Release

- Version lives in `apps/<app>/Cargo.toml`. Tags: `pos-v1.2.3`, `school-v0.4.0`, `companion-v0.1.2`.
- **Tag by product, never by company.** CI fails if tag version ≠ manifest version.
- Tags are created only from `main`, after green CI. Assets are never committed to git — Releases only.
- Linux builds on `ubuntu-22.04` (glibc floor), `--locked`, `strip` via the release profile.
- Brand strings live in exactly one `APP_NAME` constant per app. Android package id and MSI `UpgradeCode` are permanent once published — never derive them casually and never change them.

## When adding a product

1. `apps/<name>/` with `main.rs`, `db.rs`, `screens.rs`, `migrations/`.
2. Reuse `ui-shell` + `app-boot`; do not copy their code.
3. Own schema, own migration band, own `application_id`.
4. Add a tag prefix and a matrix entry; nothing else in CI changes.
