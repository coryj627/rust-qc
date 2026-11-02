# Organization

## Crates and workspaces

**ORG-1. Split crates on real boundaries.** A separate crate is justified by a separate consumer, a separate dependency set, an FFI or proc-macro boundary, or compile-time isolation. ripgrep splits into `grep-searcher`, `grep-printer`, `ignore`, and others because each is useful alone. Otherwise use modules.

**ORG-2. Workspaces centralize settings.** Put `edition`, `rust-version`, `license`, shared dependency versions, and `[workspace.lints]` in the root `Cargo.toml`; member crates inherit with `edition.workspace = true` and `[lints] workspace = true`. Members live side by side in one folder (`crates/`), as in ripgrep, jiff, and dua-cli.

**ORG-3. New crates use the latest stable edition.** Raise `rust-version` conservatively and only for a feature you use.

**ORG-4. Features are additive.** Enabling a feature never removes API or changes existing behavior. `default` features are what most users need to get started; the crate builds with `--no-default-features`.

## Binary and library split

**ORG-5. Keep `main.rs` thin.** `main` parses arguments, calls a `run` function that returns `anyhow::Result<ExitCode>` (or `Result<()>`), and turns errors into an exit code and message. ripgrep's `crates/core/main.rs` is the model: it also treats `BrokenPipe` as a clean exit, matching Unix convention. Logic that deserves tests lives in modules or a library crate.

## Modules

**ORG-6. Name leaf modules after their primary item.** `line_buffer.rs` holds `LineBuffer`. A module with no primary item gets a name describing its concern (`walk`, `output`, `filter` in fd).

**ORG-7. Non-leaf modules hold structure, not logic.** `lib.rs` and `mod.rs` contain the crate or module doc, `mod` declarations, and re-exports. A small amount of glue is acceptable; an implementation of the public API is not. memchr's `lib.rs` is 221 lines, almost all documentation.

**ORG-8. Balance module size.** A module that needs a table of contents to navigate, or that mixes unrelated concerns, should split. dua-cli's 1,400-line `main.rs` is an example of what to avoid; fd's flat set of 15 focused modules is an example to follow.

**ORG-19. Do not create a module for one small item.** A new module earns its own file when it holds a distinct concept and either several items or a few hundred lines. A single function and its error type belong in the module that uses them (or `lib.rs` of a small crate). Splitting 100 lines into three files adds navigation without adding clarity.

**ORG-9. Prefer `foo.rs` plus `foo/` over `foo/mod.rs`** in new code, unless the crate already uses `mod.rs` consistently.

## Visibility

**ORG-10. Private by default, `pub(crate)` for internal sharing.** Make an item `pub` only when it is part of the API you will support. jiff has hundreds of `pub(crate)` items behind a deliberately small public surface.

**ORG-11. Each public item has one path.** Re-export at the crate root or keep the item in its public module, not both. Mark re-exports `#[doc(inline)]` when the re-export is the canonical path.

**ORG-12. No glob re-exports and no preludes.** `pub use module::*;` makes the API surface invisible and accidentally public. Re-export items by name.

**ORG-13. Do not leak dependency types** in a stable public API unless the dependency is itself part of the contract (for example `std::io::Error`). A leaked type ties your semver to theirs.

## Imports

**ORG-14. Group imports**: `std`/`core`/`alloc`, then external crates, then `crate::`, `super::`, `self::`, each group separated by a blank line.

**ORG-15. No glob imports** except `use super::*;` inside `#[cfg(test)] mod tests` and `use SomeEnum::*;` inside a function dominated by a match on that enum.

**ORG-16. Types by name, functions by module.** Import types and traits and use them unqualified (`Regex`, not `regex::Regex`). Call free functions from other modules through their module (`mem::swap`, `fs::read`) so a bare lowercase call reads as local. Qualify ambiguous common names (`io::Error`, `fmt::Result`).

**ORG-17. Import a trait where it is used** when only one function needs it, and use `as _` when only its methods are needed (`use std::io::Write as _;`, as dua-cli does with clap traits).

## Statics and global state

**ORG-18. Avoid mutable statics and hidden globals.** Pass dependencies explicitly. Use `std::sync::OnceLock` or `LazyLock` for genuinely global, immutable, lazily computed values. Never run code before `main` (no `ctor`).
