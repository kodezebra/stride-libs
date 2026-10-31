# Stride libs

Shared Rust crates for the Stride desktop products. This is one of three repos:

| Repo | Holds |
|---|---|
| `stride-libs` (this one) | `ui-kit`, `ui-shell`, `storage`, `config`, `money`, `app-boot` |
| `stride-pos` | the till: screens, schema, migrations, releases |
| `stride-school` | school admin: same |

Products consume these crates from GitHub at a pinned commit (`rev = "<sha>"`).
Nothing here knows about a product, and nothing here depends on one.

Full rationale and rules: **[docs/architecture.md](docs/architecture.md)** and
**[AGENTS.md](AGENTS.md)**.

## Quick start

```bash
cargo build --offline   # must always work: deps are pinned to the local registry
cargo test --locked
```

There are no binaries in this repo. To run an app, clone its product repo and
build there.

## Where code goes

| You are writing… | It belongs in… |
|---|---|
| A widget every product can use | `crates/ui-kit/src/components/` |
| Header, sidebar, content frame, slots | `crates/ui-shell/src/` |
| Database open/migrate/query plumbing | `crates/storage/src/` |
| Amounts, currency, parsing and formatting money | `crates/money/src/` |
| Settings file read/write | `crates/config/src/` |
| Logging, panic hook, data directories, exit codes | `crates/app-boot/src/` |
| A screen, a command, a domain rule for one product | that product's repo, not here |

If only one product needs it, it stays in that product. `crates/` is for code
with two or more real consumers.
