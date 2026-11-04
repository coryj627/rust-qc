# pulldown-cmark 0.13.0 audit report

## Verdict

The CommonMark core is robust. The spec suites pass, and 1.35 million seeded random inputs over a Markdown-heavy alphabet caused no panics and no unbalanced events. Every text offset was valid; checked builds were the default build plus the `simd` build. The defects are in the extensions, the CLI binary and the docs. A wikilink with an empty label after the pipe corrupts the rest of its paragraph. Paragraph lines that start with `[^` cost time proportional to the rest of the document. Footnote references whose case differs from their definition produce dangling links. The CLI concatenates input files and silently drops non-UTF-8 stdin.

Defect counts: 0 Blocker, 9 Major, 5 Minor, 1 Note. QA hygiene: 6 items.

## Major

### A wikilink with an empty label swallows and duplicates the rest of the paragraph
Location: `crate/pulldown-cmark/src/parse.rs:889-905` (`handle_wikilink`).
Problem: `[[a|]]` turns everything after it in the paragraph into link text. That text then appears a second time after the link, with links nested inside links.
Cause: `scan_wikilink_pipe` returns `rest == end_ix` when nothing follows the `|`. `scan_nodes_to_ix(..., rest)` then returns the closing `]]` node itself as the body. Its `next` chain still runs to the end of the paragraph, so it becomes the link's child and also stays a sibling.
Evidence (ran, with `ENABLE_WIKILINKS`):
- Input `[[a|]] x` gives `<p><a href="a">]] x</a> x</p>`.
- Input `x [[a|]] y [[b|c]] z` gives `<p>x <a href="a">]] y <a href="b">c</a> z</a> y <a href="">c</a> z</p>`.

The wikilinks spec covers `[[|]]` but not `[[name|]]`.
Fix: when `rest >= end_ix`, either fall through to the no-pipe branch with the wikiname as the display text, or return `None`. Add `[[a|]] x` as a spec example.

### Quadratic time for paragraph lines that start with `[^` when footnotes are enabled
Location: `crate/pulldown-cmark/src/firstpass.rs:2158-2165` (`scan_paragraph_interrupt_no_table`).
Problem: work grows with (number of such lines) × (bytes left in the document). A list of footnote definitions or footnote references, one per line with no blank lines between them, is a realistic trigger.
Cause: for every paragraph line starting with `[^`, `std::str::from_utf8(&bytes[2..]).unwrap()` re-validates the whole rest of the document. The label scan that follows only needs a few bytes of it.
Evidence (ran, release build, `ENABLE_FOOTNOTES` only):

| Input | Size | Time |
|---|---|---|
| 40,000 lines of `[^fN]`, then their definitions | 0.94 MB | 1.80 s |
| Same input with footnotes off | 0.94 MB | 0.076 s |
| 10,000 lines of `[^fN]: note` | | 0.056 s |
| 40,000 lines of `[^fN]: note` | | 0.88 s |

Going from 10,000 to 40,000 lines makes it about 16 times slower, which is quadratic. The 19 other blowup shapes I tried (nesting, brackets, tables, math braces, link definitions) scale linearly.
Fix: pass the `&str` the caller already has (`&self.text[ix..]`) into the interrupt check instead of bytes, so no re-validation is needed. Or validate only up to the end of the line.

### Footnote references that differ in case from their definition render a dangling link
Location: `crate/pulldown-cmark/src/parse.rs:768-777` (matching) and `crate/pulldown-cmark/src/html.rs:138-145` (rendering).
Problem: the parser accepts `[^Note]` as a reference to `[^note]:`. The HTML then links to `#Note`, which does not exist, and gives it a new number (2) even though only one footnote is defined.
Cause: the definition lookup uses `UniCase`, so it ignores case. But `Event::FootnoteReference` carries the reference's own spelling, and the renderer's `numbers` map and `href` use that exact string.
Evidence (ran, `ENABLE_FOOTNOTES`): input `[^note]: x` followed by `ref[^note] ref[^Note]` gives `<a href="#note">1</a>` and `<a href="#Note">2</a>`. The only definition has `id="note"`. No spec example mixes case.
Fix: emit the definition's label in `FootnoteReference` when a definition matched, or case-fold the keys in the HTML writer, consistently for `id`, `href` and numbering.

### The CLI renders each later file with all earlier files prepended
Location: `crate/pulldown-cmark/src/main.rs:167-182`.
Problem: running `pulldown-cmark a.md b.md` prints `a.md`, then `a.md` and `b.md` concatenated.
Cause: `input` is declared once outside the loop, and `read_to_string` appends to it.
Evidence (ran): with two files containing `first` and `second`, the output is `<p>first</p>` followed by `<p>first\nsecond</p>`.
Fix: call `input.clear()` at the top of each loop iteration.

### The CLI silently prints nothing for unreadable stdin and ignores write errors
Location: `crate/pulldown-cmark/src/main.rs:184`, `main.rs:219`.
Problem: piping Latin-1 text, or a directory, into stdin prints nothing and exits 0. Output write failures (for example, disk full) are also discarded.
Cause: `let _ = io::stdin().lock().read_to_string(&mut input);` and `let _ = html::write_html_io(...)`.
Evidence (ran): `pulldown-cmark < latin1.md` gives empty output, `exit=0`. The same file passed as an argument correctly reports `InvalidData`.
Fix: propagate both errors with `?`, since `main` already returns `io::Result<()>`.

### The `ENABLE_DEFINITION_LIST` example does not produce two terms
Location: `crate/pulldown-cmark/src/lib.rs:723-731`.
Problem: the documented four-line example (no blank lines) renders `title 2` as a lazy continuation of `definition 1`. The result is one `<dt>`, not two.
Evidence (ran): output is `<dt>title 1</dt><dd>definition 1\ntitle 2</dd><dd>definition 2</dd>`. Every spec example separates entries with blank lines.
Fix: add blank lines to the example, or document that a new term needs a blank line before it.

### The `ENABLE_OLD_FOOTNOTES` docs describe behavior the parser does not have
Location: `crate/pulldown-cmark/src/lib.rs:696-714`.
Problem: the docs say that under the old syntax, two consecutive `[^1]:`/`[^2]:` lines form a single definition with two lines.
Evidence (ran, `ENABLE_OLD_FOOTNOTES`): the parser produces two separate definitions. `specs/old_footnotes.txt:145-150` expects the same, so the docs are wrong, not the parser.
Fix: remove that sentence from the example. The other two contrasts in the docs (indented code block, dangling link) do match.

### The `ENABLE_SMART_PUNCTUATION` docs give the wrong dash
Location: `crate/pulldown-cmark/src/lib.rs:674`.
Problem: the docs say `--` becomes `—` (em dash). The parser produces `–` (en dash), which is the correct behavior.
Evidence (ran): `a -- b --- c` gives `a – b — c`.
Fix: change the docs to "`--` with `–`".

### `unsafe` SIMD code without SAFETY justification
Location: `crate/pulldown-cmark/src/firstpass.rs:2693` (the `unsafe` block), `firstpass.rs:2703` (`process_mask`, an `unsafe fn` with no `# Safety` section).
Problem: the block calls a `target_feature` function whose length precondition is not tied to the guard by any comment. `process_mask` uses `get_unchecked` with no stated precondition. The code appears sound: offsets come from mask bits within a 16-byte window inside the slice.
Evidence (ran): `cargo +nightly-2026-10-01 miri test --features simd --lib simd`, with and without `RUSTFLAGS="-C target-feature=+ssse3"`, passes. The fuzz harness built with `simd` and debug assertions also found nothing.
Fix: add `// SAFETY:` above line 2693, citing the `is_x86_feature_detected!("ssse3")` check and the `bytes.len() >= VECTOR_SIZE` check. Add `# Safety` sections to `process_mask`, `compute_mask` and `simd_iterate_special_bytes` stating the bound each relies on.

## Minor

### Fenced code info string is split only on spaces
Location: `crate/pulldown-cmark/src/html.rs:261`.
Problem: a fenced block opening with ```` ```rust ```` followed by a tab and `ignore` renders `class="language-rust<TAB>ignore"`, which browsers read as two classes. The reference implementations split on any whitespace.
Fix: use `info.split(|c: char| c.is_ascii_whitespace()).next()`.

### Math delimiters around 255 or 256 unbalanced braces render as math
Location: `crate/pulldown-cmark/src/firstpass.rs:973` (`brace_context_next as u8`), `firstpass.rs:1022` (`wrapping_sub` on a `u8`).
Problem: the brace context is meant to stop `$a{b$` from becoming math. With exactly 255 `{`, or 256 `}`, it wraps back to the root value and the span becomes inline math. 254 and 256 `{` correctly do not.
Evidence (ran, `ENABLE_MATH`): input `$a` + 255×`{` + `b$` gives `math-inline`.
Fix: saturate instead of truncating, or keep a separate "overflowed" context that never matches.

### The CLI panics with a misleading message on open errors
Location: `crate/pulldown-cmark/src/main.rs:172-174`.
Problem: any failure to open a file, including permission errors, panics with "file not found". Read errors also panic with a backtrace hint instead of a plain error and a non-zero exit code.
Fix: return the error with the path as context, through `main`'s `io::Result`.

### Public functions return types that callers cannot name
Location: `crate/pulldown-cmark/src/parse.rs:1811` (`LinkDef`), `crate/pulldown-cmark/src/strings.rs:12` (`StringTooLongError`), `crate/pulldown-cmark/src/lib.rs:99-102`.
Problem: `RefDefs::get` and `RefDefs::iter` return `LinkDef`, and `InlineStr: TryFrom<&str>` uses `StringTooLongError`. Neither type is re-exported, so users cannot write either type in a signature or a struct field.
Fix: re-export both from the crate root.

### Error types implement neither `Display` nor `Error`
Location: `crate/pulldown-cmark/src/lib.rs:457` (`InvalidHeadingLevel`), `crate/pulldown-cmark/src/strings.rs:12` (`StringTooLongError`).
Problem: neither error works with `?` into `Box<dyn Error>`, and neither can be shown to a user.
Fix: implement `Display` and `std::error::Error` for both.

## Note

### Public event enums are exhaustive
`Event`, `Tag`, `TagEnd`, `LinkType` and `BlockQuoteKind` have no `#[non_exhaustive]`, so every new extension is a semver break. That has happened in every recent release. If matching on every variant is not essential, consider `#[non_exhaustive]` at the next breaking release.

## QA hygiene

These come from the audit pass of `scripts/check.sh` (run per package, see below) and `audit_scan.py`.

- **Documentation: 77 public items without docs.** Examples include `Tag::to_end`, `Event::into_static`, `CowStr::into_string`, `TextMergeStream::new`, and the `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS`, `ENABLE_SUPERSCRIPT` and `ENABLE_SUBSCRIPT` constants. Also 7 functions returning `Result` lack an Errors section (e.g. `html::write_html_io`, `html::write_html_fmt`), and 2 lack a Panics section.
- **`unwrap` outside tests: 43; asserts without messages: 8.** Almost all are tree-structure invariants. The fuzzing above reached none of them. The one that matters for users is the CLI `expect`, reported above as a Minor finding.
- **Lint configuration and imports:** `pub use crate::utils::*` (`lib.rs:103`) is a glob re-export that gives `TextMergeStream` and `TextMergeWithOffset` two public paths. There are also 4 wildcard imports. The crate sets `#![warn(missing_debug_implementations)]` but has no `[lints]` baseline beyond `unexpected_cfgs`.
- **Comments:** 11 TODOs with no owner or issue reference (e.g. `firstpass.rs:1732`, `parse.rs:735`). There is a caret-drawing ASCII diagram at `firstpass.rs:1009-1021`.
- **Toolchain drift:** `cargo clippy -p pulldown-cmark --all-targets --all-features -- -D warnings` fails on Rust 1.99.1 with 4 lints in `build.rs` (`needless_borrows_for_generic_args`, `expect_fun_call` twice, a redundant `format!` reference). These lints appear to be newer than the project's MSRV of 1.71.1, which I could not test here.
- **Workspace gate:** the workspace-wide gate cannot build here because the `dos-fuzzer` member's `mozjs` build needs `llvm-objdump`. That member is outside the audited code, so I ran every step per package instead.

## Unverified

- Whether the footnote case mismatch also breaks `id`/`href` pairing in custom renderers depends on how they key their maps. I checked only the bundled HTML writer.

## Verification commands

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail. The `mozjs` build in `dos-fuzzer` fails (environment, not the audited crate).
- `cargo clippy --locked -p pulldown-cmark --all-targets --all-features -- -D warnings`: fail, 4 newer-toolchain lints in `build.rs` (toolchain drift).
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-deps --document-private-items --all-features`: pass.
- `cargo test --locked -p pulldown-cmark`: pass (59 unit tests, 1148 spec-suite tests, 23 + 20 + 5 others).
- `cargo test --locked -p pulldown-cmark --features simd --lib`: pass (65).
- `cargo +nightly-2026-10-01 miri test --locked -p pulldown-cmark --features simd --lib simd`, with and without `RUSTFLAGS="-C target-feature=+ssse3"`: pass (6 each).
- Audit lints: `cargo clippy --locked -p pulldown-cmark --lib --bins --all-features -- -W missing_docs -W clippy::unwrap_used ...`: advisory, counted above.
- `python3 -I skill/scripts/audit_scan.py crate/pulldown-cmark`: 1 glob re-export and 11 TODOs, both confirmed.
- Throwaway harness `tmp/h` (path dependency, release build with debug assertions and overflow checks): 1.35 million seeded random inputs under all options, GFM-only and no options, plus the `simd` build. It checks for panics, offset bounds and char boundaries, broken-link span validity, event balance and text/source agreement, and found nothing. Crafted inputs come from `tmp/gen*.py`. Blowup shapes were timed at 10,000 and 40,000 repetitions.
- MSRV build at 1.71.1: skipped, toolchain not installed.

## Closing summary

I ran the gate per package, then spent most of the time on behavior. That meant random-input fuzzing with offset and balance checks, about 70 crafted growth-rate inputs, targeted probes of each extension and of the documented examples, Miri on the SIMD path, and running the CLI. I read `html.rs`, `strings.rs`, `utils.rs`, `main.rs`, the public API in `lib.rs`, and the SIMD, wikilink and footnote code. I did not read the bulk of `firstpass.rs`, `parse.rs`, `scanners.rs` and `linklabel.rs` line by line. I relied on the spec suites and fuzzing for those, so semantic bugs in rarely combined extensions could remain. I did not test the MSRV build. I stopped at 13 of 45 minutes because the fuzzing and blowup probes had stopped producing new results, and every finding was confirmed with a reproducing input and a traced cause.

Last clock reading: 13 minutes used of 45.