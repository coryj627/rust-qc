# pulldown-cmark 0.13.0 audit report

## Summary

The CommonMark core is robust. The spec suites pass, and 1.35 million seeded random inputs over a Markdown-heavy alphabet caused no panics and no unbalanced events. Every text offset was valid; checked builds were the default build plus the `simd` build. The defects are in the extensions, the CLI binary and the docs. A wikilink with an empty label after the pipe corrupts the rest of its paragraph. Paragraph lines that start with `[^` cost time proportional to the rest of the document. Footnote references whose case differs from their definition produce dangling links. The CLI concatenates input files and silently drops non-UTF-8 stdin.

Defect counts:  other notes: 6 items.

## Medium

**A wikilink with an empty label swallows and duplicates the rest of the paragraph**
`crate/pulldown-cmark/src/parse.rs:889-905` (`handle_wikilink`).
`[[a|]]` turns everything after it in the paragraph into link text. That text then appears a second time after the link, with links nested inside links.
`scan_wikilink_pipe` returns `rest == end_ix` when nothing follows the `|`. `scan_nodes_to_ix(..., rest)` then returns the closing `]]` node itself as the body. Its `next` chain still runs to the end of the paragraph, so it becomes the link's child and also stays a sibling.
Evidence (ran, with `ENABLE_WIKILINKS`):
- Input `[[a|]] x` gives `<p><a href="a">]] x</a> x</p>`.
- Input `x [[a|]] y [[b|c]] z` gives `<p>x <a href="a">]] y <a href="b">c</a> z</a> y <a href="">c</a> z</p>`.

The wikilinks spec covers `[[|]]` but not `[[name|]]`.

**Quadratic time for paragraph lines that start with `[^` when footnotes are enabled**
`crate/pulldown-cmark/src/firstpass.rs:2158-2165` (`scan_paragraph_interrupt_no_table`).
work grows with (number of such lines) × (bytes left in the document). A list of footnote definitions or footnote references, one per line with no blank lines between them, is a realistic trigger.
for every paragraph line starting with `[^`, `std::str::from_utf8(&bytes[2..]).unwrap()` re-validates the whole rest of the document. The label scan that follows only needs a few bytes of it.
Evidence (ran, release build, `ENABLE_FOOTNOTES` only):

| Input | Size | Time |
|---|---|---|
| 40,000 lines of `[^fN]`, then their definitions | 0.94 MB | 1.80 s |
| Same input with footnotes off | 0.94 MB | 0.076 s |
| 10,000 lines of `[^fN]: note` | | 0.056 s |
| 40,000 lines of `[^fN]: note` | | 0.88 s |

Going from 10,000 to 40,000 lines makes it about 16 times slower, which is quadratic. The 19 other blowup shapes I tried (nesting, brackets, tables, math braces, link definitions) scale linearly.

**Footnote references that differ in case from their definition render a dangling link**
`crate/pulldown-cmark/src/parse.rs:768-777` (matching) and `crate/pulldown-cmark/src/html.rs:138-145` (rendering).
the parser accepts `[^Note]` as a reference to `[^note]:`. The HTML then links to `#Note`, which does not exist, and gives it a new number (2) even though only one footnote is defined.
the definition lookup uses `UniCase`, so it ignores case. But `Event::FootnoteReference` carries the reference's own spelling, and the renderer's `numbers` map and `href` use that exact string.
Evidence (ran, `ENABLE_FOOTNOTES`): input `[^note]: x` followed by `ref[^note] ref[^Note]` gives `<a href="#note">1</a>` and `<a href="#Note">2</a>`. The only definition has `id="note"`. No spec example mixes case.

**The CLI renders each later file with all earlier files prepended**
`crate/pulldown-cmark/src/main.rs:167-182`.
running `pulldown-cmark a.md b.md` prints `a.md`, then `a.md` and `b.md` concatenated.
`input` is declared once outside the loop, and `read_to_string` appends to it.
with two files containing `first` and `second`, the output is `<p>first</p>` followed by `<p>first\nsecond</p>`.

**The CLI silently prints nothing for unreadable stdin and ignores write errors**
`crate/pulldown-cmark/src/main.rs:184`, `main.rs:219`.
piping Latin-1 text, or a directory, into stdin prints nothing and exits 0. Output write failures (for example, disk full) are also discarded.
`let _ = io::stdin().lock().read_to_string(&mut input);` and `let _ = html::write_html_io(...)`.
`pulldown-cmark < latin1.md` gives empty output, `exit=0`. The same file passed as an argument correctly reports `InvalidData`.

**The `ENABLE_DEFINITION_LIST` example does not produce two terms**
`crate/pulldown-cmark/src/lib.rs:723-731`.
the documented four-line example (no blank lines) renders `title 2` as a lazy continuation of `definition 1`. The result is one `<dt>`, not two.
output is `<dt>title 1</dt><dd>definition 1\ntitle 2</dd><dd>definition 2</dd>`. Every spec example separates entries with blank lines.

**The `ENABLE_OLD_FOOTNOTES` docs describe behavior the parser does not have**
`crate/pulldown-cmark/src/lib.rs:696-714`.
the docs say that under the old syntax, two consecutive `[^1]:`/`[^2]:` lines form a single definition with two lines.
Evidence (ran, `ENABLE_OLD_FOOTNOTES`): the parser produces two separate definitions. `specs/old_footnotes.txt:145-150` expects the same, so the docs are wrong, not the parser.

**The `ENABLE_SMART_PUNCTUATION` docs give the wrong dash**
`crate/pulldown-cmark/src/lib.rs:674`.
the docs say `--` becomes `—` (em dash). The parser produces `–` (en dash), which is the correct behavior.
`a -- b --- c` gives `a – b — c`.

**`unsafe` SIMD code without SAFETY justification**
`crate/pulldown-cmark/src/firstpass.rs:2693` (the `unsafe` block), `firstpass.rs:2703` (`process_mask`, an `unsafe fn` with no `# Safety` section).
the block calls a `target_feature` function whose length precondition is not tied to the guard by any comment. `process_mask` uses `get_unchecked` with no stated precondition. The code appears sound: offsets come from mask bits within a 16-byte window inside the slice.
`cargo +nightly-2026-10-01 miri test --features simd --lib simd`, with and without `RUSTFLAGS="-C target-feature=+ssse3"`, passes. The fuzz harness built with `simd` and debug assertions also found nothing.

## Low

**Fenced code info string is split only on spaces**
`crate/pulldown-cmark/src/html.rs:261`.
a fenced block opening with ```` ```rust ```` followed by a tab and `ignore` renders `class="language-rust<TAB>ignore"`, which browsers read as two classes. The reference implementations split on any whitespace.

**Math delimiters around 255 or 256 unbalanced braces render as math**
`crate/pulldown-cmark/src/firstpass.rs:973` (`brace_context_next as u8`), `firstpass.rs:1022` (`wrapping_sub` on a `u8`).
the brace context is meant to stop `$a{b$` from becoming math. With exactly 255 `{`, or 256 `}`, it wraps back to the root value and the span becomes inline math. 254 and 256 `{` correctly do not.
Evidence (ran, `ENABLE_MATH`): input `$a` + 255×`{` + `b$` gives `math-inline`.

**The CLI panics with a misleading message on open errors**
`crate/pulldown-cmark/src/main.rs:172-174`.
any failure to open a file, including permission errors, panics with "file not found". Read errors also panic with a backtrace hint instead of a plain error and a non-zero exit code.

**Public functions return types that callers cannot name**
`crate/pulldown-cmark/src/parse.rs:1811` (`LinkDef`), `crate/pulldown-cmark/src/strings.rs:12` (`StringTooLongError`), `crate/pulldown-cmark/src/lib.rs:99-102`.
`RefDefs::get` and `RefDefs::iter` return `LinkDef`, and `InlineStr: TryFrom<&str>` uses `StringTooLongError`. Neither type is re-exported, so users cannot write either type in a signature or a struct field.

**Error types implement neither `Display` nor `Error`**
`crate/pulldown-cmark/src/lib.rs:457` (`InvalidHeadingLevel`), `crate/pulldown-cmark/src/strings.rs:12` (`StringTooLongError`).
neither error works with `?` into `Box<dyn Error>`, and neither can be shown to a user.

## Low

**Public event enums are exhaustive**
`Event`, `Tag`, `TagEnd`, `LinkType` and `BlockQuoteKind` have no `#[non_exhaustive]`, so every new extension is a semver break. That has happened in every recent release. If matching on every variant is not essential, consider `#[non_exhaustive]` at the next breaking release.

## Other notes

- **Documentation: 77 public items without docs.** Examples include `Tag::to_end`, `Event::into_static`, `CowStr::into_string`, `TextMergeStream::new`, and the `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS`, `ENABLE_SUPERSCRIPT` and `ENABLE_SUBSCRIPT` constants. Also 7 functions returning `Result` lack an Errors section (e.g. `html::write_html_io`, `html::write_html_fmt`), and 2 lack a Panics section.
- **`unwrap` outside tests: 43; asserts without messages: 8.** Almost all are tree-structure invariants. The fuzzing above reached none of them. The one that matters for users is the CLI `expect`, reported above as a Low finding.
- **Lint configuration and imports:** `pub use crate::utils::*` (`lib.rs:103`) is a glob re-export that gives `TextMergeStream` and `TextMergeWithOffset` two public paths. There are also 4 wildcard imports. The crate sets `#![warn(missing_debug_implementations)]` but has no `[lints]` baseline beyond `unexpected_cfgs`.
- **Comments:** 11 TODOs with no owner or issue reference (e.g. `firstpass.rs:1732`, `parse.rs:735`). There is a caret-drawing ASCII diagram at `firstpass.rs:1009-1021`.
- **Toolchain drift:** `cargo clippy -p pulldown-cmark --all-targets --all-features -- -D warnings` fails on Rust 1.99.1 with 4 lints in `build.rs` (`needless_borrows_for_generic_args`, `expect_fun_call` twice, a redundant `format!` reference). These lints appear to be newer than the project's MSRV of 1.71.1, which I could not test here.

## Other notes

- Whether the footnote case mismatch also breaks `id`/`href` pairing in custom renderers depends on how they key their maps. I checked only the bundled HTML writer.
