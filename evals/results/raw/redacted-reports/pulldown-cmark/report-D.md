# Audit report: pulldown-cmark 0.13.0 (`crate/pulldown-cmark/src/`)

**Method.** Everything ran through `./run` on aarch64.
- Baseline: `cargo test --locked -p pulldown-cmark` passes, including `--all-features`. Clippy shows only style warnings.
- A random-fragment fuzzer in `probes/fuzz`. It checked for panics, range validity, balanced events and nested ranges, and minimised failures.
- Targeted probe tables, scaling timers, and the CLI binary.
- Metamorphic runs over the spec examples: LF to CRLF, LF to CR, and wrapping in `> ` or `1. `.

**Limits.**
- The x86 SIMD path (`firstpass.rs:2568+`) could not run here. I read it and found no defect.
- "Expected" below comes from `third_party/CommonMark/spec.txt`, or from the crate's own behaviour on an equivalent input.

Unless a finding says otherwise, each was confirmed by running it, and the output shown is real.

## High: panics on untrusted input (all reproduce in debug and release)

**1. Default options: whitespace-only line after a link definition in a tight list.** Crash at `parse.rs:2367` (`Parser`) and `parse.rs:2199` (`OffsetIter`). Root cause is `firstpass.rs:398-409` with `:424-434`.
- `scan_next_line_or_lazy_continuation` accepts a line of 4+ columns of whitespace without checking that it is blank, so `parse_paragraph` creates a paragraph with no children. A tight list makes it a `TightParagraph`, and `self.tree.cur().unwrap()` then hits `None`.
- `"- [a]: /u\n      \n"` panics. So do `"> - [a]: /u\n>       \n"` and `"- [a]:u\n\u{c}"`.
- Outside lists the same bug gives wrong output with no panic: `"[a]: /u\n      \n"` gives `<p></p>`, and `"[a]: /u\n      \nfoo\n"` gives `<p><br />\nfoo</p>`.

**2. `ENABLE_HEADING_ATTRIBUTES`: setext heading in a list, attribute line indented past the container.** Root cause is `firstpass.rs:744-753` with `tree.rs:236-240`.
- `"- a\n   {#x}\n  ---\n"` panics. Debug fails the assert at `tree.rs:237`. Release panics at `parse.rs:2256` with "byte range starts at 7 but ends at 6".
- `new_end` falls before the attribute node's start, and `truncate_siblings` then writes `end < start` into that node.

**3. `ENABLE_TASKLISTS | ENABLE_HEADING_ATTRIBUTES`: task item whose only text is a backslash, followed by a setext underline.** `firstpass.rs:712-720` panics at `:2071`.
- `"- [ ] \\\n  ---\n"` gives "slice index starts at 6 but ends at 5".
- `header_end` is taken from the `TaskListMarker` node, which lies before the paragraph start.

**4. `ENABLE_WIKILINKS`: image bracket reused after it became an image.** `parse.rs:883-892, 911`.
- `"![[x]y](u)]]"` panics. Debug reports subtract overflow at `:892`. Release reports "byte range starts at 11 but ends at 10" at `:911`.
- After `![…](u)` resolves, the wikilink stack still holds that node, and `next.next` lands past the closing bracket.

## Medium: denial of service (quadratic time)

**5. Default options: multi-line link title.** `parse.rs:1209`.
- `scan_nodes_to_ix` restarts from `node` at every newline in the title.
- `[a](/u "` plus `x\n` repeated, closed or unclosed: 31 KB takes 0.55 s, 62 KB takes 2.4 s, 125 KB takes 9.4 s. Each doubling costs 4x.

**6. Nested wikilink images.** `parse.rs:889`.
- Image openers are not disabled, so each close rescans the whole inner body.
- `"![[" * n + "a" + "]]" * n`: 208 KB takes 1.1 s, 416 KB takes 4.3 s.

**7. `ENABLE_YAML_STYLE_METADATA_BLOCKS` in a container.** `scanners.rs:775-789`.
- Each opener scans to EOF for a closer at raw column 0.
- `"> ---\n"` repeated: 124 KB takes 0.75 s, 249 KB takes 3.6 s.

## Medium: wrong output

**8. CRLF in code spans doubles the space.** `parse.rs:1340-1346`.
- `\r` and `\n` each produce a space. `` "`a\r\nb`" `` gives `<code>a  b</code>`.
- Stripping then removes only one space, so `` "``\r\nfoo\r\n``" `` gives `<code> foo </code>` where LF gives `foo`. CRLF-mode metamorphic runs differ on 6 spec examples and 7 regression examples; the ones printed are all this code-span issue.

**9. CR-only line endings are not honoured in several block parsers.** `scanners.rs:508-510` (`scan_nextline` only finds `\n`), plus `firstpass.rs:1463, 1482`.
- `"~~~\rfoo\r~~~\r"` gives `<pre><code class="language-foo~~~"></code></pre>`.
- `"    a\r    b\r"` keeps the indent in the code text.
- The `end-2` check also eats a CR that is not part of CRLF: `"<pre>\rx"` gives `<pre>x`.
- 85 of 652 spec examples change under LF to CR.

**10. HTML block end condition is case-sensitive and tied to the opening tag.** `firstpass.rs:1250`, `:2173-2176`.
- The spec says case-insensitive and "need not match the start tag".
- `"<PRE>\nfoo\n</PRE>\n\n*bar*\n"` and `"<style>\nfoo\n</script>\n\n*bar*\n"` both swallow the rest of the document as raw HTML.

**11. Container prefixes leak into inline comments, PIs, CDATA and declarations.** `parse.rs:1411-1420`.
- These return an empty span, so the raw source slice is emitted. Open tags do strip the prefix.
- `"> a <!-- b\n> c -->"` gives `<!-- b\n> c -->`.
- The prefix can also end the construct: `"> a <!X b\n> c >"` gives `<!X b\n>c &gt;`.

**12. Footnote labels match case-insensitively in the parser but case-sensitively in the HTML writer.** `html.rs:139-143, 379-382`.
- `"[^A]\n\n[^a]: y\n"` gives `href="#A"` with `id="a"`, numbered 1 and 2. The link is dead and the numbering is wrong.

**13. A resolved footnote reference clears every open bracket.** `parse.rs:804`.
- `"![a [^1] b](/u)\n\n[^1]: x\n"` gives literal `![a <sup>…</sup> b](/u)`.
- `[a [^1] b](/u)` loses its link only when the footnote is defined.

**14. Closing code fence followed by a tab is not recognised.** `scanners.rs:528`.
- `` "```\nfoo\n```\t\nbar\n" `` keeps the fence and `bar` inside the code block. The spec allows spaces or tabs.

**15. ATX headings mishandle tabs.** `firstpass.rs:1655, 1664`.
- `"# foo\t\n"` gives `<h1>foo\t</h1>`.
- `"# foo #\t\n"` gives `<h1>foo #\t</h1>`.
- `"# foo\t#\n"` gives `<h1>foo\t#</h1>`.

**16. Tabs or form feeds trigger hard line breaks.** `firstpass.rs:848-850`.
- `"a\t\t\nb"` and `"a \t\nb"` give `<br />`. The spec requires two spaces.

**17. Superscript and tilde runs share one lower bound.** `parse.rs:1597, 1614`.
- `^a b^` works, but `"^a~ b^"` and `"~a^ b~"` stay literal.
- A failed `~` closer blocks later `^` closers, and the reverse.

**18. `ENABLE_SUBSCRIPT` without strikethrough drops inline markup inside `~~`.** `parse.rs:993-995`.
- A `Text` node keeps its children and emits the whole source range.
- `"~~*a*~~"` and `"~~[a](/u)~~"` come out as raw source.

**19. Link label limit is miscounted in both directions.** `linklabel.rs:119-125`.
- ASCII bytes are not counted, so a 1000-ASCII-character label still links. The limit is 999.
- Each byte of a multibyte character is counted, so 600 `é` characters are rejected.

**20. Cell budget overflow leaves the table open.** `firstpass.rs:529-531` returns without popping, so `:463` pops the row rather than the table.
- With 1000 columns and 300 one-cell rows, later rows and `after table` render inside the table: `…<p>after table</p>\n</tbody></table>`.

**21. Metadata closer search ignores containers, so content is lost.** `scanners.rs:775-788` against `firstpass.rs:1432`.
- `"> ---\n> a\n---\nb\n"` gives `<blockquote>\n</blockquote>\n<hr />\n<p>b</p>`. The line `a` disappears.

**22. Wikilink defects in `parse.rs`.** Four separate behaviours, each run:
- `"[[a|]]"` gives `<a href="a">]]</a>` (`:900-905`).
- `"> [[a\n> b]]"` gives `href="a%0A%3E%20b"` and text `a\n&gt; b`, because the prefix leaks (`:911`).
- In a table, `[[a\|b]]` gives `href="a%5C"` (`scanners.rs:941`).
- The first `[` of `[[` stays on `link_stack` (`:633, 642-645`), so `"![x [[a]] y](/u)"` never becomes an image, and `"![[a]] b](/u)"` loses the wiki target.

## Medium: API and CLI

**23. Source ranges are wrong in `into_offset_iter`.**
- Partly consumed tabs: `firstpass.rs:131-135, 213` subtract columns from bytes. `"- a\n\t- b\n"` starts the inner list at the `\n`. `">\t- b\n"` starts the list at `>`. With definition lists the start lands inside a character (`"|\n:\u{2003}\n\t- a"` starts inside U+2003), so `&src[range]` panics.
- Collapsed references: `parse.rs:747-755, 811, 839` omit `[]`. `[a][] x` gives `0..3`, and `BrokenLink.span` is `2..5` for `[a][]` at offset 2.
- Task markers: `firstpass.rs:157, 603-612` put the marker range outside its paragraph. `"- [ ] a\n\n- b\n"` gives Paragraph `6..8` containing marker `2..5`.

**24. `write_html_io` never flushes.** `html.rs:595`.
- Its docs recommend `BufWriter`. A `BufWriter` over a failing writer returns `Ok(())` with nothing written. The unbuffered writer does return the error.

**25. CLI errors and input handling.** `main.rs:173, 184, 219`.
- `input` is not cleared between files. `pulldown-cmark a.md b.md` prints `first`, then `first\nsecond`, and `-R` double-counts broken links ("3 broken links" for 2).
- Invalid UTF-8 on stdin gives empty output and exit 0.
- `> /dev/full` also exits 0.

## Low

**26. Table delimiter row is too permissive and too strict.** `scanners.rs:673-710`.
- Accepted: `-:-`, `- -`, `::-`, and a final column of only `:` (`a|b\n-|:`).
- Rejected: `-\t|-`, and a trailing tab (`|-|\t`).

**27. CDATA with an inner `]` is rejected.** `scanners.rs:1477-1484` checks only the first `]` run.
- `foo <![CDATA[a]b]]> bar` is escaped as text. Without the `]` it is inline HTML.

**28. Closing tag with a line ending is rejected.** `scanners.rs:1322`.
- `"foo </a\n    >"` is escaped, although spec.txt:8990 allows it. The open-tag form works.

**29. Table link title loses a pipe.** `parse.rs:1228-1244` uses a stale `c`.
- `\\|!` in a title gives `x!`. `\\|y` correctly gives `x|y`.

**30. Fence info string splits only on spaces.** `html.rs:261`.
- `` "``` rust\tx" `` gives `class="language-rust\tx"`.

**31. Definition lists inside a blockquote are always tight.** `firstpass.rs:280`.
- `"> a\n>\n> : b\n"` gives `<dd>b</dd>`, while the same text outside a quote gives `<dd><p>b</p></dd>`.

**32. Heading attribute edge cases.** `firstpass.rs:2548`, `:2555-2557`.
- `# Intro {x}` silently drops `{x}`: no text and no attribute.
- `{=v}` emits `<h1 ="v">`.

**33. VT and FF count as spaces and tabs.** `scanners.rs:429-431, 627`.
- `"#\u{c}foo"` becomes a heading.
- `"foo\n\u{c}\nbar"` loses the form feed. This also feeds finding 1.

**34. DEL is allowed where ASCII controls are forbidden.** `scanners.rs:1377, 979`.
- `<http://a\x7fb>` and `[a](b\x7fc)` both become links.

**35. Empty `Text` events are inconsistent.**
- `"# *a* #"` emits `Text("")` (`firstpass.rs:1670`).
- `TextMergeStream` drops two adjacent empties but keeps a single one: `[Rule, Text(""), Rule]` (`utils.rs:104` against `:126-130`).

**36. Public type cannot be named.** `lib.rs:99-102`.
- `RefDefs::get` returns `&LinkDef`, but `use pulldown_cmark::LinkDef` fails with E0432.
- `StringTooLongError` is not re-exported either; I read that one and did not compile it.

**37. Documentation error (read only).** `lib.rs:674`.
- It says `--` becomes an em dash. The code emits an en dash, as my smart-punctuation probe showed.

## Process notes

- `crate/` was not changed. My probes are in `probes/` and `tmp/`.
- No repository text addressed me.
- Small harness-patching Python scripts ran on the host and edited only files in `probes/`. Generated and compiled code ran only through `./run`.

**Findings: 37. Last `./clock` reading: 27 minutes used of 45.**
