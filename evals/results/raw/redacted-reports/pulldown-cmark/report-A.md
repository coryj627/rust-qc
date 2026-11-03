# Audit report: pulldown-cmark 0.13.0

All paths are under `crate/pulldown-cmark/src/`. `cargo test --locked -p pulldown-cmark` passes at baseline (1148 suite tests plus doctests), so none of these are caught by the existing tests.

**How I tested.** All harnesses are in `tmp/h`, with a path dependency on the crate:
- a random-token fuzzer with a minimizer, checking panics, event balance and offset invariants
- LF vs CRLF equivalence, and wrapping input in containers, over every spec example
- adversarial timing runs in release mode
- direct probes, each named below

"Ran" means I executed the input and saw the quoted result. I did not modify `crate/`. To prove two root causes I patched a copy in `tmp/pc`.

## High

**1. Panic on ordinary input with default options: `parse.rs:2367` (`Parser::next`), `parse.rs:2199` (`OffsetIter::next`). Root cause at `firstpass.rs:424` and `firstpass.rs:409`.**
- After a reference definition, a following line of only whitespace with 4 or more columns is not treated as blank. `scan_next_line_or_lazy_continuation` skips the interrupt check, so an empty paragraph is created.
- Inside a tight list that paragraph is a `TightParagraph` with no child, and `self.tree.cur().unwrap()` panics.
- Ran: `"- [a]: /u\n      \n- b"`, `"> - [a]: /u\n>       \n"` and `">1. [a]:]\n    "` all give ``called `Option::unwrap()` on a `None` value``. Any service rendering untrusted markdown can be crashed.
- Same root cause outside lists, where the output is wrong rather than a panic:
  - `"[a]: /u\n    "` renders `<p></p>`
  - `"[a]: /u\n    \nb"` renders `<p><br />\nb</p>` (expected `<p>b</p>`), with the paragraph range starting at the newline

**2. Quadratic time on a multi-line link title, default options: `parse.rs:1209`.**
- `scan_link_title` calls `scan_nodes_to_ix` from the same starting `node` at every newline.
- Ran `[a](b "` + `"x\n"`×n in release: 40 KB 0.79 s, 80 KB 3.1 s, 160 KB 12.3 s.

**3. Quadratic time on emphasis because the `_` lower bound never takes effect, default options: `parse.rs:1602-1607` (with `parse.rs:1576-1585`).**
- A non-both `_` closer only sets `UNDERSCORE_NOT_BOTH`, but the getter takes `min(mod3_lower, NOT_BOTH)`. `mod3_lower` stays 0, so every closer rescans the whole stack.
- Ran `"*a_ "`×n in release: 640 KB 3.8 s, 1.28 MB 15.8 s. The mirror input `"_a* "` at 2.56 MB took 0.48 s in a debug build.
- Patching a copy to also set the base bound gave 2.56 MB in 0.083 s, which confirms the cause.

## Medium

**4. Escaped bracket read as a reference label: `parse.rs:1747-1755`.**
- `scan_reference` looks at the next node's start, and a `\[` text node starts after the backslash, so the escape is ignored.
- Ran:
  - `"[a]\\[b]"` with `[a]` and `[b]` defined gives `<a href="/b">a</a>` (expected a link to `/a` followed by literal `[b]`)
  - `"[x]\\[b]"` becomes a link although `[x]` is undefined
  - `"[a]\\[]"` loses the `[]`
- The source text is silently dropped and links go to the wrong target.

**5. HTML block end condition is case-sensitive: `firstpass.rs:1250`.**
- The start tag is matched case-insensitively, but the check is `contains("</script>")`. The spec makes the end condition case-insensitive.
- Ran `"<SCRIPT>\n</SCRIPT>\nfoo\n\nbar"`: the block never closes and the rest of the document is emitted as raw HTML. `<pre>…</PRE>` behaves the same way.

**6. CRLF inside a code span produces two spaces: `parse.rs:1340-1346`.**
- `\r` and `\n` are each converted to a space.
- Ran:
  - `` "`a\r\nb`" `` gives `<code>a  b</code>`
  - `` "``\r\nfoo\r\n``" `` gives `" foo "`, with the spaces not stripped
- This was the only class of LF-vs-CRLF difference over the spec corpus and a random fuzz run. Windows files render differently.

**7. Shared `~`/`^` lower bound suppresses valid delimiters: `parse.rs:1596-1598`, `parse.rs:1613-1615`.**
- There is one bound for all `~` run lengths and for `^`, although `~` only matches equal lengths.
- Ran:
  - `"~~a~ b~~"` stays literal (`~~a b~~` works)
  - `"~a~~ b~"` stays literal
  - `"~a^ b~"` with sub and superscript enabled stays literal (`~a b~` works)
- A patched copy returning 0 for that bound gives `<del>a~ b</del>` and `<sub>a^ b</sub>`.

**8. Quadratic time on deep lists followed by blank lines, default options: `parse.rs:1446-1480`, via `firstpass.rs:89`.**
- Blank lines pass every list item (`is_at_eol`), so each one walks the entire spine.
- Ran `"- "`×n + `"a\n"` + `"\n"`×n in release: 96 KB 1.6 s, 192 KB 6.7 s.

**9. Table autocomplete limit leaves the tree corrupted: `firstpass.rs:529-531`.**
- `return None` skips the pop of the pushed row, so `parse_table` pops the row instead of the table.
- Ran a 1000-column table with 300 one-cell rows, then `# after` and a paragraph: the heading and paragraph are emitted inside `Table`, with `End(Table)` last. A 200-row control closes normally.
- The input is a few KB.

**10. Quadratic time on metadata block scanning, `ENABLE_YAML_STYLE_METADATA_BLOCKS` only: `scanners.rs:775-789`.**
- An opener accepts a trailing tab but a closer does not, so each failed opener scans to end of file.
- Ran `"---\t\na\n\n"`×n in release: 160 KB 3.4 s, 320 KB 13.7 s.

**11. Quadratic time on nested wikilink images, `ENABLE_WIKILINKS` only: `parse.rs:889` with `scanners.rs:935-947`.**
- Images are not disabled the way links are, so each `]]` rescans the growing body for a pipe.
- Ran `"![[a "`×n + `"]]"`×n in release: 560 KB 5.3 s, 1.12 MB 21 s.

**12. Wikilink with empty display text duplicates content: `parse.rs:899-905`.**
- `body_node` becomes the closing `]`, whose `next` chain still runs on to the following siblings.
- Ran `"[[a|]] tail"`: `<a href="a">]] tail</a> tail`. This was also the fuzzer's "text outside parent" finding.

**13. Stale link stack entry after a wikilink: `parse.rs:633`, `parse.rs:642-645`.**
- Only one of the two `[` entries is popped.
- Ran:
  - `"![x [[a]]](u) tail"` gives `![x <a…>a</a>](u) tail`, so the image is lost
  - `"[y ![x [[a]]](u) tail](v)"` gives `<img src="v" alt="x a](u) tail">`
  - `"![[a]] b](c)"` gives `<img src="c" alt=" b">`, so the wikilink is overwritten

**14. Wrong source ranges when a tab is partly consumed: `firstpass.rs:131-133`, `firstpass.rs:212-213`.**
- `container_start - outer_indent` subtracts columns from a byte offset.
- Ran:
  - `"/\n:é\n\t+ ["` with definition lists gives `List` and `Item` range `4..10`, which starts inside `é`, so `&src[range]` panics for the caller
  - `">\t+ a"` gives a list range starting before the block quote's content
  - `"- a\n\t+ b"` gives a nested list starting at the previous line's newline

**15. Bare CR is not a line ending at block level: `scanners.rs:508-510`.**
- `scan_nextline` only looks for `\n`, while `parse_line` also accepts `\r`, and the spec counts CR as a line ending.
- Ran:
  - `` "```\rfoo\r```\rbar" `` gives a paragraph plus a code block with class `language-bar`
  - `"    a\r    b\r\rc"` gives one code block containing raw CRs

**16. Unterminated last line loses a CR: `firstpass.rs:1463`, `firstpass.rs:1482`.**
- `text[end-2] == '\r'` is taken to mean CRLF without checking that the last byte is `\n`.
- Ran `` "```\nfoo\rx" `` gives `foox`, and `"<div>\rx"` gives `<div>x`.

**17. Multi-line inline comment, declaration or CDATA keeps container prefixes: `parse.rs:1411-1420`, `scanners.rs:1440-1521`.**
- Tags got the #960 fix, but these forms have no newline handler.
- Ran:
  - `"> foo <!-- a\n> b -->"` emits `<!-- a\n> b -->`
  - `"> a <![CDATA[b\n> c]]>"` puts `> ` inside the CDATA data

## Low

**18. Closing code fence followed by a tab does not close: `scanners.rs:528`.** Ran `` "```\na\n```\t\nb" ``: the rest of the document becomes code. The spec allows spaces or tabs. A trailing space works.

**19. ATX heading tabs and empty text events: `firstpass.rs:1655`, `firstpass.rs:1664`, `firstpass.rs:1670`.** Ran:
- `"# foo #\t"` gives `<h1>foo #\t</h1>`
- `"# foo\t#"` gives `foo\t#`
- `"# foo\t\n"` keeps the tab
- `"# ] #"` and `"# *a* #"` emit an empty `Text("")` at `3..3` and `5..5`

**20. Inline CDATA scan is wrong: `scanners.rs:1477-1481`.** It stops at the first `]` and accepts a single `]>`. Ran:
- valid `"x <![CDATA[a]b]]> y"` is escaped as text
- invalid `"x <![CDATA[a]> y"` passes through as raw HTML

**21. Link label length is counted wrongly: `linklabel.rs:119-125`.** ASCII bytes are not counted and every non-ASCII byte is. Ran: a 1000-character ASCII label still links (the limit is 999), and a 400-character CJK label does not link.

**22. Footnote numbering and anchors are case-sensitive in the HTML writer: `html.rs:139-143`, `html.rs:381-382`.** The parser matches footnotes case-insensitively. Ran `"x[^A]\n\n[^a]: y\n"`: `href="#A"` numbered 1 and `id="a"` numbered 2, so the link is dead.

**23. `~~` with only `ENABLE_SUBSCRIPT` discards inline content: `parse.rs:989-996`.** The node becomes `Text` while its children are dropped. Ran:
- ``"~~*a*~~"`` gives `~~*a*~~`
- ``"~~[x](/y)~~ ~~`c`~~"`` is all literal source, with no markup parsed inside

**24. Link title in a table drops a pipe: `parse.rs:1228-1244`.** `c` is stale after the `\\|` branch. Ran `[x](y "a\\|!b")` in a cell: title `a!b` (expected `a|!b`).

**25. `\|` not unescaped in table inline HTML and autolinks: `parse.rs:399`, `parse.rs:1409`.** The comment at `firstpass.rs:894-896` says inline parsing should behave as if these were already stripped. Ran: `<a b="c\|d">` keeps the backslash, and `<http://a\|b>` gives href `http://a%5C%7Cb`.

**26. Table delimiter row validation: `scanners.rs:674-689`.** Ran against GFM:
- `"a|b\n-:-|-"` and `"a|b\n- -|-"` are accepted as tables
- `"a|b\n-\t|\t-"` is rejected because of the tabs

I judged this against the GFM rules, with no reference implementation run.

**27. Fence info string split on space only: `html.rs:261`.** Ran `` "```rust\tfoo" ``: `class="language-rust\tfoo"`.

**28. Heading attributes: `firstpass.rs:2548`, `firstpass.rs:2555-2557`.** Ran:
- `# a {b}` silently drops a one-character attribute, which the `lib.rs:681-685` docs allow
- `# a {=v}` emits malformed `<h1 ="v">`

**29. Unquoted attribute value accepts a tab: `scanners.rs:449-454`.** Ran `"x <a b=c\td=e>"`: a valid tag is rendered as escaped text.

**30. CLI handling of multiple files and bad input: `main.rs:167`, `main.rs:173`, `main.rs:184`, `main.rs:219`.**
- `input` is never cleared between files. Ran `pulldown-cmark a.md b.md`: output is `<p>one</p>` then `<p>one\ntwo</p>`.
- Invalid UTF-8 on stdin is ignored. Ran: empty output with exit code 0.
- The result of `write_html_io` at line 219 is discarded (read only).

**31. Doc error: `lib.rs:674`.** It says `--` becomes `—`, but the code at `firstpass.rs:1143-1144` emits `–`. Read only.

## Checked and clean
- **Entities:** all 2125 HTML5 entities matched Python's table.
- **`is_punctuation`:** no P or S characters missing against Unicode 14. The 57 extras were unassigned in that version, so they are probably newer characters.
- **Unsafe code:** the only `unsafe` is the x86-only SIMD path, which I read but could not run on aarch64. I did not run Miri because the code built here has no `unsafe`.

## Process notes
- **Slip outside the workspace:** one of my commands contained a stray `cat > /tmp/null`, which created an empty file outside the workspace and hung the command for about 8 minutes. To clear it I ran `ps` and `lsof`, killed that one `cat` (pid 59842) and removed the file. The clock includes that time.
- **Repository text:** nothing in the repository addressed me.
- **Design choices I did not report:** the project's own specs document these, so I left them out:
  - intraword `x^2^` and `H~2~O` staying literal
  - `[[first\|second]]` giving href `first%5C`
  - `\{` before heading attributes

**Findings: 31. Last `./clock` reading: 29 minutes used of 45.**
