# Audit report: pulldown-cmark 0.13.0

## Verdict

The parser is memory-safe by construction in its default build and passes its own suite, but it has real defects. Default options can panic on a five-token input. With `ENABLE_WIKILINKS`, a 168-byte input produces 84 million events, and the growth is exponential. There are also four quadratic paths and many wrong-output cases, mostly around non-LF line endings, tabs, offsets and the newer extensions.

Counts: 2 Blocker, 29 Major, 20 Minor, 2 Note.

Unless a finding says "read only", I established it by running the input through the public API from a throwaway crate in `tmp/probe`, in a release build with debug assertions and overflow checks on. "Expected" results come from the bundled CommonMark 0.31.2 spec or my reading of it. No reference implementation was available to compare against.

One side effect in `crate/`: running with `--all-features` turned on `gen-tests`, so `build.rs` rewrote the 18 files in `crate/pulldown-cmark/tests/suite/`. I made no other changes there.

## Blocker

### Panic on a whitespace-only line after a link definition in a list item
Location: root cause `src/firstpass.rs:424-435` (`scan_next_line_or_lazy_continuation`), called at `src/firstpass.rs:398`. The panic is at `src/parse.rs:2367` and, for the offset iterator, `src/parse.rs:2199`.
Problem: when the next line has at least 4 columns of whitespace after the container prefix, the `scan_space(4)` branch treats it as a continuation without checking for a blank line. `parse_paragraph` then creates an empty paragraph.
- In a tight list that paragraph is an empty `TightParagraph`, and `self.tree.cur().unwrap()` panics. `Parser::new("- [a]: /u\n      \n").count()` panics with default options. So do `1. [a]: /u\n       \n`, the CRLF form, and definition-list items.
- Outside tight lists the output is wrong instead: `[a]: /u\n    \nfoo [a]\n` gives `<p><br />\nfoo <a href="/u">a</a></p>`, and `[a]: /u\n    \n` alone gives an empty paragraph.

Established: ran it; also found independently by fuzzing (no other panic class appeared in about 1.2M token-fuzz inputs and several minutes of structural fuzz).
Fix: in that function, return `None` when the line is blank, before the indented-continuation branch.

### Exponential event growth from a wikilink with an empty display text
Location: `src/parse.rs:900-905` (`handle_wikilink`).
Problem: for `[[a|]]`, `scan_nodes_to_ix` returns the closing `]` node itself as the link body. The link's child chain then runs on into the rest of the paragraph, and the same nodes stay linked as siblings. `[[a|]] tail` renders as `<a href="a">]] tail</a> tail`. Repeating the pattern doubles the events for every 7 bytes:
- n=8 (56 bytes): 1,276 events.
- n=16 (112 bytes): 327,676 events.
- n=24 (168 bytes): 83,886,076 events.

At 4096 repeats the container was killed (exit 137, out of memory or time). `![[a|]]` and `[[a]|]]` hit the same path.
Established: ran it and counted events.
Fix: if the body node is `cur_ix`, either emit a wikilink with no children or treat the input as not a wikilink.

## Major

### Quadratic time on multi-line link titles
Location: `src/parse.rs:1209` (`scan_link_title`).
Problem: each newline in a title rescans the node chain from the start through `scan_nodes_to_ix`. `[a](b "` plus `c\n` repeated takes 2.5 s at 66 KB and quadruples with each doubling, with default options. It happens even when the title never closes, and also inside block quotes.
Established: doubling timing harness.
Fix: keep a moving node cursor instead of restarting from `node`.

### Quadratic time on blank lines inside deep lists
Location: `src/parse.rs:1446-1480` (`scan_containers`), called per line from `src/firstpass.rs:89`.
Problem: a blank line passes every list level at no byte cost, so each blank line costs the full nesting depth. `"- "` repeated n times, then `a`, then n newlines takes 1.7 s at 98 KB, default options. Nesting depth has no limit.
Established: doubling harness.
Fix: short-circuit blank lines, or cap list nesting depth.

### Quadratic UTF-8 validation on lines starting with a footnote bracket
Location: `src/firstpass.rs:2161`.
Problem: `from_utf8(&bytes[2..])` validates the whole remainder of the document for every interrupt check on a line starting with `[^`. 1.6 MB of `[^n]: b` lines takes 1.9 s, and 1.6 MB of `[^a x` lines takes 3.6 s, with growth of about 4x per doubling.
Established: doubling harness; the cause was read from the code, not profiled.
Fix: scan the label from `&str`, or limit the slice to the current line.

### Quadratic time on nested wikilink images
Location: `src/scanners.rs:935` via `src/parse.rs:889`, with `src/parse.rs:1786-1793` not disabling images.
Problem: `![[a` repeated n times then `]]` repeated n times rescans the growing body for a pipe at every closer. 393 KB takes 3.1 s.
Established: doubling harness; cause read from the code.
Fix: bail out on nested wikilink closers, or scan for the pipe once.

### CRLF inside a code span produces two spaces
Location: `src/parse.rs:1340-1346` (`make_code_span`).
Problem: CR and LF are converted separately. `` `a\r\nb` `` gives `<code>a  b</code>` where LF input gives `<code>a b</code>`, including in block quotes.
Established: ran it, plus a CRLF-versus-LF fuzz.
Fix: consume the whole line ending with `scan_eol`.

### Carriage-return-only line endings break code and HTML blocks
Location: `src/scanners.rs:508-510` (`scan_nextline` looks only for `\n`), used at `src/firstpass.rs:1240,1289,1329,1376,1408`.
Problem: the spec counts a lone CR as a line ending, and paragraphs honor it, but these blocks do not:
- ```` ```\rcode\r```\r\rpara\r ```` gives `<p>```\ncode</p>\n<pre><code class="language-para"></code></pre>`.
- `    a\r    b\r\rpara\r` becomes one code block containing the rest of the file.
- `<div>\r\r*p*\r` stays one HTML block.
- `<div>\rx` loses the CR (`src/firstpass.rs:1482`).

Established: ran it.
Fix: make `scan_nextline` treat a lone CR as a line end, and fix the `end - 2` CRLF check.

### HTML block end tags are matched case-sensitively
Location: `src/firstpass.rs:1250`.
Problem: start tags match case-insensitively, and the spec says the end condition is too, but `contains("</pre>")` is exact. `<PRE>\n\ntext</PRE>\n\n*x*\n` and `<SCRIPT>\n...\n</SCRIPT>\n\n*x*\n` never close, so the rest of the document is emitted as raw HTML.
Established: ran it.
Fix: compare with ASCII case folding.

### A closing code fence followed by a tab is not recognized
Location: `src/scanners.rs:528`.
Problem: only spaces are allowed after the fence. ```` ```\ncode\n```\t\nafter\n ```` puts the fence and `after` inside the code block.
Established: ran it.
Fix: accept spaces and tabs.

### ATX headings mishandle tabs
Location: `src/firstpass.rs:1655` and `src/firstpass.rs:1664`.
Problem: trailing trimming and closing-sequence detection consider only spaces.
- `# foo\t` gives `<h1>foo\t</h1>`.
- `# foo\t##` gives `<h1>foo\t##</h1>`.
- `# foo ##\t` gives `<h1>foo ##\t</h1>`.

Established: ran it.
Fix: treat tab like space in both checks.

### Trailing tabs create hard line breaks
Location: `src/firstpass.rs:848-850`.
Problem: the check counts tabs, but the spec requires two or more spaces. `foo\t\t\nbar`, `foo \t\nbar` and `foo\t \nbar` all give `foo<br />`.
Established: ran it.
Fix: count only spaces for the hard-break decision.

### Inline CDATA handling is wrong in both directions
Location: `src/scanners.rs:1477-1486`.
Problem: the scan stops at the first `]` and never requires `]]>`.
- `foo <![CDATA[ a] b ]]> bar` is escaped as text.
- `foo <![CDATA[a]> bar` is accepted as raw HTML.

Established: ran it.
Fix: search for the literal `]]>`.

### Multi-line inline comments, CDATA, processing instructions and declarations keep container prefixes
Location: `src/parse.rs:1411-1420`.
Problem: only the tag path strips prefixes. `> x <![CDATA[ a\n> b ]]> y` emits `<![CDATA[ a\n> b ]]>`, so the quote marker ends up inside the raw HTML. The same happens with `<!--`, `<?` and `<!D`, including list indentation.
Established: ran it.
Fix: rebuild the span with `skip_container_prefixes`, as the tag path does.

### Failed `~` and `^` closers wrongly block later valid closers
Location: `src/parse.rs:1597,1614,1661`.
Problem: there is one lower bound for all `~` run lengths and it is also shared with `^`.
- `~~a~ b~~` stays literal; the expected result is `<del>a~ b</del>`.
- With superscript and strikethrough on, `~a^ b~` and `^a~ b^` also stay literal.

Established: ran it; cause read from the code.
Fix: keep separate bounds per delimiter and run length.

### With subscript only, double-tilde spans lose all inner markup
Location: `src/parse.rs:993-995`.
Problem: a `Text` body is written over a node that spans the whole run and has children, so the children are never emitted. With `ENABLE_SUBSCRIPT` alone, `~~*a*~~ b` and `x ~~a [l](u) b~~ y` come out as literal source. The emphasis and the link should still parse.
Established: ran it.
Fix: leave the delimiters as text nodes and keep the content as siblings.

### Intraword superscript and subscript do not work
Location: `src/firstpass.rs:2281-2294` and `src/firstpass.rs:2329`.
Problem: `x^2^` and `H~2~O` stay literal, which are the usual uses of these extensions. The docs do not mention this limit. `specs/super_sub.txt` covers `~` only and has no intraword `^` case, so this may be intended, but it is at minimum undocumented.
Established: ran it.
Fix: allow intraword `^` and `~` for these options, or document the limit.

### A footnote reference destroys the enclosing link or image
Location: `src/parse.rs:804`.
Problem: `self.link_stack.clear()` removes outer brackets. `[a [^1] b](c)` and `![a [^1] b](c)` render as literal brackets around the footnote. My expectation that the outer link should survive comes from recalled cmark-gfm behavior, not from a run.
Established: ran it.
Fix: pop only the footnote's own bracket.

### Footnote labels match case-insensitively but the HTML writer does not
Location: `src/html.rs:139-143` and `src/html.rs:381-382`.
Problem: `x[^A] y\n\n[^a]: d` gives `href="#A"` numbered 1 and `id="a"` numbered 2, so the link goes nowhere.
Established: ran it.
Fix: normalize the label for numbering and anchors.

### Hitting the table cell cap leaves the tree open
Location: `src/firstpass.rs:529-531`.
Problem: the early `return None` skips `self.pop`, so the row stays pushed and the table stays open. With about 6 KB of input (1000 columns and 300 one-cell rows), the rest of the document nests inside the table. 263 rows parse, then two paragraphs appear inside it, and `</tbody></table>` comes after `<p>after <em>x</em></p>`.
Established: ran it and counted events.
Fix: pop the row and table before returning, then continue with truncated rows.

### Reference links silently stop resolving after an undocumented budget
Location: `src/parse.rs:288` and `src/parse.rs:325`.
Problem: once URL and title bytes exceed `max(text.len(), 100_000)`, references render as literal text, with no error or callback.
- A 39 KB document using one 69-byte URL 3000 times gets 1450 links and 1550 literal `[r]`.
- A 390 KB document with 30000 uses gets 5654 links.

Established: ran it.
Fix: document the limit and raise it, or limit output differently (for example, count only allocated copies).

### Collapsed references have a truncated source range and broken-link span
Location: `src/parse.rs:747-755`, used at `src/parse.rs:811` and `src/parse.rs:839`.
Problem: `[foo][]` reports `0..5`, and `![foo][]` reports `0..6`. `BrokenLink.span` is `0..5` too. No event covers the `[]`, so tools that rewrite source by span corrupt it.
Established: ran it.
Fix: end the range at the end of the `[]`.

### Wikilink ranges miss the final bracket
Location: `src/parse.rs:944`.
Problem: the end is `end_ix + 1`, but two closers follow `end_ix`. `[[a|b]]` reports `0..6`, which covers `[[a|b]`.
Established: ran it.
Fix: use `end_ix + 2`.

### Wikilink names spanning lines keep the newline and container prefix
Location: `src/parse.rs:889-923`.
Problem: `> [[a\n> b]] t` gives `href="a%0A%3E%20b"` and link text `a\n&gt; b`. CRLF input gives `%0D%0A`.
Established: ran it.
Fix: reject line breaks in wikilink names, or normalize them.

### A leading byte order mark defeats block parsing of the first line
Location: `src/firstpass.rs:69-73`.
Problem: `\u{feff}# a` gives `<p>\u{feff}# a</p>`, and a metadata block after a BOM becomes `<h2>`. The CLI passes file contents through unchanged, so a BOM-prefixed file hits this directly. From memory, cmark and commonmark.js strip it; I could not run either here.
Established: ran it.
Fix: skip a leading BOM in `run` while keeping offsets correct.

### CLI re-renders earlier files when given several
Location: `src/main.rs:170-174`.
Problem: `input` is never cleared. `pulldown-cmark a.md b.md` prints `<h1>one</h1>`, `<h1>one</h1>`, `<h1>two</h1>`. Broken links are also reported repeatedly.
Established: ran the binary.
Fix: clear `input` for each file.

### CLI hides read and write failures
Location: `src/main.rs:184`, `src/main.rs:219`, and `src/main.rs:172,174`.
Problem:
- Non-UTF-8 stdin gives empty output and exit 0.
- Writing to `/dev/full` gives exit 0.
- A bad file path panics.

Established: ran the binary.
Fix: propagate these through `main`'s `io::Result`, and flush the buffer explicitly.

### Old-style footnote in a block quote nests a second quote
Location: `src/firstpass.rs:118-124`.
Problem: after skipping the blank remainder of the line, the next line is scanned without container handling. `> [^a]:\n> x\n` with `ENABLE_OLD_FOOTNOTES` wraps `x` in an extra blockquote inside the footnote.
Established: ran it; found by a blockquote-neutrality fuzz.
Fix: rescan containers before continuing.

### Smart punctuation docs are wrong
Location: `src/lib.rs:674`.
Problem: the docs say `--` becomes an em dash. It becomes an en dash (`src/firstpass.rs:1144`). `a--b---c` gives `a–b—c`.
Established: ran it.
Fix: correct the doc.

### Public types cannot be named
Location: `src/lib.rs:99-102`, `src/parse.rs:1811`, `src/strings.rs:12`.
Problem: `RefDefs::get` returns `LinkDef`, and `InlineStr::try_from` returns `StringTooLongError`, but neither is re-exported. `fn f(_: &pulldown_cmark::LinkDef)` fails with E0425.
Established: compiled it.
Fix: re-export both.

### Undocumented unsafe in the SIMD path
Location: `src/firstpass.rs:2644`, `src/firstpass.rs:2693`, `src/firstpass.rs:2703`, `src/firstpass.rs:2733`.
Problem: the blocks have no `// SAFETY:` comment and the `unsafe fn`s have no `# Safety` section. I found no soundness bug.
Established: read only. This code is x86_64-only, and the x86_64 target is not installed here.
Fix: document the bounds and `ssse3` preconditions.

## Minor

### Language class is not split on tabs
Location: `src/html.rs:261`.
Problem: ```` ``` rust\tfoo ```` gives `class="language-rust\tfoo"`. Ran it.
Fix: split on any ASCII whitespace.

### Table delimiter row validation is too loose and too strict
Location: `src/scanners.rs:673-683`.
Problem: `| - - |` and `| -:- |` are accepted (the latter as right-aligned), while `|\t---\t|` is rejected. Ran it.
Fix: validate each cell as optional colon, hyphens, optional colon, with spaces or tabs around it.

### Link label length limit counts the wrong thing
Location: `src/linklabel.rs:58` and `src/linklabel.rs:119-125`.
Problem: ASCII is not counted, and multi-byte characters count per high byte. A 1200-`a` label works (the spec limit is 999), while 400 `日` or 600 `é` fail. Ran it.
Fix: count characters.

### Blocks at end of file lack the final newline
Location: `src/firstpass.rs:1331` and `src/firstpass.rs:1463`.
Problem: ```` ```\nabc ```` and `    abc` give `<pre><code>abc</code></pre>`. In a quote, `> <?` gives `<?</blockquote>`. Ran it.
Fix: synthesize the newline, as the code comment at line 1331 already asks.

### Heading attribute edge cases
Location: `src/firstpass.rs:2548` and `src/firstpass.rs:2555-2557`.
Problem: `# h {a}` drops the one-letter attribute, and `# h {=v}` gives `<h1 ="v">`. Ran it.
Fix: test the first byte properly and reject empty keys.

### A wikilink inside an image alt prevents the image
Location: `src/parse.rs:633-645`.
Problem: only one bracket is popped for two closers. `![x [[a]]](b)` stays literal around the link. Ran it.
Fix: pop both brackets.

### Definition looseness differs inside block quotes
Location: `src/firstpass.rs:280`, with `src/firstpass.rs:217-224`.
Problem: `t\n\n: d` is loose, but `> t\n>\n> : d` is tight. Ran it.
Fix: record the blank line in this case.

### Empty text events
Location: `src/firstpass.rs:1670`.
Problem: `# *a* #` emits `Text("")` at `5..5`. Ran it.
Fix: drop the node when it becomes empty.

### Tab indentation pushes list ranges back into the previous line
Location: `src/firstpass.rs:131-133` and `src/firstpass.rs:213`.
Problem: columns are subtracted from a byte index. `- a\n\t- b` gives the inner list `3..9`, starting at the previous newline. Ran it.
Fix: track the byte offset separately.

### Task markers fall outside their paragraph's range
Location: `src/firstpass.rs:145` and `src/firstpass.rs:604`.
Problem: in `- [x] a\n\n- b`, the paragraph is `6..8` and its marker child is `2..5`. Ran it.
Fix: start the paragraph at the marker.

### NUL is not replaced
Location: no replacement exists anywhere in the text path.
Problem: the spec requires U+FFFD, but `a\0b` passes through, including in code spans. `&#0;` is handled correctly. Ran it.
Fix: replace NUL in text output, or document that callers must.

### Form feed and vertical tab count as block whitespace
Location: `src/scanners.rs:426-431`.
Problem: `a\n\x0c\nb` gives `<p>a\n\nb</p>`, and `#\x0cfoo` becomes a heading. Ran it.
Fix: use spaces and tabs only in block scanning.

### Closing tags cannot span a line
Location: `src/scanners.rs:1322`.
Problem: `foo </a\n     > bar` is escaped as text, though the spec allows one line ending. Ran it.
Fix: use the newline-aware scan for closing tags too.

### CRLF is kept in math and link titles
Location: `src/parse.rs:1294-1297` and `src/parse.rs:1208-1212`.
Problem: `$a\r\nb$` and `[a](b "c\r\nd")` keep `\r\n` in their output. Ran it.
Fix: normalize to `\n`.

### Inline links can read into a heading's attribute block
Location: `src/parse.rs:391-392`.
Problem: `block_end` includes the attribute text. `# [a](b{c)}` gives `<h1 c)=""><a href="b%7Bc">a</a></h1>`, using the same bytes twice. Ran it.
Fix: bound inline scanning at the content end.

### Error types lack `Display` and `Error`
Location: `src/strings.rs:12` and `src/lib.rs:457`.
Problem: `StringTooLongError` and `InvalidHeadingLevel` implement neither. Read only.
Fix: implement both.

### Documentation gaps
Location: throughout the public API, for example `src/lib.rs:651,669,670,732,733`.
Problem: clippy reports 71 missing-docs items, 7 missing `# Errors` and 2 missing `# Panics` sections. `src/lib.rs:274` also has a stale description of `Tag::Image`.
Fix: document the public items, starting with the option flags.

### Production unwraps and bare assertions
Location: about 25 `unwrap` calls, including `src/parse.rs:1752`, which its own TODO calls suspect, and 4 assertions without messages.
Problem: no panic other than the Blocker was reached. Clippy advisory pass.
Fix: use `expect` with the invariant stated, or handle the case.

### Public surface issues
Location: `src/lib.rs:86,103` and `src/parse.rs:2174`.
Problem: a glob re-export gives `utils` items two public paths, and `OffsetIter` lacks the `FusedIterator` impl that `Parser` has. Read only.
Fix: re-export by name and add the impl.

### Clippy failures on this newer toolchain
Location: `build.rs`, plus style lints in `src/`.
Problem: `clippy --all-targets --all-features -D warnings` fails on Rust 1.99.1, and there are small style warnings. This is toolchain drift against an MSRV of 1.71.1; I could not test the project's own toolchain.
Fix: clean up when the toolchain is bumped.

## Note

### Exhaustive public enums
`Event`, `Tag`, `TagEnd` and `LinkType` have grown with new features and are not `#[non_exhaustive]`. Consider adding it at the next breaking release.

### Lint configuration and bare TODOs
There is no lint baseline in `Cargo.toml`, and there are 11 TODO/FIXME comments without an owner.

## Checked with no defect found

- Parser and offset iterator produce identical events.
- Events stay balanced, and ranges are valid and on character boundaries.
- Each option is neutral on documents without its trigger syntax (500k inputs).
- `HTML_TAGS` matches the bundled spec.
- About 95 other crafted blowup inputs stayed linear.

## Verification

The plain workspace gate could not run here. `scripts/check.sh --audit` failed clippy, rustdoc and tests because the `dos-fuzzer` member's `mozjs` dependency needs `llvm-objdump`, which is missing. This is an environment limit, not a crate finding. So I ran the checks per package.

- `cargo fmt -p pulldown-cmark --check`: pass.
- `cargo test --locked -p pulldown-cmark`: pass (1148 suite tests, plus unit, doc and other tests).
- `cargo test --locked -p pulldown-cmark --all-features`: pass (rewrites `tests/suite/`, see Verdict).
- `RUSTDOCFLAGS="-D warnings" cargo doc -p pulldown-cmark --no-deps --document-private-items --all-features`: pass.
- `cargo clippy -p pulldown-cmark --all-targets --all-features -- -D warnings`: fail (toolchain drift, see Minor).
- `cargo check -p pulldown-cmark --no-default-features` and `--features simd`: pass.
- `cargo test -p pulldown-cmark --no-default-features --no-run`: pass.
- `cargo +nightly-2026-10-01 miri test -p pulldown-cmark --lib`: pass (59 tests). The default build forbids unsafe, so this adds little.
- x86_64 SIMD build: skipped, target not installed.
- Audit scan: run; its hits are reflected above.
- Fuzz and timing harnesses: in `tmp/probe/src/bin/` of the workspace (`f1`, `m1`, `m2`, `e1`, `b1`, `b2`, `x1`, `p1`-`p5`, `n1`, `n2`).

Number of findings: 53 (2 Blocker, 29 Major, 20 Minor, 2 Note).
Last `./clock` reading: 27 minutes used of 45.
