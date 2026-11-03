pulldown-cmark 0.13.0 audit report

Verdict
Two inputs panic the parser, and one of them needs no options, only a reference definition inside a list followed by a whitespace-only line. Common documents also get wrong output: uppercase HTML end tags, CRLF code spans, tabs after closing fences. Five inputs take quadratic time, the worst needing 25 s for 200 KB of plain CommonMark. The spec suite passes, because these problems sit outside what its fixtures cover.
Findings per severity: Blocker 2, Major 35, Minor 14, Note 2.

Unless a finding says "read only", I ran it through the public API from a throwaway crate at /work/tmp/probe. Timings and the debug-only panics use opt-level 2 with debug assertions and overflow checks on. The Blocker panics were also confirmed with those checks off.

## Blocker

### Panic on a reference definition followed by a whitespace-only line in a tight list
Location: src/parse.rs:2367 and 2199 (unwrap), root cause src/firstpass.rs:398-409.
Problem: after a refdef, the lazy-continuation check accepts a line of whitespace when it is indented 4 or more past the container, and parse_paragraph then builds a paragraph with no children. In a tight list or definition list that becomes an empty TightParagraph, and `self.tree.cur().unwrap()` panics. No options are needed.
Inputs that panic: "- [a]: /u\n      \n", "- [a]:b\n\t  ", "1. [a]: /u\n       \n", and with definition lists "x\n: [a]: /u\n         \n".
Outside lists the same cause gives wrong output: "[a]: /u\n    " renders <p></p>, and "[a]: /u\n    \nb" renders <p><br />\nb</p>.
Evidence: found by a seeded fuzzer, reduced automatically, and reproduced in a build with debug checks off.
Fix: after the refdef loop, return early if the line is blank instead of calling parse_paragraph.

### Panic with wikilinks and a reference that spans the wikilink's opener
Location: src/parse.rs:892 (subtract overflow), src/parse.rs:911 (slice panic without overflow checks).
Problem: "![[]j][x]]]\n\n[x]: /u" with only ENABLE_WIKILINKS panics with "byte range starts at 10 but ends at 9". A regular reference consumed the nodes, but wikilink_stack still holds the stale opener, so start_ix is past end_ix. A broken-link callback that returns a destination triggers it without the definition.
Evidence: fuzzed and reduced, then run under both profiles.
Fix: return None when start_ix > end_ix, and clear stale wikilink entries when a link consumes their nodes.

## Major

### Uppercase HTML end tags never close the block
Location: src/firstpass.rs:1250, tags at 2175.
Problem: "<PRE>\nx\n</PRE>\n\n*y*" emits everything to end of file as raw HTML, and SCRIPT and Style behave the same. Start tags match case-insensitively, but the end check is a case-sensitive `contains`. The spec (spec.txt:2379) says case-insensitive, and the bundled commonmark.js regex has the /i flag.
Fix: search for the end tag ASCII-case-insensitively.

### A closing code fence followed by a tab does not close
Location: src/scanners.rs:528.
Problem: "```\nfoo\n```\t\nbar" keeps the fence and bar inside the code block. The spec (spec.txt:1960) allows spaces or tabs after the fence.
Fix: skip tabs as well as spaces.

### CRLF inside a code span produces two spaces and defeats space stripping
Location: src/parse.rs:1340-1345.
Problem: CR and LF each push a space. "`a\r\nb`" gives "a  b", and "``\r\nfoo\r\n``" gives " foo " instead of "foo". Replacing LF with CRLF across all spec examples gave 13 differences of 1120, and every one was this.
Fix: treat CRLF as one line ending in make_code_span.

### CR-only line endings break block parsing
Location: src/scanners.rs:508 (scan_nextline searches only for LF).
Problem: code fences, indented code, HTML blocks and attribute headings treat the rest of the document as one line. "<div>\rfoo\r\rbar" never ends, and "# a {#b}\rfoo" becomes one heading with no id. 138 of 1091 spec examples changed when LF was replaced by CR, and "```x\r" repeated takes quadratic time (1 MB, 1.0 s, ratio 4).
Fix: make scan_nextline stop at LF or CR, consuming CRLF as one ending.

### A backslash-escaped bracket after a closing bracket is parsed as a reference
Location: src/parse.rs:1747-1757 (scan_reference).
Problem: it starts from the next node's start, which skips the backslash. "[a]\[b]\n\n[b]: /u" renders <a href="/u">a</a> and drops the text. "[a]\[]" with [a] defined does the same. "[a]\[b]" with [a] defined gives no link at all.
Fix: only scan a reference when the next node starts exactly at the closing bracket's end.

### Tilde and caret delimiters share one match lower bound
Location: src/parse.rs:1597, 1614, 1661.
Problem: a failed closer raises the shared bound past valid openers. "~~a~ b~~" stays literal with strikethrough on, and "^a b~ c^" stays literal with superscript and strikethrough on.
Fix: key the bound by delimiter character and, for tildes, by run length.

### With only ENABLE_SUBSCRIPT, text between double tildes loses all inline markup
Location: src/parse.rs:989-996.
Problem: the matched pair becomes one Text node over the whole range. "~~a *b* c~~" emits literal "*b*", and "~~a [x](y) c~~" loses its link.
Fix: do not match the run as a pair; leave the delimiters as text so inner markup is parsed.

### A footnote reference destroys enclosing link and image brackets
Location: src/parse.rs:804.
Problem: `self.link_stack.clear()` runs on every footnote reference. "[a [^1] b](u)" and "![a [^1] b](u)" both render as literal brackets, even though src/html.rs:510 has code for footnotes in image alt text. Nothing in the crate's specs covers this.
Fix: pop only the footnote's own bracket.

### HTML footnote numbers and anchors break on case-variant labels
Location: src/html.rs:139-143, 381-382.
Problem: the parser matches labels case-insensitively, but the writer's `numbers` map is case-sensitive and it writes labels verbatim. "x[^FOO]\n\n[^foo]: y" gives href="#FOO" numbered 1 and id="foo" numbered 2, so the link is dead and the numbers disagree. The Kelvin sign and k behave the same.
Fix: key the map by a case-folded label and emit a normalized anchor.

### Metadata block detection ignores containers
Location: src/scanners.rs:775-788, src/firstpass.rs:316, 1435.
Problem: with YAML blocks on, "> ---\n> quoted\n\nmore\n\n---\nend" renders an empty blockquote and "quoted" disappears, because the closer is an unrelated line outside the quote. Conversely "> ---\n> a: b\n> ---\n> text" is not treated as a block, because the closer has a prefix.
Fix: scan for the closer through scan_containers, or reject metadata blocks inside containers.

### Long non-ASCII reference labels are rejected and ASCII labels are unlimited
Location: src/linklabel.rs, the `b & 0b1000_0000` branch of scan_link_label_rest.
Problem: it counts non-ASCII bytes and not ASCII characters. A label of 334 CJK characters is not a reference although the limit is 999, and a 5000-character ASCII label is accepted.
Fix: count each byte that is not a UTF-8 continuation byte.

### A colon with no following space starts a definition
Location: src/scanners.rs:287-295.
Problem: with definition lists on, "foo\n:smile: bar" and "Note\n:param x: desc" become definition lists. No spec example asserts this. From memory, commonmark-hs and pandoc require whitespace after the colon; I could not run them here.
Fix: require a space, tab or end of line after the colon.

### A table delimiter row containing a tab is not recognized
Location: src/scanners.rs:673-704.
Problem: "a|b\n-|-\t\n1|2" renders as a paragraph, and "a|b\n-\t|-\n1|2" renders as a paragraph plus a list.
Fix: accept tabs in the delimiter row.

### Hitting the table cell cap corrupts the tree
Location: src/firstpass.rs:529-531.
Problem: `return None` skips the pop of the pushed row, so parse_table pops the row instead of the table. With a 1000-column header and 270 one-cell rows (about 5 KB), the rest of the document nests inside the table: "<p>after..." and the blockquote appear before </tbody></table>, and the failed row is parsed again. OffsetIter hits the assertion at src/parse.rs:2215 in debug builds and yields inverted ranges in release, so caller slicing would panic.
Fix: pop the row before returning None.

### Tab-indented nested lists get wrong source ranges
Location: src/firstpass.rs:131-133.
Problem: `container_start - outer_indent` subtracts virtual tab columns from a byte offset. "- a\n\t- b" gives the inner list 3..8, which is "\n\t- b". With definition lists, "x\n:é\n\t- b" gives 4..10, which starts inside the "é", so `&text[range]` panics in the caller.
Fix: record the byte offset before scanning the indent.

### Inline comments, processing instructions, declarations and CDATA keep container prefixes
Location: src/parse.rs:1411-1420.
Problem: only tags strip prefixes. "> foo <!-- a --\n> b -->" emits the "> " inside the InlineHtml event, and "- x <?a\n  b?>" keeps the indent.
Fix: use the same prefix handling as tags.

### Inline CDATA end detection is wrong both ways
Location: src/scanners.rs:1477-1486.
Problem: it looks only at the first "]". "a <![CDATA[x]y]]> b" is rejected and escaped, while "a <![CDATA[x]> b" is accepted as raw HTML.
Fix: search for the literal "]]>".

### Closing tags with a line ending are rejected
Location: src/scanners.rs:1322.
Problem: "a </b\n    > c" is escaped as text, while the matching opening tag form passes. The spec allows one line ending in a closing tag.
Fix: handle one line ending through the newline handler.

### Tabs count toward hard line breaks
Location: src/firstpass.rs:848-850.
Problem: "a\t\t\nb" and "a \t\nb" both give <br />. The spec (spec.txt:9208) says two or more spaces.
Fix: count only spaces.

### ATX headings mishandle tabs at the end
Location: src/firstpass.rs:1655, 1664.
Problem: "# foo\t\n" keeps the tab, and "# foo\t#\n" and "# foo #\t\n" keep the closing sequence.
Fix: treat tabs like spaces in both trims.

### Fenced code language class includes tabs
Location: src/html.rs:261.
Problem: "```rust\tfoo" gives class="language-rust\tfoo", because it splits only on a space.
Fix: split on any ASCII whitespace.

### Wikilinks spanning lines keep container prefixes
Location: src/parse.rs:889-923.
Problem: "> [[foo\n> bar]]" gives the destination "foo\n> bar" and shows "> " in the link text.
Fix: reject line endings in wikilink names, or strip prefixes.

### Wikilink pipe scan works on raw bytes
Location: src/scanners.rs:941.
Problem: the normal table form "|[[a\|b]]|" gives href="a%5C". "[[a`|`b]]" splits inside a code span and gives href="a%60".
Fix: treat an escaped pipe in tables as a pipe, and skip code span nodes.

### An empty wikilink pothole shows the closing brackets as text
Location: src/parse.rs:900-905.
Problem: "[[foo|]]" renders <a href="foo">]]</a>.
Fix: handle an empty body explicitly, either as literal text or by using the name.

### Stale link stack entries after a wikilink break the enclosing image
Location: src/parse.rs:633-645.
Problem: the outer "[" stays on link_stack and `prev` is not updated, so "![a [[b]] c](url)" is not an image.
Fix: pop the second opener and update prev when a wikilink forms.

### Wikilink source ranges omit the last bracket
Location: src/parse.rs:944.
Problem: "[[foo]] x" reports 0..6, which is "[[foo]".
Fix: use end_ix + 2.

### Old-style footnotes in a blockquote nest a spurious blockquote
Location: src/firstpass.rs:121-123.
Problem: the next line is rescanned from its raw start, so "> [^a]:\n> b" treats its ">" as a new quote.
Fix: scan containers before continuing the new line.

### Multi-line link title takes quadratic time
Location: src/parse.rs:1209.
Problem: each newline restarts scan_nodes_to_ix from the first node. "[a](b \"\n" + "c\n" x N + "\")" takes 6.2 s at 100 KB, 25 s at 200 KB and 100 s at 400 KB, with no options.
Fix: keep the cursor between newlines.

### Blank lines under deep lists take quadratic time
Location: src/parse.rs:1446, called from src/firstpass.rs:89.
Problem: every blank line walks the full spine. "- " x N + "x\n" + "\n" x N takes 15 s at 300 KB and 60 s at 600 KB, with no options.
Fix: cap nesting depth or add a blank-line fast path.

### Underscore closers rescan the delimiter stack
Location: src/parse.rs:1601-1607 with 1581-1584.
Problem: a failed non-both "_" closer updates only a bound that the min() ignores. "*a " x N + "b_ " x N takes 6.8 s at 600 KB and 26.6 s at 1.2 MB. The "*" equivalent takes 0.03 s.
Fix: update the mod-3 bound as the "*" branch does.

### Metadata scan is quadratic inside blockquotes
Location: src/scanners.rs:775-788.
Problem: "> ---\n> a\n>\n" x N scans to end of file per block, taking 5.6 s at 300 KB and 94 s at 1.2 MB with YAML blocks on.
Fix: the container-aware scan from the metadata finding above fixes this as well.

### Footnote interrupt check validates the rest of the document per line
Location: src/firstpass.rs:2161.
Problem: from_utf8 runs on all remaining bytes for each line starting "[^". "[^a]: b\n" x N takes 2.9 s at 1.6 MB, and the time quadruples per doubling, so this affects ordinary footnote-heavy documents.
Fix: pass the original &str or bound the check to the line.

### The CLI repeats earlier files when given several
Location: src/main.rs:167, 173.
Problem: `input` is never cleared, so running on a.md and b.md prints a, then a+b.
Fix: use a fresh String per file.

### The CLI hides or misreports I/O errors
Location: src/main.rs:184, 219, 172-174.
Problem: non-UTF-8 stdin gives empty output and exit 0. Writing to /dev/full exits 0. File errors end in `expect` panics.
Fix: propagate the errors with `?`.

### SIMD unsafe code lacks safety justification
Location: src/firstpass.rs:2644, 2693, 2703, 2733.
Problem: there are no SAFETY comments and process_mask has no safety section. By reading, the code looks sound: bits fall within checked 16-byte windows and SSSE3 is checked at runtime. It is read only, because the container is aarch64 and has no x86 target.
Fix: add SAFETY comments and # Safety sections.

## Minor

### Table delimiter validation is too lax
Location: src/scanners.rs:673-710.
Problem: "a|b\n-|:", "a|b\n- -|-" and "a|b\n-:-|-" all form tables. The first has a final cell with no hyphen.
Fix: validate each cell as optional colon, hyphens, optional colon.

### Definition list looseness is lost directly inside a blockquote
Location: src/firstpass.rs:280.
Problem: "> a\n>\n> : b" is tight, while the same text unquoted is loose.
Fix: set last_line_blank for this case.

### Heading attributes drop single-character names and allow empty ones
Location: src/firstpass.rs:2548, 2556.
Problem: "# a {x y}" drops both attributes, and "# a {=v}" emits <h1 ="v">.
Fix: limit the length check to # and . prefixes, and reject empty keys.

### Unquoted attribute values accept tabs
Location: src/scanners.rs:449-453.
Problem: "a <b c=d\te=f>" is rejected as HTML, though the spec excludes tabs from unquoted values.
Fix: add tab to the excluded set.

### DefinitionList range overruns a following paragraph
Location: src/firstpass.rs:1506, 2228.
Problem: "bar\n:    baz\n\nBloze\n" gives the list a range ending at 20, overlapping the paragraph at 14..20.
Fix: set the end from the last kept item.

### Reference expansion cap silently drops links and is undocumented
Location: src/parse.rs:288, 325.
Problem: a 40 KB document with one 40 KB data URI used 4 times renders the 4th as literal "[logo]". The limit appears only in private comments.
Fix: document it and consider making it configurable.

### Autofilled cell budget is per table
Location: src/firstpass.rs:449.
Problem: a 246 KB document of 60 tables yields 31.7 million events and 142 MB of HTML.
Fix: share the budget across the document.

### VT and FF count as whitespace in block syntax
Location: src/scanners.rs:425-431, 627.
Problem: "#\x0cfoo" becomes a heading, though the spec requires a space or tab. Blank-line scanning uses the same whitespace set.
Fix: use space and tab only.

### LinkDef and related types cannot be named
Location: src/lib.rs:99-101.
Problem: RefDefs::get returns LinkDef, but `pulldown_cmark::LinkDef` fails to compile (E0425). FootnoteDef and FootnoteDefs are also pub in the private module and not re-exported (read only).
Fix: re-export them.

### Error types are hard to use
Location: src/strings.rs:12, src/lib.rs:457.
Problem: StringTooLongError is not exported, and neither it nor InvalidHeadingLevel implements Display or Error. Read only.
Fix: export it and add both impls.

### Smart punctuation docs are wrong
Location: src/lib.rs:674.
Problem: the docs say "--" becomes an em dash, but the output is an en dash ("--" renders as "–").
Fix: correct the doc.

### Missing documentation
Location: public API, for example lib.rs Options::ENABLE_TABLES.
Problem: clippy with missing_docs reports 66 items. It also reports 2 functions missing # Errors and 1 missing # Panics.
Fix: add them.

### Unwraps and bare asserts in library code
Location: 28 unwraps and 4 assert-without-message warnings; parse.rs:1752 carries the comment "this unwrap is sus".
Problem: only parse.rs:2367/2199 were shown reachable (first Blocker). The rest I found safe or could not trigger.
Fix: replace them with expect messages stating the invariant.

### Glob re-export and toolchain lint drift
Location: src/lib.rs:103; clippy overall.
Problem: `pub use crate::utils::*` hides the public surface, and 11 TODOs have no owner. Clippy fails with -D warnings on 1.99 with style lints only. That is drift from the declared 1.71.1, not checked on that version.
Fix: name the re-exports and tidy the lints.

## Note

### Public event enums are exhaustive
Location: src/lib.rs (Tag, TagEnd, Event, LinkType).
Problem: each new extension variant is a breaking change.
Fix: consider #[non_exhaustive] at the next breaking release.

### Feature and build setup
Location: pulldown-cmark/Cargo.toml, build.rs.
Problem: the `gen-tests` feature makes --all-features rewrite tests/suite/*.rs in the source tree, and `getopts` is a default dependency for library users.
Fix: generate into OUT_DIR and move getopts out of the defaults.

## Verification
- `cargo fmt --all --check`: pass.
- `cargo test --locked -p pulldown-cmark`: pass (1148 spec tests plus unit and doc tests).
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-deps --all-features`: pass.
- `cargo clippy --locked -p pulldown-cmark --all-targets --all-features`: warnings only, see the drift finding.
- Workspace-wide `scripts/check.sh --audit`: clippy, rustdoc and tests fail only because dos-fuzzer's mozjs build needs llvm-objdump, which is missing here. That is environmental, not a crate defect.
- `cargo check --no-default-features`, with `--features html`, and with `--features serde`: pass.
- x86-64 `--features simd` check and SIMD execution: skipped, target unavailable on aarch64.
- `cargo miri test`: skipped. Unsafe code exists only under x86-64 with simd, and other builds forbid unsafe.
- Harnesses:
  - Exhaustive inputs up to 5 characters and exhaustive token sequences up to length 5 or 6, depending on the token set.
  - 20 million seeded mutations under catch_unwind, checking event balance and ranges.
  - Metamorphic reruns of the spec examples with CRLF, CR, blockquote, list and footnote wrapping.
  - Each option flipped on the CommonMark spec examples.
  - About 60 crafted inputs timed at increasing sizes.

Change to crate/: the gate's --all-features runs regenerated pulldown-cmark/tests/suite/*.rs through gen-tests. I made no other changes, and I could not diff the regenerated files because there is no git history. Nothing in the repository addressed me.

Findings: 53 (Blocker 2, Major 35, Minor 14, Note 2).
Last ./clock reading: 32 minutes used of 45.
