# Audit checklist

Walk these in order. The first two are mechanical; the rest need reading. Rule IDs point you to the rule in the other reference files; they stay out of the report itself.

## 1. Toolchain results

Run `scripts/check.sh --audit <crate-root>`. A failing build or test is a Blocker. A gate command that fails with the project's own toolchain (its `rust-toolchain.toml` or CI configuration) is Major; one that fails only on a newer toolchain is toolchain drift, reported as one Minor finding. Record pre-existing failures as such. Group the advisory warnings into one Minor finding per category with a count, except unsafe, unwrap, and panic warnings, which are judged one by one. Map each advisory-pass warning to its rule: `undocumented_unsafe_blocks` and `missing_safety_doc` to UNSAFE-1 and UNSAFE-2 (Major, or Blocker only if you can show or plausibly argue a path to undefined behavior), `missing_docs` to DOC-1, `missing_errors_doc` and `missing_panics_doc` to DOC-3 and PANIC-5, `unwrap_used` to PANIC-1, `allow_attributes_without_reason` to LINT, `wildcard_imports` to ORG-15, `missing_assert_message` to PANIC-4, `map_err_ignore` to ERR-5, `dbg_macro`/`todo`/`unimplemented` to PANIC-6, `missing_debug_implementations` to API-19.

## 2. Heuristic scan

Run `python3 -I scripts/audit_scan.py <crate-root>`. Confirm every hit by reading the surrounding code. Discard false positives silently; do not pad the report with them.

## 3. Behavior probes

List the input features the crate claims to handle, from its docs, its spec, and the format it reads. Check which ones the existing tests and fixtures exercise. For each feature they skip, build a small realistic input and run it through the public API from a throwaway crate outside the workspace:

- values that are inherited, indirect, or defaulted rather than written inline
- the same entry listed twice
- optional, hidden, or disabled parts that must not appear in output
- mild damage that other readers of the format tolerate
- non-ASCII text, legacy encodings, and passwords or keys outside ASCII
- legitimate inputs at the size the crate's own limits allow

Compare the output with what a user should get. A wrong result, a whole-input failure caused by one bad part, or a rejected file that other tools open is at least Major. Keep the input that shows each problem, so the fix can start from a failing test.

## 4. Panic and blowup harness for untrusted input

For each public function that parses untrusted bytes or text, write a throwaway harness outside the crate that feeds it every short input over a relevant alphabet, or a few million seeded random mutations, under `catch_unwind`. Any panic is a Blocker candidate; include the smallest input that triggers it. Also build crafted structures whose cost grows faster than their size (shared or recursive references, deep nesting) and run them under a timeout.

## 5. Soundness and safety (UNSAFE-*, PANIC-*)

- Is each `unsafe` justified under UNSAFE-4, minimal, and covered by a SAFETY comment whose claims you can verify from the code?
- Can safe code reach undefined behavior through any public function, including with panicking closures or misbehaving trait impls?
- Which panics are reachable from external input? In a library, each is a Blocker unless documented as a contract violation in `# Panics`.
- Does the crate declare `#![forbid(unsafe_code)]` if it has none?
- In code that talks to the operating system: does any syscall, I/O, or environment failure end in a panic or a process exit (`assert!` on a return value, `unwrap` on a non-UTF-8 environment variable or passwd field, `exit` from a library)?
- Does code between `fork` and `exec` (`pre_exec` closures) or in a signal handler allocate, lock, format, log, or exit? None of these is async-signal-safe.
- Are file descriptors and handles closed on every path, and can a signal reach a process ID that was already reaped and reused?
- Are sizes, counts, and coordinates converted with checked or saturating arithmetic where an overflow is reachable (window sizes in `u16`, cell counts in `usize`)?

## 6. Error design (ERR-*)

- Does each function choose `Option`, `Result`, or panic correctly?
- Are errors swallowed anywhere (`let _ =`, `.ok()`, `map_err(|_|`)?
- Library: are error types concrete, with a public kind or `is_*` predicates so callers can branch without parsing messages, and causes preserved through `source()`? Do messages avoid copying whole inputs?
- Application: is context added at each layer, and are errors turned into exit codes in one place?

## 7. Public API (API-*, NAME-*)

- Read the public surface (`cargo doc --open` structure, or every `pub` item). Is it minimal? Does any item have two public paths?
- Signatures: borrowed parameters, no container references, no opaque booleans, no smart pointers forced on callers.
- Will growth force a breaking change (exhaustive enums or public-field structs, leaked dependency types)?
- Names: RFC 430 casing, conversion prefixes, no weasel words, no project-private abbreviations.
- Standard traits implemented where expected (`Debug` always, `Display` for readable types, `FromStr`, `From`, `Default`, iterator traits on collections).

## 8. Organization (ORG-*)

- Module boundaries match concepts; leaf modules named after their primary type.
- `lib.rs`/`mod.rs` hold structure, not logic. `main.rs` is thin.
- Visibility: `pub(crate)` used for internal sharing; no glob re-exports; imports grouped.
- Oversized modules that should split.

## 9. Documentation (DOC-*, CMT-*)

- Every public item documented; first sentence about 15 words.
- `# Errors`, `# Panics`, `# Safety` present where they apply; no parameter tables.
- Examples compile, assert something, and use `?` rather than `unwrap`.
- Crate and module docs exist and describe current behavior, not design history.
- Comments explain why; no commented-out code, bare TODOs, or ASCII art.

## 10. Tests (TEST-*)

- Inline unit tests and integration tests exist for the public behavior.
- Tests assert behavior and properties, not restated constants or mirrored branches.
- Edge cases and every documented error and panic are covered.
- Feature combinations are tested; Miri runs if there is unsafe.

## 11. Project configuration

- `edition` and `rust-version` set; workspace settings centralized.
- Features additive; crate builds with `--no-default-features`.
- Lint configuration lives in `Cargo.toml`; overrides use `#[expect]` with reasons.
- If no lint table exists, recommend the baseline in `tooling.md` as a Note.

## Rule ID index

ORG-1 to ORG-19 in `organization.md`. API-1 to API-27 and NAME-1 to NAME-9 in `api-design.md`. ERR-1 to ERR-13 and PANIC-1 to PANIC-8 in `errors.md`. DOC-1 to DOC-11 and CMT-1 to CMT-6 in `documentation.md`. UNSAFE-1 to UNSAFE-9 in `unsafe.md`. TEST-1 to TEST-12 in `testing.md`. LINT findings refer to `tooling.md`.
