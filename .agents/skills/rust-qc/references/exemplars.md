# Exemplars

These repositories are the reference for what good Rust looks like in practice. Study the named files when a rule needs a concrete example. They are not perfect; where one breaks a rule, `precedence.md` says so.

## memchr (BurntSushi/memchr)

Low-level, performance-critical, `no_std`, heavy `unsafe` done right.

- `src/lib.rs`: crate root that is almost entirely documentation: purpose, examples for each entry point, feature flags. Logic lives in leaf modules. `#![deny(missing_docs)]`.
- `src/arch/`: platform-specific SIMD code isolated behind safe functions. Every unsafe block carries a `// SAFETY:` comment; there are more safety notes than unsafe sites.
- Testing: inline test modules plus `quickcheck` property tests comparing every vectorized routine against a naive implementation; Miri in CI; feature matrix (`--no-default-features`, `alloc`, `std`) in CI.
- `rustfmt.toml`: `max_width = 79`, `use_small_heuristics = "max"`.

## jiff (BurntSushi/jiff)

Large, carefully designed public API with exhaustive documentation.

- `crates/jiff/src/error/mod.rs`: one crate-wide `Error` struct, internals private behind `Arc<ErrorInner>`, predicate methods, and a `# Design` doc section explaining the trade-off with a link to the discussion.
- Docs: about 700 `# Example` sections and about 800 doc tests using the hidden `# Ok::<(), Box<dyn std::error::Error>>(())` line instead of `unwrap`. `# Errors` and `# Panics` sections are consistent.
- Visibility: hundreds of `pub(crate)` items behind a deliberately small public surface.
- Workspace of focused crates under `crates/` (core, tzdb, integration crates for sqlx, diesel, ICU), each with its own purpose and dependencies.
- `DESIGN.md` and `COMPARE.md` keep design rationale out of rustdoc.

## ripgrep (BurntSushi/ripgrep)

Application built on a set of reusable library crates.

- `crates/core/main.rs`: thin `main` that calls `run`, maps errors to exit codes, and treats `BrokenPipe` as success. Comments explain decisions (the musl allocator comment is the model for "why" comments).
- `crates/searcher`, `crates/printer`, `crates/ignore`: library crates with `#![deny(missing_docs)]`, re-exports in `lib.rs`, and logic in leaf modules.
- `tests/`: integration tests split by feature area, an `rgtest!` macro for setup, `eqnice!` for readable diffs, and `regression.rs` with over 100 tests named after issue numbers.
- CI runs `cargo fmt --check` and `cargo doc` with `RUSTDOCFLAGS=-D warnings` and `--document-private-items`.

## fd (sharkdp/fd)

Mid-sized CLI with a clean flat module layout.

- `src/main.rs`: 15 focused modules declared at the top (`walk`, `filter`, `output`, `exec`, `config`), imports grouped std, external, crate.
- `anyhow` for application errors with `Context`; a small `error.rs` that sanitizes untrusted text before printing it to the terminal.
- `tests/testenv/`: a reusable test environment for integration tests.
- Edition 2024, explicit `rust-version`, default `rustfmt.toml`.

## dua-cli (Byron/dua-cli)

Workspace with a shared lint table and snapshot tests.

- `Cargo.toml`: `[workspace.lints.clippy]` with explicit, named opt-outs, inherited by members with `[lints] workspace = true`.
- `src/lib.rs`: `#![deny(unsafe_code)]`, `#![deny(missing_docs)]`; `src/main.rs`: `#![forbid(rust_2018_idioms, unsafe_code)]`.
- `tests/`: snapshot tests with stored fixtures.
- Counter-examples to avoid: `pub use common::*;` (glob re-export) in `lib.rs`, and a 1,400-line `main.rs` that should be split into modules.

## Shared traits across all five

1. rustfmt-clean, enforced in CI.
2. Public items documented; libraries deny `missing_docs`.
3. Thin entry points; logic in focused leaf modules.
4. Comments explain decisions and constraints, rarely what the code does.
5. Errors: concrete types in libraries, `anyhow` with context in binaries.
6. Inline unit tests plus integration tests under `tests/`, with regression tests for reported bugs.
7. Explicit `edition` and `rust-version`.
8. memchr, jiff, and ripgrep carry an `AI_POLICY.md` that requires a human who understands AI-produced work and forbids autonomous agent contributions. Check for such a policy before preparing changes for any upstream repository.
