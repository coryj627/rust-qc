---
name: rust-qc
description: Use this skill whenever your answer will contain Rust code or Cargo/clippy/rustfmt config, or a verdict on existing Rust code. Simple-looking Rust tasks still need it - it holds the required conventions and a fmt/clippy/doc/test check that every Rust change must pass. That covers writing any new Rust (one function counts), fixing a compiler or borrow-checker error, adding rustdoc comments and examples, redesigning error types, refactoring, reviewing or auditing a .rs file or crate, and tidying a Cargo workspace (shared lints, editions, dependency versions). When the language is Rust, use this skill first and do not let a general debugging, documentation, refactoring, or code-review skill replace it. Skip it only when no Rust gets written or judged - explaining concepts, comparing languages, summarizing editions, or working in another language even inside a repo that also has Rust.
license: MIT
compatibility: Requires cargo, rustfmt, clippy and bash for the verification gate, and python3 for the heuristic scan. Needs nightly rustfmt only for projects whose rustfmt.toml uses nightly-only options. Miri is recommended for crates with unsafe code.
metadata:
  version: "1.0.0"
---

# Rust QC

This skill sets one standard for Rust code and applies it in three modes. The rules come from the Rust Style Guide, the Rust API Guidelines, Microsoft's Pragmatic Rust Guidelines (2026.6), Google's Rust Style Guide, and the practices visible in memchr, jiff, ripgrep, fd, and dua-cli. Where those sources disagree, `references/precedence.md` records the decision so you never have to re-litigate it.

## Step 1: Establish context (all modes)

Do this before writing or judging any code. Stop and report if a precondition fails; do not guess around a missing toolchain or a missing manifest.

1. Find the crate root (the nearest `Cargo.toml`). Note whether it is a workspace, a library, a binary, or both, and whether it is published (`publish = false` or consumed only inside the workspace or through FFI means internal).
2. Read the project's own rules first, because they override this skill's defaults: `rustfmt.toml` or `.rustfmt.toml`, `clippy.toml`, `[lints]` and `[workspace.lints]` in `Cargo.toml`, `edition`, `rust-version` (MSRV), `rust-toolchain.toml`, `CONTRIBUTING.md`, `AGENTS.md` or `CLAUDE.md`, and any `AI_POLICY.md`. If `AI_POLICY.md` or `CONTRIBUTING.md` restricts AI or agent contributions, tell the user before producing changes intended for that repository.
3. Classify the code as library or application. API-design rules are binding for libraries and strong recommendations for applications, which should favor directness over generality.
4. Note the platforms. If `rust-toolchain.toml` lists targets or the code is heavy with `#[cfg(windows)]`-style gates for a platform other than this host, run the gate with `--target <triple>` as well (it lints and documents for that target; tests are skipped). Warnings such as dead code that appear only on the host because of platform gates are host artifacts, not findings.
5. If the code comes from an untrusted source and you are not already sandboxed, tell the user before running the gate or any harness. Both run the crate's build scripts, procedural macros, and tests with the user's permissions.
6. Run `scripts/check.sh <crate-root>` once to record the baseline. You need it to tell your own regressions from pre-existing debt. Use `--features "<list>"` instead of the default `--all-features` when the crate has mutually exclusive features.

Do not install toolchains, components, or tools (nightly, Miri, cargo-hack) and do not use the network beyond fetching crates unless the user asks. When a check needs something missing, say so and recommend it.

## Step 2: Pick the mode

**Create** when the user wants new functions, methods, types, modules, or crates. Follow "Creating code" below.

**Modify** when the user wants existing code changed: a feature, refinement, bug fix, refactor, or remediation of audit findings. Follow "Modifying code" below.

**Audit** when the user wants existing code reviewed, assessed, or checked, with issues identified and fixes proposed. Follow "Auditing code" below. Audit does not change code unless the user asks; it produces findings.

A request can span modes ("audit this module and fix what you find"). Run audit first, report, then switch to modify for the fixes.

## Non-negotiable rules

These apply in every mode. The full rule set, with IDs, lives in the reference files listed at the end.

- **Formatting is rustfmt's job.** Never hand-format against rustfmt. Never add `#[rustfmt::skip]` except for deliberate alignment (tables, matrices) or generated code.
- **Code is sound.** Safe code must not be able to cause undefined behavior. Every `unsafe` block has a `// SAFETY:` comment directly above it explaining why each precondition holds. Every `unsafe fn` and `unsafe trait` has a `# Safety` doc section. Use `unsafe` only for FFI, a benchmarked performance need, or a novel abstraction, and never to bypass `Send`, `Sync`, or lifetimes. See `references/unsafe.md`.
- **Errors versus panics.** Recoverable failures caused by input or the environment return `Result` (or `Option` when the cause is obvious). Detected programming bugs panic. Library code never panics on caller input that could reasonably be invalid. See `references/errors.md`.
- **Library errors let callers branch without parsing text.** A concrete error type with a `kind()` returning a public `#[non_exhaustive]` enum, or `is_*()` predicates. Private fields are right; a private kind with no accessor is a defect. Messages point at the offending token and position, and never copy the whole input.
- **No bare `unwrap()` in production code.** Use `?`, pattern matching, or `expect("...")` whose message states why the invariant holds. `unwrap()` is fine in tests. Doc examples use `?` with the hidden `# Ok::<(), Box<dyn std::error::Error>>(())` line.
- **Every new dependency is a cost, dev-dependencies included.** Add one only when it removes real complexity or provides correctness you cannot reasonably write (Unicode case folding is a good reason; a property-testing framework for a 100-line parser is not). Say why in your report.
- **Structure matches size.** Do not create a module for one small function and its error type; a module earns its file with a distinct concept and several items or a few hundred lines.
- **Lint overrides use `#[expect(lint, reason = "...")]`**, not `#[allow]`, so stale overrides surface.
- **Public items are documented**, the first sentence is one line of about 15 words, and functions include `# Errors`, `# Panics`, and `# Safety` sections whenever they apply. See `references/documentation.md`.
- **Public types implement `Debug`.** Types meant to be read by people also implement `Display`.
- **Comments explain why, not what.** If a comment restates the code, delete it. ripgrep's allocator comment in `crates/core/main.rs` is the model: decision, reason, and the conditions under which it applies.
- **No ASCII-art diagrams or box-drawing characters in comments or docs.** Describe structure in sentences; drawings are unreadable with a screen reader and rot when code changes. (Team convention, not upstream.)
- **No design-journal content in docs.** Docs describe current behavior, not how the code came to be or which guidelines were followed.
- **Tests verify behavior.** No tests that restate a constant or mirror the implementation's branches.

## Creating code

Work outside-in: types and signatures first, then documentation, then implementation, then tests. Writing the doc comment before the body is the fastest way to discover a confusing API.

1. **Place it.** Put the item in the module that owns its concept; in a small crate that is often `lib.rs`. New modules are named after their primary type and only created when the code warrants one (ORG-19). See `references/organization.md`.
2. **Model with types.** Newtypes for values that are easy to mix up or carry invariants, with constructors that enforce the invariant. An `enum` instead of a `bool` parameter whose meaning is not obvious at the call site. Named fields over tuples. Make invalid states unrepresentable.
3. **Shape the signature.** Borrow what you only read (`&str`, `&[T]`, `&Path`, `Option<&T>`), take ownership only when you store or consume. In libraries, accept `impl AsRef<Path>` or `impl IntoIterator` where it removes friction; in applications, keep concrete types. Builders for types with several optional settings, validating in `build()`. See `references/api-design.md`.
4. **Name it.** RFC 430 casing; `as_` / `to_` / `into_` conversions; `new`, `with_*`, `try_new` constructors; `foo()` getters. No weasel words (`Manager`, `Service`, `Factory`, `Helper`, `Util`), and no name that collides with an existing meaning in the same interface (NAME-9).
5. **Document it** per `references/documentation.md`, including a runnable example for public items, and the unit of any position or length you return.
6. **Implement it.** Single-purpose functions. A plain `for` loop over a combinator chain once the chain needs conditionals or outside state; combinators when they replace hand-written logic (`any`, `position`, `sum`). Type annotations where inference would leave a reader guessing. Human text gets Unicode-aware handling: case-insensitive matching uses Unicode case folding, or documents its gaps (API-27).
7. **Handle errors** per `references/errors.md`: in libraries a concrete error type with a public kind or predicates; in applications `anyhow` with `.context()`.
8. **Test it** per `references/testing.md`: inline unit tests, integration tests under `tests/`, and broad invariant checks that need no new dependencies (enumerate short inputs, or a seeded loop against a naive reference).
9. **Verify** with the gate below.

## Modifying code

The goal is the smallest correct change that leaves the touched code at least as clear as you found it.

1. **Read before editing.** Read the whole function, its callers, its tests, and the neighboring API. Identify local conventions (import style, error type, doc heading style such as `# Example` versus `# Examples`, line width, density, parameter and key names) and match them, even where this skill's defaults differ.
2. **Bug fixes start with a failing test that covers the bug class.** Reproduce the reported input, then add the neighboring cases the same root cause breaks (more elements, other orderings, combinations). Watch them fail, fix the root cause, watch them pass. If the project keeps regression tests in one place (ripgrep's `tests/regression.rs`), add yours there.
3. **Fix the cause, not the symptom.** A new `unwrap_or_default()`, `clone()`, or early return that silences a failure is not a fix unless you can explain why the original path was wrong. Put a one-line comment at the fix if the reason is not obvious from the code.
4. **Stay surgical.** Do not rename, reorder, or reformat code you were not asked to touch. List unrelated problems for the user instead of fixing them silently.
5. **Keep public APIs and ABIs stable** unless the user approved a breaking change. Adding a field to a public struct with public fields, a variant to an exhaustive public enum, a required trait method, or changing an FFI contract is breaking.
6. **Update docs and tests with the code.** A changed signature or behavior means changed `# Errors`, `# Panics`, examples, and tests in the same change.
7. **Formatting:** if the baseline was rustfmt-clean, run `cargo fmt`. If it was not, format only what you changed and tell the user. If `rustfmt.toml` uses nightly-only options (stable rustfmt warns that they are "only available in nightly channel"), the project's standard is `cargo +nightly fmt`; use it if installed, and otherwise format nothing automatically and say so, because stable rustfmt would rewrite code against the project's settings.
8. **Remediation from an audit:** fix findings in severity order, one finding per logical change, and name the finding in your summary.
9. **Verify** with the gate below and confirm you introduced no new warnings relative to the baseline, on every target you checked in step 1.

## Auditing code

1. Run `scripts/check.sh --audit <crate-root>` (plus `--target` per step 1). The `--audit` pass reports, with public-API reachability, undocumented `unsafe`, missing docs and `# Errors`/`# Panics` sections, `unwrap` outside tests, `#[allow]` without a reason, `dbg!`/`todo!`, glob imports, and assertions without messages. Report these hygiene results compactly: one Minor finding per category (documentation, `Debug`, SAFETY comments, lint configuration), each with a count and the few instances that matter most. A gate step that fails only because the toolchain here is newer than the one the project's CI or `rust-toolchain.toml` uses is toolchain drift: one Minor finding, not a Major.
2. Run `python3 -I scripts/audit_scan.py <crate-root>` for what clippy cannot see: glob re-exports, weasel-word type names, ASCII-art comments, design-history docs, parameter tables, bare TODOs, empty `expect` messages, and print macros in library code. Confirm each hit by reading the code; discard false positives.
3. **Probe behavior the way users meet it.** This is where most user-facing bugs in real crates are found, so spend most of the audit here and in step 5. Read the crate's docs, spec, and tests to list the input features it claims to handle. For each, build a small realistic input that the existing tests do not cover, run it through the public API from a throwaway crate outside the workspace, and compare the output with what a user should get. Cover the features the format allows but the fixtures skip:
   - inherited, indirect, or defaulted values
   - duplicates
   - optional or hidden parts
   - mild damage that other tools tolerate
   - non-ASCII text and legacy encodings
   - large but legitimate sizes

   Report each confirmed problem with the input that shows it.
4. **Harness untrusted input.** For every public function that parses untrusted bytes or text, feed it every short input over a relevant alphabet or a few million seeded random mutations, and catch panics. Also try crafted structures that make work grow faster than input size (shared or recursive references, deep nesting) under a timeout.
5. Read the code for what tools cannot judge: soundness of each `unsafe` block, error design, API shape, module boundaries, naming, test quality, and whether comments explain why. In code that talks to the operating system, also check:
   - panics on a syscall, I/O, or environment failure (`assert!` or `unwrap` on a return value, a non-UTF-8 environment variable or passwd field)
   - code between `fork` and `exec` (`pre_exec` closures) or in a signal handler that allocates, locks, formats, logs, or exits, none of which is async-signal-safe
   - leaked file descriptors or handles, and signals sent to a process ID that may have been reaped and reused
   - narrowing casts and unchecked arithmetic on sizes, counts, and coordinates

   Use `references/audit-checklist.md`.
6. Report using the format below. Propose a concrete fix for every finding.

Steps 1 and 2 are a quick floor, not the audit: spend no more than about a fifth of the time on them. An audit that lists mostly lint, documentation, and style findings has not looked at what the code does.

### Severity

- **Blocker:** a demonstrated or plausible path to undefined behavior from safe code, a panic reachable from external input in a library, data loss or corruption, a security issue, or a failing build or test suite.
- **Major:** wrong output or a rejected file on realistic input, work that grows much faster than input on legitimate files, a library panic or process exit on an operating-system or environment failure, work after `fork` or in a signal handler that is not async-signal-safe, `unsafe` that appears sound but lacks SAFETY justification, the wrong error-versus-panic choice, errors callers cannot branch on, misleading docs, public API that will force a breaking change soon, lint or rustdoc failures with the project's own toolchain, tests that do not test behavior.
- **Minor:** missing docs, missing `Debug`, incomplete `# Safety` sections where the code is sound, lint failures that appear only on a newer toolchain, naming, module placement, comment quality, readability, local convention drift.
- **Note:** optional improvements and tooling recommendations.

Calibrate honestly: missing comments on correct code are not Blockers, and a long Blocker list for a working crate erodes trust in the real ones.

### Report format

Plain text with headings, so it reads well in a screen reader and pastes cleanly into an issue. No emoji and no decorative tables. Structure:

1. A verdict of two to four sentences, including a count of findings per severity.
2. One section per severity, in order. Each finding is a heading that names the problem in a few words, followed by a location line and a short problem and fix. One problem per finding. Within a severity, put what users hit first, then soundness, then maintainability.
3. The verification commands, written as plain `cargo` commands the user can rerun without this skill, with pass, fail, or skipped and why.

```text
## Blocker

### Heap overflow when the iterator outruns its size hint
Location: src/lib.rs:1044 (insert_many).
Problem: the length is set to 0 as a panic guard, so reserve(1) sees spare capacity and never grows; later writes go past the allocation. Any .filter() iterator triggers it.
Fix: restore the length before reserving, or collect the surplus items and insert them in a second pass.
```

Leave rule IDs out of the report: a reader without this skill cannot look them up. Name the rule in words when it helps, for example "undocumented unsafe". Keep code snippets to the lines that carry the fix. Aim for the shortest report that a busy reviewer can act on; do not add a closing summary that repeats the findings.

## Verification gate

Run before declaring create or modify work done, and as the first step of an audit:

```sh
scripts/check.sh [--audit] [--target TRIPLE] [--features LIST] <crate-root>
```

It stops immediately if `cargo`, rustfmt, clippy, the project's pinned toolchain, or a requested target is missing, then runs every step and reports each:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`
4. `cargo test --workspace --all-features` (skipped for a cross `--target`)

Recommend, and run when already installed: `cargo miri test` for crates with `unsafe`; `cargo hack check --feature-powerset` for crates with several features; a build at the declared `rust-version`. Report each result honestly. A failure that predates your change is reported as pre-existing, not hidden.

## Reference files

Read the one that matches what you are doing; do not load them all.

- `references/precedence.md` - how sources rank, and the decisions on every conflict between them.
- `references/organization.md` - crates, modules, files, visibility, imports.
- `references/api-design.md` - signatures, types, traits, text handling, naming, future-proofing.
- `references/errors.md` - error types, `Result` versus `Option` versus panic, `anyhow` in applications.
- `references/documentation.md` - doc comments, canonical sections, examples, module docs.
- `references/unsafe.md` - when unsafe is allowed and how to document it.
- `references/testing.md` - unit, integration, invariant, regression, doc tests.
- `references/tooling.md` - `[lints]` baseline, rustfmt, CI gate, and the verdict on the `rust-style-guide` crate.
- `references/exemplars.md` - what each exemplar repository demonstrates, with file paths to study.
- `references/audit-checklist.md` - the audit walk-through and rule IDs.
