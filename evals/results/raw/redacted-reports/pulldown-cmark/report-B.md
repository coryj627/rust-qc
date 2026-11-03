pulldown-cmark 0.13.0 audit report

Verdict
Two inputs panic the parser, and one of them needs no options, only a reference definition inside a list followed by a whitespace-only line. Common documents also get wrong output: uppercase HTML end tags, CRLF code spans, tabs after closing fences. Five inputs take quadratic time, the worst needing 25 s for 200 KB of plain CommonMark. The spec suite passes, because these problems sit outside what its fixtures cover.
Findings per severity: High 2, Medium 35, Low 14, Note 2.

Unless a finding says "read only", I ran it through the public API from a throwaway crate at /work/tmp/probe. Timings and the debug-only panics use opt-level 2 with debug assertions and overflow checks on. The High panics were also confirmed with those checks off.

## High

**Panic on a reference definition followed by a whitespace-only line in a tight list**
src/parse.rs:2367 and 2199 (unwrap), root cause src/firstpass.rs:398-409.
after a refdef, the lazy-continuation check accepts a line of whitespace when it is indented 4 or more past the container, and parse_paragraph then builds a paragraph with no children. In a tight list or definition list that becomes an empty TightParagraph, and `self.tree.cur().unwrap()` panics. No options are needed.
Inputs that panic: "- [a]: /u\n      \n", "- [a]:b\n\t  ", "1. [a]: /u\n       \n", and with definition lists "x\n: [a]: /u\n         \n".
Outside lists the same cause gives wrong output: "[a]: /u\n    " renders <p></p>, and "[a]: /u\n    \nb" renders <p><br />\nb</p>.
found by a seeded fuzzer, reduced automatically, and reproduced in a build with debug checks off.

**Panic with wikilinks and a reference that spans the wikilink's opener**
src/parse.rs:892 (subtract overflow), src/parse.rs:911 (slice panic without overflow checks).
"![[]j][x]]]\n\n[x]: /u" with only ENABLE_WIKILINKS panics with "byte range starts at 10 but ends at 9". A regular reference consumed the nodes, but wikilink_stack still holds the stale opener, so start_ix is past end_ix. A broken-link callback that returns a destination triggers it without the definition.
fuzzed and reduced, then run under both profiles.

## Medium

**Uppercase HTML end tags never close the block**
src/firstpass.rs:1250, tags at 2175.
"<PRE>\nx\n</PRE>\n\n*y*" emits everything to end of file as raw HTML, and SCRIPT and Style behave the same. Start tags match case-insensitively, but the end check is a case-sensitive `contains`. The spec (spec.txt:2379) says case-insensitive, and the bundled commonmark.js regex has the /i flag.

**A closing code fence followed by a tab does not close**
src/scanners.rs:528.
"```\nfoo\n```\t\nbar" keeps the fence and bar inside the code block. The spec (spec.txt:1960) allows spaces or tabs after the fence.

**CRLF inside a code span produces two spaces and defeats space stripping**
src/parse.rs:1340-1345.
CR and LF each push a space. "`a\r\nb`" gives "a  b", and "``\r\nfoo\r\n``" gives " foo " instead of "foo". Replacing LF with CRLF across all spec examples gave 13 differences of 1120, and every one was this.

**CR-only line endings break block parsing**
src/scanners.rs:508 (scan_nextline searches only for LF).
code fences, indented code, HTML blocks and attribute headings treat the rest of the document as one line. "<div>\rfoo\r\rbar" never ends, and "# a {#b}\rfoo" becomes one heading with no id. 138 of 1091 spec examples changed when LF was replaced by CR, and "```x\r" repeated takes quadratic time (1 MB, 1.0 s, ratio 4).

**A backslash-escaped bracket after a closing bracket is parsed as a reference**
src/parse.rs:1747-1757 (scan_reference).
it starts from the next node's start, which skips the backslash. "[a]\[b]\n\n[b]: /u" renders <a href="/u">a</a> and drops the text. "[a]\[]" with [a] defined does the same. "[a]\[b]" with [a] defined gives no link at all.

**Tilde and caret delimiters share one match lower bound**
src/parse.rs:1597, 1614, 1661.
a failed closer raises the shared bound past valid openers. "~~a~ b~~" stays literal with strikethrough on, and "^a b~ c^" stays literal with superscript and strikethrough on.

**With only ENABLE_SUBSCRIPT, text between double tildes loses all inline markup**
src/parse.rs:989-996.
the matched pair becomes one Text node over the whole range. "~~a *b* c~~" emits literal "*b*", and "~~a [x](y) c~~" loses its link.

**A footnote reference destroys enclosing link and image brackets**
src/parse.rs:804.
`self.link_stack.clear()` runs on every footnote reference. "[a [^1] b](u)" and "![a [^1] b](u)" both render as literal brackets, even though src/html.rs:510 has code for footnotes in image alt text. Nothing in the crate's specs covers this.

**HTML footnote numbers and anchors break on case-variant labels**
src/html.rs:139-143, 381-382.
the parser matches labels case-insensitively, but the writer's `numbers` map is case-sensitive and it writes labels verbatim. "x[^FOO]\n\n[^foo]: y" gives href="#FOO" numbered 1 and id="foo" numbered 2, so the link is dead and the numbers disagree. The Kelvin sign and k behave the same.

**Metadata block detection ignores containers**
src/scanners.rs:775-788, src/firstpass.rs:316, 1435.
with YAML blocks on, "> ---\n> quoted\n\nmore\n\n---\nend" renders an empty blockquote and "quoted" disappears, because the closer is an unrelated line outside the quote. Conversely "> ---\n> a: b\n> ---\n> text" is not treated as a block, because the closer has a prefix.

**Long non-ASCII reference labels are rejected and ASCII labels are unlimited**
src/linklabel.rs, the `b & 0b1000_0000` branch of scan_link_label_rest.
it counts non-ASCII bytes and not ASCII characters. A label of 334 CJK characters is not a reference although the limit is 999, and a 5000-character ASCII label is accepted.

**A colon with no following space starts a definition**
src/scanners.rs:287-295.
with definition lists on, "foo\n:smile: bar" and "Note\n:param x: desc" become definition lists. No spec example asserts this. From memory, commonmark-hs and pandoc require whitespace after the colon; I could not run them here.

**A table delimiter row containing a tab is not recognized**
src/scanners.rs:673-704.
"a|b\n-|-\t\n1|2" renders as a paragraph, and "a|b\n-\t|-\n1|2" renders as a paragraph plus a list.

**Hitting the table cell cap corrupts the tree**
src/firstpass.rs:529-531.
`return None` skips the pop of the pushed row, so parse_table pops the row instead of the table. With a 1000-column header and 270 one-cell rows (about 5 KB), the rest of the document nests inside the table: "<p>after..." and the blockquote appear before </tbody></table>, and the failed row is parsed again. OffsetIter hits the assertion at src/parse.rs:2215 in debug builds and yields inverted ranges in release, so caller slicing would panic.

**Tab-indented nested lists get wrong source ranges**
src/firstpass.rs:131-133.
`container_start - outer_indent` subtracts virtual tab columns from a byte offset. "- a\n\t- b" gives the inner list 3..8, which is "\n\t- b". With definition lists, "x\n:é\n\t- b" gives 4..10, which starts inside the "é", so `&text[range]` panics in the caller.

**Inline comments, processing instructions, declarations and CDATA keep container prefixes**
src/parse.rs:1411-1420.
only tags strip prefixes. "> foo <!-- a --\n> b -->" emits the "> " inside the InlineHtml event, and "- x <?a\n  b?>" keeps the indent.

**Inline CDATA end detection is wrong both ways**
src/scanners.rs:1477-1486.
it looks only at the first "]". "a <![CDATA[x]y]]> b" is rejected and escaped, while "a <![CDATA[x]> b" is accepted as raw HTML.

**Closing tags with a line ending are rejected**
src/scanners.rs:1322.
"a </b\n    > c" is escaped as text, while the matching opening tag form passes. The spec allows one line ending in a closing tag.

**Tabs count toward hard line breaks**
src/firstpass.rs:848-850.
"a\t\t\nb" and "a \t\nb" both give <br />. The spec (spec.txt:9208) says two or more spaces.

**ATX headings mishandle tabs at the end**
src/firstpass.rs:1655, 1664.
"# foo\t\n" keeps the tab, and "# foo\t#\n" and "# foo #\t\n" keep the closing sequence.

**Fenced code language class includes tabs**
src/html.rs:261.
"```rust\tfoo" gives class="language-rust\tfoo", because it splits only on a space.

**Wikilinks spanning lines keep container prefixes**
src/parse.rs:889-923.
"> [[foo\n> bar]]" gives the destination "foo\n> bar" and shows "> " in the link text.

**Wikilink pipe scan works on raw bytes**
src/scanners.rs:941.
the normal table form "|[[a\|b]]|" gives href="a%5C". "[[a`|`b]]" splits inside a code span and gives href="a%60".

**An empty wikilink pothole shows the closing brackets as text**
src/parse.rs:900-905.
"[[foo|]]" renders <a href="foo">]]</a>.

**Stale link stack entries after a wikilink break the enclosing image**
src/parse.rs:633-645.
the outer "[" stays on link_stack and `prev` is not updated, so "![a [[b]] c](url)" is not an image.

**Wikilink source ranges omit the last bracket**
src/parse.rs:944.
"[[foo]] x" reports 0..6, which is "[[foo]".

**Old-style footnotes in a blockquote nest a spurious blockquote**
src/firstpass.rs:121-123.
the next line is rescanned from its raw start, so "> [^a]:\n> b" treats its ">" as a new quote.

**Multi-line link title takes quadratic time**
src/parse.rs:1209.
each newline restarts scan_nodes_to_ix from the first node. "[a](b \"\n" + "c\n" x N + "\")" takes 6.2 s at 100 KB, 25 s at 200 KB and 100 s at 400 KB, with no options.

**Blank lines under deep lists take quadratic time**
src/parse.rs:1446, called from src/firstpass.rs:89.
every blank line walks the full spine. "- " x N + "x\n" + "\n" x N takes 15 s at 300 KB and 60 s at 600 KB, with no options.

**Underscore closers rescan the delimiter stack**
src/parse.rs:1601-1607 with 1581-1584.
a failed non-both "_" closer updates only a bound that the min() ignores. "*a " x N + "b_ " x N takes 6.8 s at 600 KB and 26.6 s at 1.2 MB. The "*" equivalent takes 0.03 s.

**Metadata scan is quadratic inside blockquotes**
src/scanners.rs:775-788.
"> ---\n> a\n>\n" x N scans to end of file per block, taking 5.6 s at 300 KB and 94 s at 1.2 MB with YAML blocks on.

**Footnote interrupt check validates the rest of the document per line**
src/firstpass.rs:2161.
from_utf8 runs on all remaining bytes for each line starting "[^". "[^a]: b\n" x N takes 2.9 s at 1.6 MB, and the time quadruples per doubling, so this affects ordinary footnote-heavy documents.

**The CLI repeats earlier files when given several**
src/main.rs:167, 173.
`input` is never cleared, so running on a.md and b.md prints a, then a+b.

**The CLI hides or misreports I/O errors**
src/main.rs:184, 219, 172-174.
non-UTF-8 stdin gives empty output and exit 0. Writing to /dev/full exits 0. File errors end in `expect` panics.

**SIMD unsafe code lacks safety justification**
src/firstpass.rs:2644, 2693, 2703, 2733.
there are no SAFETY comments and process_mask has no safety section. By reading, the code looks sound: bits fall within checked 16-byte windows and SSSE3 is checked at runtime. It is read only, because the container is aarch64 and has no x86 target.

## Low

**Table delimiter validation is too lax**
src/scanners.rs:673-710.
"a|b\n-|:", "a|b\n- -|-" and "a|b\n-:-|-" all form tables. The first has a final cell with no hyphen.

**Definition list looseness is lost directly inside a blockquote**
src/firstpass.rs:280.
"> a\n>\n> : b" is tight, while the same text unquoted is loose.

**Heading attributes drop single-character names and allow empty ones**
src/firstpass.rs:2548, 2556.
"# a {x y}" drops both attributes, and "# a {=v}" emits <h1 ="v">.

**Unquoted attribute values accept tabs**
src/scanners.rs:449-453.
"a <b c=d\te=f>" is rejected as HTML, though the spec excludes tabs from unquoted values.

**DefinitionList range overruns a following paragraph**
src/firstpass.rs:1506, 2228.
"bar\n:    baz\n\nBloze\n" gives the list a range ending at 20, overlapping the paragraph at 14..20.

**Reference expansion cap silently drops links and is undocumented**
src/parse.rs:288, 325.
a 40 KB document with one 40 KB data URI used 4 times renders the 4th as literal "[logo]". The limit appears only in private comments.

**Autofilled cell budget is per table**
src/firstpass.rs:449.
a 246 KB document of 60 tables yields 31.7 million events and 142 MB of HTML.

**VT and FF count as whitespace in block syntax**
src/scanners.rs:425-431, 627.
"#\x0cfoo" becomes a heading, though the spec requires a space or tab. Blank-line scanning uses the same whitespace set.

**LinkDef and related types cannot be named**
src/lib.rs:99-101.
RefDefs::get returns LinkDef, but `pulldown_cmark::LinkDef` fails to compile (E0425). FootnoteDef and FootnoteDefs are also pub in the private module and not re-exported (read only).

**Error types are hard to use**
src/strings.rs:12, src/lib.rs:457.
StringTooLongError is not exported, and neither it nor InvalidHeadingLevel implements Display or Error. Read only.

**Smart punctuation docs are wrong**
src/lib.rs:674.
the docs say "--" becomes an em dash, but the output is an en dash ("--" renders as "–").

**Missing documentation**
public API, for example lib.rs Options::ENABLE_TABLES.
clippy with missing_docs reports 66 items. It also reports 2 functions missing # Errors and 1 missing # Panics.

**Unwraps and bare asserts in library code**
28 unwraps and 4 assert-without-message warnings; parse.rs:1752 carries the comment "this unwrap is sus".
only parse.rs:2367/2199 were shown reachable (first High). The rest I found safe or could not trigger.

**Glob re-export and toolchain lint drift**
src/lib.rs:103; clippy overall.
`pub use crate::utils::*` hides the public surface, and 11 TODOs have no owner. Clippy fails with -D warnings on 1.99 with style lints only. That is drift from the declared 1.71.1, not checked on that version.

## Low

**Public event enums are exhaustive**
src/lib.rs (Tag, TagEnd, Event, LinkType).
each new extension variant is a breaking change.

**Feature and build setup**
pulldown-cmark/Cargo.toml, build.rs.
the `gen-tests` feature makes --all-features rewrite tests/suite/*.rs in the source tree, and `getopts` is a default dependency for library users.
