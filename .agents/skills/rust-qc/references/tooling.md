# Tooling

## Lint baseline for new projects

Put lint configuration in `Cargo.toml` (`[lints]`, or `[workspace.lints]` with `[lints] workspace = true` in members), not in crate-level attributes, so it is visible in one place and shared across targets. This baseline follows Microsoft's M-STATIC-VERIFICATION, trimmed to lints that catch real problems without forcing churn. Do not add it to an existing project unless the user asks.

```toml
[lints.rust]
missing_docs = "warn"                  # libraries only; remove for binaries
missing_debug_implementations = "warn"
unsafe_op_in_unsafe_fn = "warn"
unused_lifetimes = "warn"
redundant_lifetimes = "warn"
trivial_numeric_casts = "warn"
unreachable_pub = "warn"
# unsafe_code = "forbid"               # uncomment for crates with no unsafe

[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
# Restriction lints that enforce this skill's rules.
allow_attributes_without_reason = "warn"
undocumented_unsafe_blocks = "warn"
unnecessary_safety_comment = "warn"
missing_assert_message = "warn"
unwrap_used = "warn"                   # pair with clippy.toml: allow-unwrap-in-tests = true
dbg_macro = "warn"
todo = "warn"
map_err_ignore = "warn"
clone_on_ref_ptr = "warn"
empty_structs_with_brackets = "warn"
if_then_some_else_none = "warn"
too_long_first_doc_paragraph = "warn"
# Pedantic opt-outs. Each opt-out needs a one-line reason.
module_name_repetitions = "allow"      # conflicts with naming leaf modules after their type
must_use_candidate = "allow"           # too noisy; apply #[must_use] by judgment (API-8)
```

`clippy.toml`:

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
```

Opting out of a lint for one item uses `#[expect(clippy::lint_name, reason = "...")]`. dua-cli's `[workspace.lints.clippy]` table is a working example of a project-wide table with explicit opt-outs.

## rustfmt

New projects need no `rustfmt.toml`; defaults are the Rust Style Guide. If one exists, it sets only `edition` (and `style_edition` when needed). Width and heuristics overrides are a project decision, not a personal one; BurntSushi's `max_width = 79` plus `use_small_heuristics = "max"` is a legitimate choice for an existing project, not a default to copy. Avoid unstable rustfmt options; they require nightly and break contributors on stable.

## CI gate

The minimum gate, matching `scripts/check.sh`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items
cargo test --workspace --all-features
```

Add when they apply:

- `cargo test --no-default-features` for crates with optional features, and `cargo hack check --feature-powerset` for more than a few.
- `cargo miri test` on nightly for crates with `unsafe` (memchr runs it in CI).
- A build with the toolchain pinned to `rust-version` (memchr has an `msrv` job).
- `cargo audit` or `cargo deny check` for dependency advisories and licenses.
- `cargo udeps` or `cargo machete` occasionally for unused dependencies.

`-D warnings` belongs in CI commands, not in `#![deny(warnings)]` in source, which breaks builds whenever a new compiler adds a lint.

## Verdict on the `rust-style-guide` crate

Do not integrate it. Version 0.1.1 (latest 0.1.4, last released December 2024, one maintainer) is a proc-macro that injects a fixed list of lint attributes into the crate root, plus a recommended `rustfmt.toml`. The problems:

1. It requires nightly: the crate root must enable `#![feature(custom_inner_attributes)]` and `#![feature(prelude_import)]`, and its `rustfmt.toml` sets `unstable_features = true`. That breaks every contributor on stable Rust.
2. It hides lint policy inside a dependency. The lints should be visible and editable in `Cargo.toml`'s `[lints]` table, which stable Cargo has supported since 1.74.
3. It adds a build dependency (and `quote`) to the shipped crate for something that is configuration, not code.
4. Several of its choices conflict with the rest of this standard: `max_width = 120` against the style guide's 100, `imports_granularity = "Crate"` against Google's `"Module"`, and `clippy::float_arithmetic` and `clippy::arithmetic_side_effects` as warnings, which flag ordinary math everywhere.
5. Its rule identifiers point to a Chinese-language coding guideline not reviewed here, so findings cannot be traced to a source we evaluated.

Worth keeping from it: its numeric-cast lints (`cast_possible_truncation`, `cast_sign_loss`, `cast_possible_wrap`, `cast_lossless`) are already in clippy's `pedantic` group, which the baseline above enables, and `missing_panics_doc` and `missing_errors_doc` are also pedantic. So the baseline already captures its useful parts without the dependency.
