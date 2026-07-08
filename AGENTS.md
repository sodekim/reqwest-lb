# AGENTS.md

## What this repo is
A single Rust library crate (`reqwest-lb`) providing `LoadBalancerMiddleware` for
`reqwest-middleware`. It routes `lb://<host>` requests to a registry of real URLs.

## Build / test / verify
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` (standard Cargo; no custom task runner or CI config present in repo).
- Run one test: `cargo test --test load_balancer_discovery` (integration tests live in `tests/`, named by file).
- Run one unit: `cargo test <test_name>`.
- Examples: `cargo run --example create_from_fixed`, `create_from_discovery`, `create_from_nacos`.

## Core mental model (not obvious from filenames)
- The `lb://` URL scheme replaces `http(s)://`. The host part is the **registry key**, not a DNS name — backends are registered against that key via `LoadBalancerRegistry::add(key, load_balancer)`.
- `LoadBalancerPolicy` defaults to `RoundRobin`; also `Random`, `First`, `Last`, and `Dynamic` (closure `Fn(&[Weighted<I>], &mut Extensions) -> usize`).

## Gotchas
- **Discovery supplier requires a `Change::Initialized` event.** Without it the load balancer does not consider elements ready and selection fails. See `tests/load_balancer_discovery.rs` and the README discovery section.
- Tests in `tests/load_balancer_retry.rs` make real network calls to `rust-lang.org` (and an intentionally bad host); they need internet and are slower. `create_from_nacos` dev-dep pulls `nacos-sdk` — requires a running Nacos server.
- `dev-dependencies` enable `full` tokio features; the library itself pins only `tokio` `sync`. Don't assume other tokio features are available in library code.
- `edition = "2021"`, license via `license-file = "LICENSE"` (Apache-2.0). `cargo package` will fail if `LICENSE` is missing.

## Module layout (`src/`)
- `lib.rs` re-exports `load_balancer::*` and `middleware::*`.
- `pub` modules: `discovery`, `runtime`, `supplier`. `runtime::Tokio` is the runtime marker used by suppliers.
