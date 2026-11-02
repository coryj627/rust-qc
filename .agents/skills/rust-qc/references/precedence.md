# Precedence and conflict decisions

## Ranking

When rules conflict, the higher item wins:

1. The project's own configuration and written conventions (`rustfmt.toml`, `clippy.toml`, `[lints]`, `CONTRIBUTING.md`, `AI_POLICY.md`).
2. The established convention of the file or module being edited.
3. Soundness and correctness rules in this skill (never overridden by convenience or local habit).
4. This skill's defaults, which synthesize the sources below.
5. The Rust Style Guide (formatting), as implemented by rustfmt defaults.
6. The Rust API Guidelines, Microsoft Pragmatic Rust Guidelines 2026.6, and Google's Rust Style Guide.

Level 3 sits above level 2 on purpose: matching a file's habit of undocumented `unsafe` or panicking on user input is not consistency, it is copying a defect. Report it instead.

## Decisions on known conflicts

**Line width.** The Rust Style Guide and rustfmt default to 100 columns. BurntSushi's crates (memchr, jiff, ripgrep) use 79 with `use_small_heuristics = "max"`. The rust-style-guide crate uses 120. Decision: honor an existing `rustfmt.toml`. New projects use rustfmt defaults with no width override; the default exists so nobody debates it.

**Clippy strictness.** Microsoft recommends warning on all major clippy groups including `pedantic`, plus selected `restriction` lints. BurntSushi's crates do not gate on clippy at all and rely on rustfmt, rustdoc with `-D warnings`, broad tests, feature-matrix builds, and miri. dua-cli enables a workspace lint table with documented opt-outs. Decision: new projects adopt the `[lints]` baseline in `tooling.md`. Existing projects keep their configuration; do not add lint configuration to someone else's project unless asked. In an audit, recommend the baseline as a Note.

**`# Example` versus `# Examples`.** The API Guidelines and Microsoft use `# Examples`. BurntSushi's crates use `# Example` (about 700 occurrences against about 20). Decision: match the crate. Default to `# Examples` in new crates.

**`unwrap` versus `expect`.** Google prefers `expect` over `unwrap` when a panic is required. The awesome-copilot instructions say avoid both. jiff and ripgrep contain many `unwrap` calls, nearly all inside inline test modules. Decision: production code uses `?`, pattern matching, or `expect` with a message explaining the invariant; tests may `unwrap`.

**Error type design.** Microsoft wants situation-specific error structs with a private kind, `is_*` predicates, and a captured `Backtrace`. jiff uses one crate-wide `Error` struct wrapping a private `Arc<ErrorInner>`, documented as a deliberate choice after finer-grained types failed to compose. The awesome-copilot instructions suggest `thiserror`. Decision: libraries expose concrete error types with private fields and a typed way to branch (a `kind()` returning a public `#[non_exhaustive]` enum, or `is_*` predicates); a private kind with no accessor is not acceptable; one crate-wide `Error` is fine for small or tightly composed crates, separate types when operations share no failure modes. `thiserror` is acceptable for `Display` and `Error` boilerplate. Capture a `Backtrace` in services and long-running applications where operators debug from logs; skip it in `no_std` and hot-path libraries. Applications use `anyhow`.

**Import granularity.** Google requires one module per `use` statement (`imports_granularity = "Module"`). The rust-style-guide crate uses `"Crate"`. ripgrep nests (`use std::{io::Write, process::ExitCode};`); fd writes one path per line. Decision: match the file. New code: one `use` per module path, grouped std, external, then `crate`/`super`, separated by blank lines. These rustfmt options are unstable, so this is enforced by review, not tooling.

**Glob imports and re-exports.** Google and Microsoft reject glob imports and glob re-exports. dua-cli has `pub use common::*;` in `lib.rs`. Decision: no glob imports except `use super::*;` in test modules and `use Enum::*;` inside a function that is mostly a match on that enum. No `pub use module::*;`. The exemplar is wrong here.

**`Rc`/`Arc`.** Google says avoid shared ownership unless the alternative is unreasonably complex. Microsoft says services are `Clone`, typically through an inner `Arc`. Decision: default to unique ownership and borrowing. Use `Arc` for cheap-to-clone handles shared across threads (jiff's `Error` uses `Arc` precisely to be cheaply cloneable) and when the borrow structure would otherwise be contorted. Clone the `Arc` with `Arc::clone(&x)` so readers see it is a pointer copy.

**Property-testing dependencies.** memchr and jiff use `quickcheck`; the API Guidelines are silent. Decision: start with zero-dependency checks (exhaustive enumeration of short inputs, a seeded loop against a naive reference). Add `quickcheck` or `proptest` only when the crate already uses one or failures need shrinking to debug. In evaluation, adding `proptest` (about 50 crates in `Cargo.lock`) to a 100-line parser was judged a cost with no gain over an exhaustive loop.

**Small crates.** Microsoft and Google (for Chromium) push toward splitting crates. ripgrep and jiff are workspaces of focused crates; fd and dua-cli are mostly single crates. Decision: split when a part has a separate consumer, a separate dependency set, or would benefit from separate compilation; otherwise use modules. Do not split preemptively.

**`#[non_exhaustive]`.** Google applies it to public enums and to public structs with public fields used across teams. Decision: apply to public library enums and all-public-field structs that may grow; skip for closed sets (`Ordering`-style enums, geometric types) and for option structs meant for struct-update syntax, which use private fields plus `Default` or a builder instead.

**Async.** Google forbids async in Chromium; Microsoft gives async guidance. Decision: no rule against async. When used, prefer `async fn` over returning `impl Future`, and give long CPU-bound loops yield points.

**Allocator.** Microsoft recommends mimalloc for applications; ripgrep uses jemalloc only on musl and explains why in a comment. Decision: no default. Change the global allocator only with a measured reason, and document it the way ripgrep does.
