# Zebra

A Rust workspace for a family of desktop apps (`pos`, `school`, …) that share a
common look, layout, storage layer, and process bootstrap.

```
crates/    shared libraries
apps/      one binary per product
```

Full rationale, rules, and conventions: **[docs/architecture.md](docs/architecture.md)**.

## Quick start

```bash
cargo run -p pos       # point of sale app
cargo run -p school    # school app
cargo test --workspace
```

First build writes `Cargo.lock`; commit it. Nothing in the manifest requires a
network fetch on this machine — `cargo build --offline` should succeed.

## Where code goes

| You are writing… | It belongs in… |
|---|---|
| A screen, a command, a domain rule for one product | `apps/<product>/src/` |
| A widget every product can use | `crates/ui-kit/src/components/` |
| Header, sidebar, content frame, slots | `crates/ui-shell/src/` |
| Database open/migrate/query plumbing | `crates/storage/src/` |
| Settings file read/write | `crates/config/src/` |
| Logging, panic hook, data directories, exit codes | `crates/app-boot/src/` |

If only one product needs it, it stays in that product. `crates/` is for code
with two or more real consumers.
