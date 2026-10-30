# AGENTS.md

Shared plumbing for the Stride products. No product domain, ever.

## Repos

Three repos. Direction is one-way: `stride-pos`/`stride-school` → `stride-libs`.
Never product → product.

| Repo | Holds |
|---|---|
| `stride-libs` (this one) | ui-kit, ui-shell, storage, config, app-boot |
| `stride-pos` | the till: workspace, lockfile, migrations, releases |
| `stride-school` | school admin: same |

Repo names carry the `stride-` prefix. Crate names never do: `ui-kit`, not `stride-ui-kit`.

## Layout

```
crates/           shared plumbing — no product domain
  ui-kit          tokens + theme; widgets behind off-by-default feature
  ui-shell        header / sidebar / content, Screen<S>, Shell<S>
  storage         Store, Migration, user_version runner, pragmas
  config          JSON settings, atomic tmp+rename save
  app-boot        init, Context, Dirs, logging, crash hook
docs/architecture.md
```

`egui`/`eframe` are pinned and only `ui-kit`/`ui-shell` may use them.

## How a shared change reaches the products

A `rev` is a bookmark to one exact snapshot. Cargo reads the bookmark, never "latest".
So a libs change is always **two commits in two repos**.

### Step 1 — land the change here

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --locked
git add -A && git commit -m "ui-kit: <what changed>"
git push origin main
git rev-parse HEAD        # copy this 40-char SHA
```

Do not start step 2 until CI on `main` is green.

### Step 2 — move the bookmark in each product that needs it

```sh
cd ~/Desktop/stride-pos
SHA=<the SHA from step 1>
sed -i -E "s|(stride-libs\", rev = \")[0-9a-f]{40}|\1$SHA|g" Cargo.toml
grep -c "$SHA" Cargo.toml     # must print 5
cargo check
git add Cargo.toml Cargo.lock
git commit -m "chore(deps): stride-libs ${SHA:0:8}"
git push
```

`Cargo.lock` goes in the same commit: CI builds `--locked`, and the lock is where the
resolved SHA is recorded.

### Which products to bump

- `ui-kit`, `ui-shell` — bump whenever convenient. Visual only, safe to lag.
- `storage`, `config`, `app-boot` — bump **both products in the same week**, and open an
  existing database and settings file afterwards. These touch shipped data.

### Rules

- Never write a `rev` that is not a pushed commit on `stride-libs` `main`.
- Never use `branch =` or `tag =` for these deps. Never `cargo update` them.
- Never force-push or rebase `stride-libs` `main`. A shipped tag must still resolve its SHA.
- If `cargo check` fails after a bump: fix forward here (new commit, new SHA) or revert the
  `rev`. Never patch the product to hide a libs bug.
- A `rev` missing from the git cache needs one online fetch. `--offline` works after that.

### Local override — only when iterating without pushing

Gitignored `<product>/.cargo/config.toml`:

```toml
[patch."https://github.com/OWNER/stride-libs"]
ui-kit = { path = "../stride/crates/ui-kit" }
```

The key must be the **git URL**, not `crates-io` — patching `crates-io` for a git dependency
silently does nothing. It rewrites `Cargo.lock`; `git checkout Cargo.lock` before committing.

## Commands

```sh
cargo build --offline            # must always work once the rev is fetched
cargo check                      # the default local loop
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

## Hard rules

- **No `crates/core`.** Shared crates must be narrow and purpose-specific.
- **No brand prefixes on crate names.** Package names are brand-free on purpose.
- **No shared domain model.** If two apps need it, it is plumbing. If it is domain, it lives
  in the app.
- **`unsafe` is forbidden** workspace-wide (lint, not convention).
- **`Cargo.lock` is committed.** CI uses `--locked`.

## Data

- SQLite, WAL, `busy_timeout`, `foreign_keys=ON`, `synchronous=NORMAL`,
  `PRAGMA application_id` per app (a random stable value — never derived from the brand).
- Open databases **only** through `storage::open_migrated(path, MIGRATIONS)`.
- Migrations: forward-only, `PRAGMA user_version`, one transaction each, embedded with
  `include_str!` from `apps/<app>/migrations/`.
- **Never edit a shipped migration.** Add a new one.
- A database newer than the binary's max version must refuse to open.
- Version bands (one counter per file): app `1–999`, `finance` `1000–1099`,
  `party`/`catalog` `2000–2999`, later app tables `10000+`.

Money and stock:

- Money is **integer minor units**. Never float.
- `finance`: `sum(postings.amount) = 0` per entry, enforced inside `post_entry()` in one
  transaction. Entries are never edited; corrections are reversing entries.
- Inventory: `stock_movements(delta)` only. Stock is `SUM(delta)`, never an absolute
  quantity column.

The ledger's tables and invariants stay in each app — each install owns its database. Only
pure money math (formatting, rounding, VAT) may live here.

## Sync

Local SQLite is the source of truth. The app must be fully functional with zero peers and
zero internet.

- One sequencer per tier: hub on the LAN, D1 in the cloud. Devices keep one cursor per
  authority (`peer_state`).
- Ordering is **Lamport + origin**, never wall-clock time. Merge: incoming wins if
  `(lamport, origin) > (local.lamport, local.origin)`.
- Append-only tables (`sales`, `payments`, `postings`, `stock_movements`): insert-or-ignore
  by `id`, no merge logic.
- Ids are UUIDv7, generated per install. Push is idempotent by row `id`.
- Every synced table carries `id`, `tenant_id`, `origin`, `lamport`, `deleted_at`.
- Endpoint paths are brand-free and versioned: `/v1/push`, `/v1/pull`, `/v1/snapshot`,
  `/v1/reports/*`.
- Do not build: CRDTs, dynamic schema engines, conflict-resolution UI, user accounts,
  per-tenant databases, realtime sync.

## UI

- `ui-kit` = tokens and theme. `ui-shell` = visible chrome and `Screen<S>`.
- Theme is installed once at startup; screens never set global visuals.

## Release

- `stride-libs` has no tags and no version numbers. Crates stay `0.1.0`. The pinned SHA is
  the version.
- Products tag from their own repo: `pos-v1.2.3`, `school-v0.4.0`, always from `main`
  after green CI, always `--locked`.
