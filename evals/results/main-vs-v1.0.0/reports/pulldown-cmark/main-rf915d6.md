# Audit report: pulldown-cmark 0.13.0 (`crate/pulldown-cmark/src/`)

## Verdict

The parser held up under stress. About 4 million generated inputs across all option sets produced no panics and no out-of-bounds offsets. Pathological inputs took time roughly in proportion to their size. The SIMD scanner gave the same results as the scalar one on 300,000 inputs. The defects are wrong output on realistic inputs, a command-line tool that loses data or reports success when it fails, and API and documentation gaps.

Counts: 0 Blocker, 9 Major, 6 Minor, 0 Note. There are also 8 QA hygiene items, counted separately.

## Blocker

None found.

## Major

### Footnote labels that differ only in case produce broken links and the wrong numbers
Location: src/html.rs:138-145 and 373-385, src/parse.rs:767-777.
Problem: the parser matches footnote labels without regard to case. The HTML writer then treats the two spellings as different footnotes. The reference links to `#A` but the definition has `id="a"`. The reference is numbered 1 and the definition 2.
Cause: the parser checks for a definition with a case-insensitive lookup (`footdefs` is keyed by `UniCase`, parse.rs:2001). But it puts the label exactly as written into `Event::FootnoteReference`. `HtmlWriter.numbers` is a plain `HashMap<CowStr, usize>` keyed on the exact string.
Evidence (ran): `x[^A]\n\n[^a]: one\n` with ENABLE_FOOTNOTES renders `<a href="#A">1</a>` and `<div class="footnote-definition" id="a"><sup ...>2</sup>`. Three spellings (`[^Note]`, `[^note]`, `[^NOTE]`) give numbers 1, 2 and 3 for a single footnote. ENABLE_OLD_FOOTNOTES behaves the same way.
Fix: have the reference event carry the label as the definition spells it, or have the writer key `numbers` and write the anchors from a case-folded label. Add the cases above as tests.

### U+0000 is not replaced with U+FFFD
Location: the whole text path (no replacement exists anywhere in src/). The visible side effect is at src/scanners.rs:1377.
Problem: the CommonMark spec bundled in the repo (third_party/CommonMark/spec.txt:481) says U+0000 "must be replaced" with U+FFFD, for security reasons. pulldown-cmark passes NUL through unchanged in text, code, headings and raw HTML. A NUL also stops a link destination from being recognised, so the link turns into literal text.
Cause: nothing in the first pass or the inline pass maps `\0`. The link destination scanner rejects any byte in `\0..=' '`.
Evidence (ran): `a\0b` renders `<p>a\0b</p>`. `` `a\0b` `` renders `<code>a\0b</code>`. `[x](/a\0b)` renders the literal text `[x](/a\0b)`.
Fix: replace NUL with U+FFFD when text events are built and in the link-destination and HTML scanners. Add spec-style tests, because the generated suite does not cover this.

### The command-line tool renders earlier files again when given several files
Location: src/main.rs:167-182.
Problem: `pulldown-cmark a.md b.md` prints a.md, then a.md and b.md together as one document. Broken-link spans then point into the combined text.
Cause: one `input` buffer is shared across the loop, and `read_to_string` appends to it without clearing it first.
Evidence (ran): with files `first` and `second`, the output is `<p>first</p>` followed by `<p>first\nsecond</p>`.
Fix: call `input.clear()` at the top of each loop iteration, or declare the buffer inside the loop.

### The command-line tool exits 0 when reading stdin or writing output fails
Location: src/main.rs:184 and 219.
Problem: invalid UTF-8 on stdin produces empty output with exit status 0. Writing to a full disk or a closed pipe also exits 0.
Cause: both results are discarded with `let _ = ...`. Errors from the final flush of the `BufWriter` are also lost, because the writer is dropped without an explicit flush.
Evidence (ran): `pulldown-cmark < bad.md`, where the file contains `\xff\xfe`, prints nothing and exits 0. `echo hi | pulldown-cmark > /dev/full` exits 0.
Fix: propagate both results with `?`, since `main` already returns `io::Result`. Call `flush()` on the writer before returning.

### A wikilink with an empty display text shows the closing brackets as link text
Location: src/parse.rs:894-908 (handle_wikilink).
Problem: `[[Page|]]` renders `<a href="Page">]]</a>`. The offset iterator reports two `Text("]")` events inside the link.
Cause: when the pipe is the last character, `rest` equals the start of the closing bracket. `scan_nodes_to_ix(..., rest)` then returns the closing `]` node, which becomes the link's child.
Evidence (ran): the output above. The spec file covers `[[|]]` but not `[[Page|]]`.
Fix: when `rest == end_ix`, either treat the link as having no pipe (show the page name) or reject it, as `[[|x]]` is rejected. Add the case to specs/wikilinks.txt.

### Smart punctuation turns an unmatched closing double quote into an opening quote
Location: src/parse.rs:1089-1098.
Problem: `a 6" board`, `He said hi" to me.` and `word"` all render with a left quote (“) where a right quote (”) belongs. Inch marks and quotes that span paragraphs are common in prose.
Cause: the default branch always produces `“`, even when the delimiter can close and cannot open. The single-quote branch handles this case correctly, defaulting to `’`.
Evidence (ran): `He said hi" to me.` with ENABLE_SMART_PUNCTUATION gives `<p>He said hi“ to me.</p>`.
Fix: in the fallback, emit `”` when `can_close && !can_open`, and `“` otherwise.

### SIMD `unsafe` code has no SAFETY justification
Location: src/firstpass.rs:2693 (the call into `simd_iterate_special_bytes`), 2715 (`get_unchecked`), and 2644-2676 (unaligned loads). This code is only compiled with the `simd` feature.
Problem: I believe the code is sound. Miri passes the module's six SIMD tests, and SIMD output matched scalar output on 300,000 random inputs. But no `unsafe` block or `unsafe fn` says why its preconditions hold. The `get_unchecked` relies on an unstated invariant: every set bit in `mask` corresponds to an in-bounds byte, and `offset` tracks the bit positions exactly.
Evidence (read): the audit lint pass reports "unsafe block missing a safety comment" at firstpass.rs:2693. None of the `unsafe fn`s has a `# Safety` section.
Fix: add `// SAFETY:` comments covering SSSE3 detection, `bytes.len() >= ix + 16`, and the mask-to-index invariant. Add `# Safety` sections. Consider `bytes[offset]` in `process_mask`, because the bounds check there is not on a hot path.

### Public enums are exhaustive, so each new extension is a breaking release
Location: src/lib.rs:154 (Tag), 384 (TagEnd), 478 (LinkType), 524 (Event), 135 (BlockQuoteKind).
Problem: each release in this series has added variants (for example, WikiLink, Superscript and Subscript in 0.13). With exhaustive enums, every downstream `match` breaks each time, which forces a semver-major release for every new syntax extension.
Fix: mark these enums `#[non_exhaustive]` in the next breaking release.

### Public error types do not implement `Error` or `Display`, and one cannot be named
Location: src/strings.rs:12 (StringTooLongError), src/lib.rs:457 (InvalidHeadingLevel), src/lib.rs:103.
Problem: `InlineStr::try_from(&str)` returns `StringTooLongError`. That type is in a private module and is not re-exported, so callers cannot name it in signatures. Neither error type implements `std::error::Error` or `Display`, so neither works with `?` into `Box<dyn Error>`. `InvalidHeadingLevel` also hides the rejected value, with no accessor.
Fix: re-export `StringTooLongError`. Implement `Display` and `Error` for both types, and add a getter for the rejected level.

## Minor

### Code-block language class keeps everything after a tab
Location: src/html.rs:261.
Problem: ```` ```rust\tlinenos ```` renders `class="language-rust\tlinenos"`. cmark and commonmark.js both split the info string on any whitespace.
Cause: `info.split(' ')` splits on spaces only.
Fix: use `info.split(|c: char| c.is_ascii_whitespace())`.

### Duplicate footnote definitions produce duplicate `id` attributes
Location: src/html.rs:373-385.
Problem: `x[^a]\n\n[^a]: one\n\n[^a]: two\n` emits two `<div ... id="a">` elements, both labelled 1. That is invalid HTML, and GitHub ignores the second definition.
Fix: skip a definition, or give it a unique id, when its label has already been rendered. Alternatively, report duplicates in the parser.

### A `class=` heading attribute produces a second `class` attribute
Location: src/html.rs:186-206.
Problem: `# H {class=x .y}` renders `<h1 class="y" class="x">`. Browsers keep only the first, so `x` is lost. The same happens with `#a id=b`.
Fix: merge `class=` values into the class list, and let `id=` override or be ignored, when rendering.

### The smart punctuation docs say `--` becomes an em dash
Location: src/lib.rs:675.
Problem: the docs say "replacing `--` with `—`". The code produces an en dash: `a -- b` gives `a – b`. The behaviour is right and the documentation is wrong.
Fix: change the text to "`--` with `–`".

### The command-line tool panics with a misleading message on file errors
Location: src/main.rs:172-174.
Problem: any open failure, including permission denied, panics with "file not found". A directory or a non-UTF-8 file panics with a backtrace hint.
Evidence (ran): `pulldown-cmark /work/tmp` panics at main.rs:174 with `IsADirectory`.
Fix: return the error with the file name added, using `map_err` and `?`, since `main` already returns `io::Result`.

### The metadata block options do not say that blocks are recognised mid-document
Location: src/lib.rs:688-695.
Problem: with ENABLE_YAML_STYLE_METADATA_BLOCKS, any block of the form `---` / text / `---` after a blank line is hidden. specs/metadata_blocks.txt shows this is intended. The option docs mention only the delimiters, so users who expect front matter only at the top can lose content without any warning.
Fix: document where blocks are recognised.

## QA hygiene

### Missing documentation: about 66 public items
These are 39 enum variants, 12 struct fields, 11 methods and associated functions (for example `CodeBlockKind::is_indented`, `TextMergeStream::new`), and `HeadingLevel`, `Options` constants and `DefaultBrokenLinkCallback`. `write_html_io` and `write_html_fmt` have no `# Errors` section, and `push_html` has no `# Panics` section.
Fix: document these items, starting with `Tag` and `Event` variants, which users match on.

### Doc examples ignore the `Result` they produce: 2
`html::write_html_io(...)` (html.rs:581) and `html::write_html_fmt(...)` (html.rs:616) discard the result.
Fix: use `?` with the hidden `Ok` line.

### `unwrap()` outside tests: 28
None of them was reachable in about 4 million fuzz cases. Several are structurally safe, for example `split(' ').next()` and `from_utf8` on `InlineStr`.
Fix: use `expect` with a message stating the invariant.

### Assertions without messages: 6
For example firstpass.rs:2645 and parse.rs:2191.

### Glob imports and re-exports: 4
The re-export at lib.rs:103 (`pub use crate::utils::*`) leaves the public surface implicit. The wildcard imports are at firstpass.rs:18 and 2590, and parse.rs:33.

### Clippy is not clean on a current toolchain: 23 errors
CI (`.github/workflows/rust.yml`) does not run clippy, so this is toolchain drift and not a project-gate failure. Examples are `map_or` simplification and `type_complexity` in scanners.rs. There are 4 more errors in build.rs.

### TODO and FIXME without an owner or issue: 11
For example parse.rs:735 and scanners.rs:950.

### No lint table beyond `unexpected_cfgs`
Consider a `[lints]` baseline, and `#[expect]` with reasons for the overrides.

## Unverified

These are not counted above.

- Heading attributes accept any attribute name, so `# H {onclick=x}` emits an event-handler attribute. Raw HTML is passed through by design, so this matters only to users who sanitise by filtering `Event::Html` and `Event::InlineHtml`. I could not find a stated policy on this.
- Tables pad short rows to the header width. A 17 KB input with 500 header columns and short rows produced 2.4 MB of HTML. GFM specifies the padding, so I list this only as a possible amplification concern.
- A single `~` or `^` does not work inside a word: `H~2~O` and `x^2^` are not converted. This looks like a deliberate choice that mirrors the `_` rules, but nothing documents it.

## Verification

- `cargo fmt --all --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: fail. The lint errors are in pulldown-cmark's build.rs and src. The fuzz member's `mozjs_sys` dependency also cannot build offline without llvm-objdump; that failure has nothing to do with this crate.
- `cargo clippy --locked -p pulldown-cmark --all-targets -- -D warnings`: fail, 23 lint errors (see hygiene).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items --all-features`: fail, only because of the mozjs build. `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-deps`: pass.
- `cargo test --workspace --all-features`: fail because of mozjs. `cargo test --locked -p pulldown-cmark`: pass (59 + 23 + 20 + 1148 + 5 tests).
- `cargo +nightly-2026-10-01 miri test --locked -p pulldown-cmark --no-default-features --features simd,html --lib simd`: pass, 6 tests. The full suite was not run under Miri because of time.
- Throwaway harnesses in tmp/fz, run with `cargo run --release --bin <name>`:
  - 400,000 random token-level inputs with random options: no panics, all offsets in bounds.
  - Exhaustive search of every input up to 5 characters over a 16-symbol alphabet, with 3 option sets (3.36 million cases): no panics.
  - 300,000 character-level inputs, built with and without `--features pulldown-cmark/simd`: identical HTML and offset hashes.
  - 36 pathological shapes at 5,000, 20,000 and 40,000 repetitions: time roughly proportional to input size.
- Behaviour probes in tmp/pr: `cargo run --release --bin pr`, `--bin p2` and `--bin p3`.

I did not modify `crate/`. No text in the repository or in tool output addressed me.

## Closing summary

I covered the full gate (rerun on the package alone, because the workspace build is broken by the fuzz crate's mozjs dependency), the heuristic scan, three fuzz and timing harnesses, a SIMD-against-scalar comparison, Miri on the SIMD tests, about 120 behaviour probes across every extension option, and a read of html.rs, strings.rs, utils.rs, main.rs, the SIMD module and the link, wikilink and smart-quote code in parse.rs. I did not check the serde feature, the full test suite under Miri, the rest of the block-level code in firstpass.rs (lists, tables, HTML blocks) beyond fuzzing, or the GFM spec examples that the generated suite skips. I stopped once fuzzing kept finding nothing and the probes stopped turning up new defect classes. The clock showed little time used, but that clock seems to leave out most waiting time.

Last clock reading: 5 minutes used of 45.