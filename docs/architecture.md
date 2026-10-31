# Stride — workspace architecture

Stride is three Cargo repos. `stride-libs` (this one) holds the crates the
products share; `stride-pos` and `stride-school` each hold one product. Products
depend on this repo at a pinned commit; this repo never depends on a product.

This document explains the layout, the rules that keep it healthy, and the
conventions each crate follows. It is the reference for "where does this code
go?".

---

## 1. Layout

```
stride-libs/
├── Cargo.toml              virtual workspace manifest (no [package])
├── Cargo.lock              committed — see §8
├── rust-toolchain.toml     tracks the stable channel
├── AGENTS.md               rules + the rev-bump procedure
├── docs/architecture.md    this file
└── crates/                 shared libraries
    ├── ui-kit/             tokens + theme (+ widgets and icons behind features)
    ├── ui-shell/           layout regions and the ready-made `Shell`
    ├── storage/            SQLite open / pragmas / migrations
    ├── config/             JSON settings read + atomic write
    └── app-boot/           logging, panic hook, directories, exit codes
```

`members = ["crates/*"]` means **adding a crate never requires editing the root
manifest**. Every directory directly under `crates/` must contain a
`Cargo.toml`: Cargo errors when a member glob matches nothing, so an empty
`crates/` is not a valid workspace.

Products are separate repos with the same `crates/` + `apps/<name>/` shape, so a
product can grow internal crates later without restructuring. See §9.

---

## 2. Who owns what

| Crate | Kind | Job | Must never contain |
|---|---|---|---|
| `ui-kit` | lib | Design tokens, palettes, theme; widget primitives behind `widgets`; a curated icon vocabulary behind `icons` | Domain types, I/O, layout decisions |
| `ui-shell` | lib | Layout regions (`rail`, `Sidebar`, `status`, `workspace`, `Palette`) and the assembled `Shell` on top of them | Product screens, domain state, storage |
| `storage` | lib | Opening SQLite, pragmas, `user_version` migrations | Product tables, repositories, queries |
| `config` | lib | Reading/atomically writing a JSON settings file | A settings schema, theme knowledge |
| `app-boot` | lib | Directories, logging, panic hook, exit codes, `Context` | Anything a single app needs |

Screens, schema, settings and business rules live in the product repos. Nothing
here may import them, and no product may be imported here.

Rules of thumb:

- **Presentation** → `ui-kit` / `ui-shell`.
- **Persistence mechanics** → `storage`; **what is stored** → the product.
- **Process behaviour** → `app-boot`.
- **Business rules** → the product. Always.

### Why there is no `core` crate

The obvious first draft is a single `crates/core` holding "common" code. It
always rots: it gains a database handle, then a theme, then an order type, and
within a month every product depends on everything and nothing can be removed.

This layout starts with narrow crates instead. If something is shared and has no
obvious home, that is a signal to name the concern (`money`, `printing`,
`sync-engine`) rather than to open a bucket.

---

## 3. Dependency graph

```
stride-pos ─┐        stride-school
            ├─→ ui-shell ─→ ui-kit ─→ egui (+ egui_extras, egui-lucide via `icons`)
            ├─→ ui-kit
            ├─→ storage ─→ rusqlite
            ├─→ config  ─→ serde_json
            └─→ app-boot ─→ tracing, directories
```

Arrows only ever point **from a product into these crates**, never between
products, never back from a crate into a product. `app-boot` depends on no other
Stride crate — that independence is what stops it becoming the new `core`.

---

## 4. The UI model: shared chrome, pluggable content

`ui-shell` owns the frame. A product supplies:

- **screens** — `impl Screen<State>`, registered in sidebar order;
- **header actions** — closures bound to app state;
- **a status bar** — a closure deriving one line from app state.

```rust
Shell::new("Stride POS")
    .theme(theme)
    .persist_in(ctx.dirs.config.clone())
    .screen(SalesScreen)
    .screen(StockScreen)
    .header_action("Clear cart", |state| state.cart.clear())
    .status_bar(|state| format!("{} line item(s)", state.cart.len()))
    .run(state)?;
```

Design decisions behind this:

- **State is one app-level struct**, passed to each screen as `&mut State`.
  The shell is generic over it, so there is no `Box<dyn Any>` downcasting and the
  borrow checker still helps. Splitting state per screen is a later option, not
  a starting point.
- **Slots, not trait growth.** A new header button is a value passed to
  `header_action`, not a new method on `Screen`. Methods added to `Screen` are
  paid for by every product.
- **`ui-shell` is opt-in.** `ui-kit` is the floor. A product that needs kiosk
  mode or a full-screen dashboard builds its layout from `ui-kit` primitives and
  skips `ui-shell`; it does not add `Screen::is_kiosk()`.
- **Composition is compile-time.** Screens are listed in `main`. There is no
  runtime plugin loading, no `.so` scanning, no stable ABI. If third-party
  extensibility is ever required, that is a separate WASM host crate, not a
  change to this layout.

The shell remembers the **active screen** in `<config dir>/shell.json`. Window
geometry and sidebar width are *not* persisted yet: that would mean enabling
eframe's `persistence` feature (extra dependencies) and a second file format.
Add it deliberately when it is missed.

### 4.1 `Shell` is one arrangement, not the only one

`Shell` is a *composition* of small region functions, not a thing POS has to
inherit. `ui_shell::regions` exposes the pieces:

| Function / type | Draws |
|---|---|
| `rail(ui, items, selected, on_click)` | the narrow icon activity bar |
| `Sidebar::new(width).fixed()` / `Sidebar::default()` | a contextual sidebar, resizable when not fixed |
| `status(ui, …)` / `status_item` / `status_end` | the bottom strip and its two clusters |
| `workspace(ui, …)` | the central panel |
| `Palette` + `Command<S>` | `Ctrl+P` command palette over app state |
| `run_app(title, size, builder)` | the eframe entry point every product shares |

POS assembles them itself in its own repo: no header, a 52 px activity rail, a
sidebar owned by the selected module, a cart column inside the Sell workspace,
and a 28 px status bar. `Shell` remains untouched for school, which wants a
header and a list of screens.

The rule this encodes: **chrome is shared, the skeleton is not.** A product that
needs a different arrangement composes regions; it never adds a flag to `Shell`
that every other product then pays for.

Module lists, hotkeys, and prefs are product-side: POS keeps
`{ "module": <index>, "sidebar": [<bool>; N] }` in `<config>/pos/prefs.json`,
deliberately separate from the shop's `settings.json` and from
`ui-shell`'s `shell.json` (one person's window habits are not shop
configuration). An index from a build with more modules is clamped on load.

---

## 5. UI split: tokens vs widgets (a feature, not a crate)

`ui-kit` can be depended on two ways:

```toml
ui-kit.workspace = true                              # tokens + theme only, no renderer
ui-kit = { workspace = true, features = ["widgets"] } # + widget primitives + egui
```

`ui-shell` always enables `widgets`, so every desktop product gets them by
feature unification. A future daemon, CLI admin tool, or receipt/PDF renderer can
depend on `ui-kit` and get brand colours without compiling egui, winit, or wgpu.

This mirrors egui's own split (`egui` core vs `eframe` backend) without a second
manifest. Split out a `ui-egui` crate only when a **second** renderer target
appears (web, PDF, printer), not before.

Tokens are deliberately framework-free (`Rgb`, `Palette`, `Spacing`, `Radii`);
conversion to `egui::Color32` happens at the edge, behind `widgets`.

### 5.1 Icons

`ui-kit`'s `icons` feature (which implies `widgets`) exposes a curated `Icon`
newtype and constants (`Icon::CART`, `Icon::PACKAGE`, …) backed by
`egui-lucide` + `egui_extras`' SVG loader. Products name icons from that
vocabulary and never import the icon crate, so swapping the set is one crate's
work rather than a search across products. `theme::install` registers the image
loaders once at startup; without it every icon would silently paint nothing.

---

## 6. Persistence conventions

**Database** (`storage`):

- One SQLite file per app, under the app's data directory (`pos.db`,
  `school.db`).
- `WAL` journal, `foreign_keys = ON`, `busy_timeout = 5000`,
  `synchronous = NORMAL`, set in `prepare`. NORMAL is the right trade for a WAL
  till: `FULL` costs a synchronous write per commit, and WAL already survives a
  process crash.
- Each app **claims** its file with `store.claim_application_id(id)`
  immediately after opening and before migrating. The value lives in the SQLite
  header, so it reads before any table exists and survives the file being
  copied; a database already stamped by a different app is refused instead of
  being half-migrated. The ids (`pos` `1836624513`, `school` `2014884902`) are
  arbitrary and permanent — never recompute them from the app's name.
- Migrations are forward-only slices of `Migration { version, name, sql }` owned
  by the product (`apps/<app>/src/db.rs` in its own repo), applied in ascending
  version inside one transaction each, tracked with SQLite's `user_version`.
- **Never edit a migration that has shipped.** Append `version: 2`.
- There is no shared repository layer: each app queries through
  `store.conn()`.

**Settings** (`config`):

- One JSON file per app under the config directory (`settings.json`).
- Each app declares its own `Settings` struct with `#[serde(default)]` so old
  files keep working when fields are added.
- Writes go to `<name>.tmp` then rename, so a crash cannot leave a file that
  fails to parse on next start.
- Missing file = first run, not an error: use `load_or_default`.

**Shell state** (`ui-shell`): `shell.json`, only `{ "active": <screen index> }`.
Corrupt or missing state is ignored; shell preferences must never block startup.

---

## 7. Process conventions

`app_boot::run("pos", |ctx| { … })` gives each app:

1. **Directories** resolved per app name via `directories::ProjectDirs` →
   `~/.local/share/pos`, `~/.config/pos`, `~/.cache/pos`, and
   `~/.local/share/pos/logs` on Linux. Per-app namespacing is why two products
   can never collide on disk.
2. **Logging**: `tracing` with `RUST_LOG` override (default `info`), to stderr
   and to `<data>/logs/<app>.log`. No rotation yet — see §10.
3. **Panic hook**: logs the panic, prints a pointer to the log file, then chains
   to the default hook (which still prints the backtrace). Important because a
   GUI app launched from a desktop shortcut has no terminal.
4. **Exit codes**: `0` on success, `1` on failure, with the error logged and
   printed as `app: message` using anyhow's `{:#}` chain.

`main` stays this thin:

```rust
fn main() -> ExitCode {
    app_boot::run("pos", |ctx| {
        /* open db, load settings, build state, Shell::run */
        Ok(())
    })
}
```

---

## 8. Dependencies and offline builds

Every version in `[workspace.dependencies]` is pinned to a version already
present in `~/.cargo/registry` on this machine, so a clean build fetches nothing:

| Crate | Pinned | Notes |
|---|---|---|
| `eframe` / `egui` | `0.36.2` | `default-features = false`: `default_fonts`, `glow`, `links`, `wayland`, `x11`. 0.36 defaults to wgpu, and taking the default would swap the renderer silently |
| `egui_extras` | `0.36.2` | `svg` only — needed to rasterise icon paths |
| `egui-lucide` | `0.1.0` | its `egui` requirement is `>=0.36,<0.37`, so it is the pin that holds the series |
| `rusqlite` | `0.32.1` | `bundled` — compiles SQLite from source in the crate, no download |
| `serde` / `serde_json` | `1.0.229` / `1.0.154` | |
| `anyhow` / `thiserror` | `1.0.104` / `2.0.21` | |
| `tracing` / `tracing-subscriber` | `0.1.44` / `0.3.23` | `env-filter` |
| `directories` | `6.0.0` | |
| `clap` | `4.6.7` | not used yet — there for CLI tooling |

`rust-version` is `1.95` (edition 2024). `rust-toolchain.toml` tracks `stable`
so the file documents intent without forcing a toolchain download.

Two dependencies were fetched **deliberately** for the 0.36 bump, because they
were not in the local registry: `accesskit 0.24.1` (a non-optional dependency of
`egui-winit` 0.36) and `resvg 0.45.1` (pulled by `egui_extras`' `svg` feature).
Anything further must be fetched the same way — once, on purpose, with
`Cargo.lock` committed and this list updated. eframe's own `accesskit` feature
stays **off**: it would need `accesskit_winit`, which has not been fetched.
Screen-reader support is a decision to take on its own, not a side effect.

Rules:

- **Declare once per repo, consume everywhere in it.** Third-party versions live
  only in a repo's root manifest; members write `name.workspace = true`. Never a
  bare version in a member manifest.
- **The product repos carry a copy of this table.** There is no shared manifest
  across repos, so a third-party bump is one commit here and one in each product
  — keep the copies aligned, or the two products drift onto different `serde`
  builds. A libs `rev` bump is a separate, more frequent operation; see
  `AGENTS.md`.
- **Commit `Cargo.lock`.** These repos ship binaries, so the lockfile is the
  record of what was built. Build with `--locked` in CI.
- **Verify offline after any manifest change**: `cargo build --offline`. If it
  succeeds, no network was used. If it fails, cargo names the exact crate and
  requirement it could not satisfy — spend the download deliberately.
- **Adding a dependency** is a three-step decision: is it already in
  `~/.cargo/registry` (`ls ~/.cargo/registry/cache/index.crates.io-*/ | grep <name>`),
  do two products need it, and does it belong here (shared) or in one product's
  own manifest. If two products need it, it belongs in this repo as a crate or
  behind an existing one — not copy-pasted into both.
- Do **not** set `net.offline = true` in `.cargo/config.toml`; that makes adding
  a dependency painful. Use `--offline` per command.

`cargo clean` only deletes `target/`; it never re-downloads crates, because the
registry cache is shared and separate.

---

## 9. Adding things

**New shared crate**

```bash
cargo new --lib --vcs none crates/printing
# then one line in [workspace.dependencies]:
#   printing = { path = "crates/printing" }
```

Promotion rule: a crate enters `crates/` when a **second** product needs it, or
when it is infrastructure whose absence would mean copy-paste (as with
`app-boot`). Never pre-emptively, never "because it looks reusable".

**New product** — a new repo, not a directory here:

1. `stride-<name>` with the same shape: `Cargo.toml`, `Cargo.lock`, `.gitignore`,
   `AGENTS.md`, `apps/<name>/`, `crates/`.
2. Pin all five crates to one `stride-libs` `rev` in its
   `[workspace.dependencies]`, and copy the third-party table from §8.
3. Own schema, own migration band, own `application_id`, own version in
   `apps/<name>/Cargo.toml`, own tag prefix `<name>-v`.
4. `crates/*` joins its members list when its first internal crate lands.

**New screen or module** — in the product repo, not here.

---

## 10. Deliberately absent (and when to add it)

| Not present | Why | Add when |
|---|---|---|
| Single-instance lock | Needs `fs4`, not in the local cache; stale-lock handling is its own design problem | Two windows writing one SQLite file becomes a real risk |
| Log rotation | Needs `tracing-appender`, not cached | Log files grow past a few MB |
| Crash dialog | Needs `rfd`, not cached | A GUI app ships to non-technical users |
| Screen-reader support | eframe's `accesskit` feature needs `accesskit_winit`, not fetched | Accessibility becomes a requirement, not a side effect of a bump |
| Window geometry persistence | Needs eframe's `persistence` feature | Users complain the window forgets its size |
| Auto-update | Network + signing + platform installers | After the first real release |
| Runtime plugins (`.so`) | Requires a stable ABI / WASM host; would dominate the design | Third-party extensibility is a product requirement |
| CI workflow | Not present in any of the three repos yet; `cargo test` and `clippy -D warnings` are run by hand | Before the first shared deployment |
| `clap` CLI entry per app | Dependency is pinned but unused | An app needs headless/migration subcommands |

Each row lists the missing dependency, so the offline-build policy in §8 can be
checked before adding it.

---

## 11. Known gaps

Product gaps — the placeholder catalogue, unimplemented pay, missing sync, and
school's placeholder screens — are tracked in `stride-pos` and `stride-school`,
where the code is. This repo has no CI workflow and no release automation yet;
see §10.

`#[allow(dead_code)]` is avoided on purpose: if a scaffolded item is unused,
delete it rather than silence the warning.

---

## 12. Commands

```bash
cargo build --offline                    # prove no network is needed
cargo check                              # the day-to-day loop
cargo test --offline --locked
cargo clippy --offline --all-targets -- -D warnings
cargo fmt --all
```

There are no binaries here. App commands (`cargo run -p pos`, `cargo build
--release`) run in the product repo, which is also where the release profile and
the app's runtime paths (`~/.local/share/pos/…`, `~/.config/pos/…`) apply.
