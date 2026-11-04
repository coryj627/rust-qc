# Audit report: pulldown-cmark 0.13.0 (`crate/pulldown-cmark/src/`)

## Summary

The parser is mostly robust. It ran in linear time on 41 pathological input shapes up to 600 KB, and about 1.6 million random inputs found only one panic. That panic is real: a few bytes of plain CommonMark, with no extensions enabled, crash the parser. The same root cause also produces wrong HTML at top level. Beyond that, the HTML renderer breaks footnote links when the reference and definition labels differ in case, and the public enums cannot grow without breaking changes.

Nothing in the repository or its output addressed the auditor.

## High

**Panic on a whitespace line with a tab after a link reference definition**
src/firstpass.rs:424 (`scan_next_line_or_lazy_continuation`); the panic is at src/parse.rs:2367 and src/parse.rs:2199 (`self.tree.cur().unwrap()`).

after a link reference definition, the next line is checked for a paragraph interrupt only when its indent is under 4 columns. A blank line indented with a tab skips that check. `parse_paragraph` then starts at the end of the line and leaves a paragraph node with no children. Inside a tight list item, `Parser::next` and `OffsetIter::next` unwrap the missing child and panic. That happens in `Parser::new`, `push_html` and `into_offset_iter` alike.

Smallest inputs found:
- `">-\t[a]:x\n\t"`, default options.
- `"> -\t[a]: /url\n\t\n"`, default options.
- `"x\n:[a]:b\n     "`, with `ENABLE_DEFINITION_LIST` only.

The same cause gives wrong output outside lists. `"[a]: /url\n    \nnext"` renders `<p><br />\nnext</p>` instead of `<p>next</p>`. Its `HardBreak` event has range 10..15, which starts before its paragraph's 14..19.

```rust
line_start.scan_all_space();
if line_start.is_at_eol() { return None; }
Some(line_start)
```
Also replace the two `unwrap()` calls with a skip of empty tight paragraphs, so a future tree bug cannot crash callers. Add the three inputs above as regression tests.

## Medium

**Footnote links point nowhere when label case differs**
src/html.rs:138-145 and src/html.rs:373-384 (`numbers: HashMap<CowStr, usize>`).

the parser matches footnote labels without regard to case, but the renderer numbers footnotes and writes `href` and `id` from the exact text. `See[^Note].\n\n[^note]: Defined.` gives `<a href="#Note">1</a>`, while the definition gets `id="note"` and label 2. The link is dead and the numbers disagree.

**Public event enums are exhaustive, so every new extension is a breaking change**
src/lib.rs:154 (`Tag`), :384 (`TagEnd`), :478 (`LinkType`), :524 (`Event`), :638 (`Alignment`), and also `CodeBlockKind`, `BlockQuoteKind` and `MetadataBlockKind`.

0.13 added `Superscript`, `Subscript` and the definition-list variants, and each one broke every downstream exhaustive `match`. The crate is still adding features at that pace.

**Unsafe SIMD code lacks SAFETY comments and a Safety section**
src/firstpass.rs:2693, :2715 (`get_unchecked`) and :2733 (`simd_iterate_special_bytes`).

the code appears sound. Mask bits never exceed the 16-byte window, `iterate_special_bytes` checks `len >= 16` and ssse3 before calling in, and Miri passed the SIMD unit tests. However, no block has a `// SAFETY:` comment, and the precondition sits in an "Important:" doc line rather than a `# Safety` section.

## Low

**Lint failures on the current toolchain (toolchain drift)**
crate-wide.

`cargo clippy -p pulldown-cmark --all-targets -D warnings` fails on 1.99.1 with 26 errors from newer lints: `map_or` simplification, byte-string literals, doc list indentation, complex types and others. Rustdoc and the tests are clean.

**Missing documentation**
src/lib.rs, src/utils.rs, src/strings.rs.

the advisory pass reports:
- 39 undocumented enum variants and 19 undocumented fields.
- 14 undocumented methods and associated functions, for example `CodeBlockKind::is_indented`, `Tag::to_end`, `Event::into_static`, `TextMergeStream::new` and `CowStr::into_string`.
- 3 undocumented structs.
- 7 functions without an `# Errors` section, such as the `write_html_*` functions that return `io::Result`.
- 2 functions without a `# Panics` section.
- The `TagEnd::List(bool)` boolean is documented only in a comment.

**Error types lack Display and Error**
src/lib.rs:457 (`InvalidHeadingLevel`), src/strings.rs:12 (`StringTooLongError`).

neither type implements `Display` or `std::error::Error`, so neither works with `?` into `Box<dyn Error>`. `InvalidHeadingLevel` also hides the rejected value behind a private field with no accessor.

**Bare unwrap and assert in production code**
43 `unwrap` calls outside tests, for example src/strings.rs:70 and src/tree.rs:111 and :197. There are also 8 asserts without messages.

the High above is one of these unwraps firing. The rest guard invariants without saying why they hold.

**Glob re-export of the utils module**
src/lib.rs:103 (`pub use crate::utils::*`).

this gives `TextMergeStream` and `TextMergeWithOffset` two public paths, and it would silently export anything later made public in `utils`.

## Low

**Raw HTML and `javascript:` URLs pass through**
`[b](javascript:alert(1))` and inline HTML reach the output unchanged. That is CommonMark behavior, but the `html` module docs should warn that untrusted input needs a sanitizer.

**Duplicated special-byte tables**
The SIMD lookup table (firstpass.rs:2600) and the scalar one (firstpass.rs:2353) list the same bytes separately. They agree today. Building one from a shared list would keep them from drifting.

**TODOs without an issue reference**
There are 11 TODO/FIXME comments without an issue reference, for example scanners.rs:733, parse.rs:735 and lib.rs:206. Link each to an issue or remove it.
