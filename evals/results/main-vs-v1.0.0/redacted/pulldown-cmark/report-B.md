# Audit report: pulldown-cmark 0.13.0

## Summary

The parser is robust. About 4.5 million fuzzed inputs across five option sets caused no panics, unbalanced events or invalid offsets. The SIMD scanner passed Miri and gave the same output as the scalar path on 300,000 random documents. The real defects are in behaviour users see: footnote anchors break when the reference and definition differ in letter case, links quietly turn into plain text past an internal budget, `x^2^` and `H~2~O` don't render as superscript or subscript, one emphasis pattern takes quadratic time, and the command-line tool mixes files together.

## Medium

**Footnote references that differ in case from the definition get broken anchors and wrong numbers**
src/html.rs:138 (FootnoteReference), src/html.rs:373 (FootnoteDefinition), src/parse.rs:800.
the parser matches footnote labels without regard to case (`FootnoteLabel` is a `UniCase`), so `Text[^Note].` with `[^note]: The note.` counts as a valid reference. But the event carries the raw label, and the HTML writer uses that raw text both as the key for numbering and as the id. The output is `<a href="#Note">1</a>` pointing at `id="note"`, with the definition numbered 2. The link is dead and the numbers don't match. `[^ẞ]` against `[^SS]` fails the same way.

**Link reference budget silently drops links in ordinary documents**
src/parse.rs:288 and src/parse.rs:325-366 (fetch_link_type_url_title).
each resolved reference link uses up `url.len() + title.len()` from a budget of `max(text.len(), 100_000)`. When the budget runs out, the remaining links render as literal text. Tested:
- A 31 KB document with 1,000 uses of `[the API][api]` (a 122-byte URL) produced only 820 links.
- 3,000 links resolved by a broken-link callback produced only 1,887. The callback stops being called at all once the budget is gone, so caller-supplied URLs are limited too.

None of this is documented, and nothing tells the caller it happened.

**Superscript and subscript never work inside a word**
src/firstpass.rs:2280-2295 (delim_run_can_open), src/firstpass.rs:2327-2337 (delim_run_can_close).
`^` and single `~` fall through to the rules for `_`, which forbid delimiters inside a word. So `x^2^ and H~2~O` renders unchanged with all options on, while `^super script^` works. These are the standard examples for this syntax (pandoc renders them as `<sup>`/`<sub>`), and specs/super_sub.txt has no test for this case. The comment "`_` and `~` cannot" predates the superscript and subscript extensions.

**Quadratic time on alternating `*` and `_` delimiters**
src/parse.rs:1601-1607 (InlineStack::set_lowerbound), used by find_match at src/parse.rs:1634.
when an `_` closer that can't also open fails to match, only `UNDERSCORE_NOT_BOTH` is updated. But `get_lowerbound` takes `min(mod3_lower, NOT_BOTH)`, and the underscore mod-3 slot stays at 0. So every later `_` closer rescans the whole stack of `*` openers. Repeating `*a_` takes 0.02 s at 30 KB, 0.51 s at 120 KB and 1.79 s at 240 KB: four times longer for each doubling, which puts a 1 MB input at roughly 30 s. The mirror pattern `_a*` stays linear (0.024 s at 240 KB), because the asterisk branch updates both slots.

**The command-line tool renders earlier files again with each later file**
src/main.rs:167-182.
`input` is declared once outside the file loop and never cleared. `pulldown-cmark a.md b.md` printed `<h1>File A</h1>` twice and then `<h1>File B</h1>`. Broken-link spans from `--reject-broken-links` also point into the combined buffer, not into the file they came from.

**The command-line tool prints nothing and exits 0 when stdin is not UTF-8**
src/main.rs:184 (also 172-174 and 219).
the result of `read_to_string` on stdin is thrown away with `let _ =`. Piping in `"# ok \377\n"` gave empty output and exit status 0. Write errors at line 219 are also ignored. Failures to open a file panic with "file not found" whatever the actual cause was (permissions, a directory, bad UTF-8).

**SIMD unsafe code has no SAFETY justification**
src/firstpass.rs:2693 (call site), plus the unsafe fns at 2644, 2703 and 2733.
with the `simd` feature, the crate uses raw unaligned loads and `get_unchecked`. Preconditions are stated loosely in prose ("only safe to call when…"), but no unsafe block has a `// SAFETY:` comment and no `unsafe fn` has a `# Safety` section. The code looks sound: Miri passed the SIMD unit tests and the table suite with `simd`, and the SIMD and scalar paths gave identical output on 300,000 random inputs. The justification is still missing, for example for why `offset` in `process_mask` stays below `bytes.len()` after `ContinueAndSkip`.

## Low

**Code-fence language class includes text after a tab**
src/html.rs:261.
`info.split(' ')` only splits on spaces, so the fence line ```` ```rust<TAB>ignore ```` produces `class="language-rust	ignore"`.

**Footnote ids keep whitespace, which makes invalid ids and hrefs**
src/html.rs:138-141 and 373-378.
`[^a b]` renders `href="#a b"` and `id="a b"`. HTML ids can't contain whitespace, so the link doesn't work.

**Smart-punctuation docs describe the wrong dash**
src/lib.rs:674.
the docs say `--` becomes `—`. It actually produces an en dash, `–`.

**Metadata-block docs don't say the blocks are recognised mid-document**
src/lib.rs:688-695.
after a blank line, `---\nnot: meta\n---` anywhere in a document is swallowed as metadata. This matches specs/metadata_blocks.txt, but users reading the option docs will expect a thematic break and a heading.

**Public error types lack Display and Error, and one can't be named**
src/strings.rs:12 (StringTooLongError), src/lib.rs:457 (InvalidHeadingLevel).
neither type implements `Display` or `std::error::Error`, so `?` into `Box<dyn Error>` doesn't compile. `StringTooLongError` sits in a private module and isn't re-exported, so callers of `InlineStr::try_from` can't name it.

**Documentation gaps**
crate-wide (advisory clippy pass).
the advisory pass counts, including a few hits in sibling workspace crates: 39 undocumented enum variants (for example `CodeBlockKind::Indented`, `Options::ENABLE_TABLES`), 19 undocumented fields (such as `BrokenLink::span`), 11 undocumented methods (such as `CodeBlockKind::is_fenced`) and 7 functions returning `Result` without an `# Errors` section (`write_html_io`, `write_html_fmt`). The `ENABLE_FOOTNOTES` docs say definitions are "at top level", but definitions inside block quotes and lists work.

**Lint and API hygiene**
src/lib.rs:103 and others.
`pub use crate::utils::*` is a glob re-export. There are about 50 bare `unwrap()` calls in production code; none was reachable in fuzzing, but none says why it can't fail. There are 9 asserts without messages and 3 `#[allow]`s without a reason.

pulldown-cmark/build.rs:54, 63, 117; the workspace members.
`cargo clippy -D warnings` fails only on build.rs lints that are new in clippy 1.99 (needless borrow, `expect_fun_call`). The workspace-wide doc and test steps fail because a non-default member pulls in `mozjs_sys`, which can't build in this offline container. Scoped to `-p pulldown-cmark`, every step passes.

## Low

**Bare TODOs**
src/firstpass.rs:1374, 1572 and 1732; src/parse.rs:735, 1751, 2396, 2625 and 2794; src/scanners.rs:733, 829 and 950.
Problem and fix: these TODOs have no issue reference. Link each one to an issue or delete it.

**Redundant condition and arbitrary heading attributes**
src/firstpass.rs:935; heading-attribute rendering in src/html.rs.
Problem and fix: `is_valid_seq` contains a redundant clause, `(c == b'~' && count == 2)`, which the first half already covers; simplify it. Heading attributes pass through any attribute name, including `onclick`. This is documented and no worse than raw HTML, but services that rely on HTML sanitisers should be told in the `ENABLE_HEADING_ATTRIBUTES` docs.
