The existing tests pass, but targeted checks confirmed **15 Major findings and four Minor findings**. Most concern parsing or rendering boundaries absent from the fixtures. This was a correctness and code-quality review; no dedicated security testing was performed.

Inputs below use Rust string escapes. Locations are under `/work/crate/pulldown-cmark/` unless stated otherwise.

## Major findings

### 1. Valid reference links silently stop resolving

Location: [src/parse.rs:325](/work/crate/pulldown-cmark/src/parse.rs:325), budget initialized at [src/parse.rs:288](/work/crate/pulldown-cmark/src/parse.rs:288).

Expected: Defined references continue resolving throughout a document.

Observed: A 2,607-byte document defining `[r]` as `https://example.com/` plus 180 `a` characters, followed by 600 `[r]` references, produces only **500 links**. The remaining references become literal text after the undocumented expansion budget reaches zero.

Consequence: Generated indexes and documents with repeated long URLs lose working links without a diagnostic.

Evidence: Executed public-API regression test. Fix: expose a configurable limit and an explicit exhaustion signal, rather than silently treating defined references as unresolved.

### 2. CLI repeats earlier files when processing multiple paths

Location: [src/main.rs:173](/work/crate/pulldown-cmark/src/main.rs:173).

Expected: Each supplied file contributes its content once.

Observed: Files containing `alpha\n` and `beta\n` produce:

```html
<p>alpha</p>
<p>alpha
beta</p>
```

`read_to_string` appends to the same buffer, which is rendered after every file.

Consequence: Multi-file conversions duplicate content and can change Markdown structure.

Evidence: Executed CLI reproduction. Fix: clear the buffer between independently rendered files, or concatenate all inputs and render once.

### 3. CLI reports success after failed input or output

Location: [src/main.rs:184](/work/crate/pulldown-cmark/src/main.rs:184), [src/main.rs:219](/work/crate/pulldown-cmark/src/main.rs:219).

Expected: Failed reads and writes produce a diagnostic and unsuccessful exit status.

Observed:

- Invalid UTF-8 on stdin produces empty output and exits **0**.
- Redirecting output to `/dev/full` exits **0** without a diagnostic.
- A missing input file panics and exits **101**.

Consequence: Scripts cannot reliably detect incomplete conversions.

Evidence: Executed CLI checks. Fix: propagate read/write errors, retain and explicitly flush the `BufWriter`, and replace file-operation `expect` calls with error propagation. Propagating only `write_html_io` is insufficient because the final buffered write can fail during drop.

### 4. HTML blocks ignore valid terminating tags

Location: [src/firstpass.rs:1250](/work/crate/pulldown-cmark/src/firstpass.rs:1250).

Expected: CommonMark 0.31.2 type-1 HTML blocks terminate on any of `</pre>`, `</script>`, `</style>`, or `</textarea>`, case-insensitively; the closing tag need not match the opener.

Observed: After either `</PRE>` or `</style>` following `<pre>`, subsequent `*after*` remains raw HTML-block content instead of becoming `<p><em>after</em></p>`.

Consequence: Valid uppercase HTML closers cause subsequent Markdown formatting to disappear.

Evidence: Two executed regression tests and the bundled specification. Fix: recognize the complete case-insensitive terminator set.

### 5. Tabs prevent code fences from closing

Location: [src/scanners.rs:528](/work/crate/pulldown-cmark/src/scanners.rs:528).

Expected: Spaces or tabs following a closing fence are ignored.

Observed: `"```\nx\n```\t\nafter\n"` renders the closing fence and `after` inside the code block.

Consequence: A trailing tab can absorb the remainder of a document into code.

Evidence: Executed regression test; the bundled CommonMark specification explicitly permits trailing tabs. Fix: accept both spaces and tabs after closing fences.

### 6. Trailing tabs incorrectly create hard line breaks

Location: [src/firstpass.rs:848](/work/crate/pulldown-cmark/src/firstpass.rs:848).

Expected: A hard break requires two trailing spaces or a backslash, as documented by `Event::HardBreak`.

Observed: `"one\t\t\ntwo"`, `"one \t\ntwo"`, and `"one\t \ntwo"` all produce `<br />`.

Consequence: Ordinary tab-containing text gains unintended visible line breaks.

Evidence: Executed regression and neighboring-case probes. Fix: distinguish trailing U+0020 spaces used for hard-break detection from whitespace removed during trimming.

### 7. CR-only line endings break block parsing

Location: [src/scanners.rs:508](/work/crate/pulldown-cmark/src/scanners.rs:508).

Expected: LF, CRLF, and standalone CR are valid CommonMark line endings.

Observed: `"```\ra\rb\r```"` becomes a paragraph containing the opening fence and text, followed by an empty code block. LF and CRLF controls render correctly.

Consequence: Documents using CR line endings receive substantially different block structure.

Evidence: Executed regression and line-ending controls. `scan_nextline` searches only for LF. Fix: recognize all three endings and normalize code text consistently.

### 8. Mixed-case footnote references produce broken anchors

Location: [src/parse.rs:768](/work/crate/pulldown-cmark/src/parse.rs:768), [src/html.rs:138](/work/crate/pulldown-cmark/src/html.rs:138).

Expected: References that resolve to the same case-folded footnote definition share its anchor and number.

Observed: `"a[^A] b[^a]\n\n[^A]: note\n"` produces references numbered **1** and **2**, targeting `#A` and `#a`, but only defines `id="A"`.

Consequence: One reference has a dangling link and inconsistent numbering.

Evidence: Executed regression; source inspection confirms definition lookup uses Unicode case folding while rendering uses case-sensitive strings. Fix: retain a canonical definition label for emitted references and rendering.

### 9. Empty wikilink aliases consume delimiters and following text

Location: [src/parse.rs:900](/work/crate/pulldown-cmark/src/parse.rs:900).

Expected: A recognized piped link’s label contains only content between `|` and `]]`.

Observed: `"[[Page|]] after"` renders:

```html
<p><a href="Page">]] after</a> after</p>
```

The corresponding image syntax also includes `]] after` in its alt text.

Consequence: Empty aliases corrupt labels and duplicate following content.

Evidence: Executed standalone, trailing-text, and image probes. Fix: handle an empty alias explicitly without making closing delimiters part of the child chain.

### 10. Wikilink source ranges omit the final bracket

Location: [src/parse.rs:944](/work/crate/pulldown-cmark/src/parse.rs:944).

Expected: `into_offset_iter()` returns the source range corresponding to the complete link or image.

Observed: `[[Page]]` yields **0..7**, although its complete range is **0..8**. A piped wiki image also omits its final bracket.

Consequence: Editors and source transformations using these ranges leave a stray `]` behind.

Evidence: Executed offset regression and image probe. Fix: derive the end from the second closing bracket’s end position.

### 11. Superscript and subscript interfere through shared matching state

Location: [src/parse.rs:1596](/work/crate/pulldown-cmark/src/parse.rs:1596), [src/parse.rs:1613](/work/crate/pulldown-cmark/src/parse.rs:1613).

Expected: An unmatched delimiter of another kind does not suppress an otherwise valid match.

Observed:

- `~a^ b~` renders as subscript with subscript enabled alone, but becomes literal text when superscript is also enabled.
- `^a~ b^` fails symmetrically when subscript is additionally enabled.

Consequence: Enabling another extension unexpectedly removes existing formatting.

Evidence: Two executed regression tests with single-extension controls. Both delimiter kinds share the `TILDES` lower-bound slot. Fix: track their matching bounds independently.

### 12. Link-label length counts the wrong bytes

Location: [src/linklabel.rs:122](/work/crate/pulldown-cmark/src/linklabel.rs:122).

Expected: The CommonMark limit is **999 Unicode code points**.

Observed:

- A valid label containing 500 `é` characters is rejected.
- A label containing 1,000 ASCII `a` characters is accepted.

ASCII characters do not increment the counter; every non-ASCII byte does.

Consequence: Valid internationalized labels fail to resolve, while over-limit ASCII labels are parsed incorrectly.

Evidence: Executed Unicode and ASCII boundary tests. Fix: count ASCII bytes and UTF-8 leading bytes once per code point.

### 13. Inline CDATA stops searching at an isolated bracket

Location: [src/scanners.rs:1477](/work/crate/pulldown-cmark/src/scanners.rs:1477).

Expected: CDATA content can contain isolated `]` characters; only `]]>` terminates it.

Observed: `"foo <![CDATA[a]b]]> bar"` is escaped as text instead of emitting the documented inline HTML construct.

Consequence: Valid embedded CDATA is altered.

Evidence: Executed regression and bundled CommonMark CDATA definition. Fix: continue searching after unsuccessful closing-bracket candidates.

### 14. NUL characters are not replaced as CommonMark requires

Location: [src/parse.rs:2256](/work/crate/pulldown-cmark/src/parse.rs:2256).

Expected: U+0000 becomes U+FFFD.

Observed: `"a\0b"` retains NUL in both text events and rendered HTML. A fenced-code probe also preserves it.

Consequence: Callers receive text that contradicts the CommonMark preprocessing contract.

Evidence: Executed paragraph regression and code-block probe. Fix: perform replacement consistently across parsing paths while preserving source offsets. No security impact was investigated.

### 15. Enabling `getopts` without `html` fails to build

Location: [src/main.rs:25](/work/crate/pulldown-cmark/src/main.rs:25); supporting configuration at [Cargo.toml:27](/work/crate/pulldown-cmark/Cargo.toml:27).

Expected: Feature selection does not enable a binary whose required module is disabled.

Observed:

```sh
cargo check --locked -p pulldown-cmark --no-default-features --features getopts
```

fails with **E0432**, unresolved import `pulldown_cmark::html`.

Consequence: This optional-feature configuration cannot build the package.

Evidence: Executed build check on Rust 1.99.1. Fix: require both `getopts` and `html` for the binary, or make the feature dependency explicit.

## Minor findings

### 16. Conversion errors do not implement standard error traits

Location: [src/lib.rs:457](/work/crate/pulldown-cmark/src/lib.rs:457), [src/strings.rs:12](/work/crate/pulldown-cmark/src/strings.rs:12).

Expected: Public conversion errors integrate with ordinary Rust error propagation.

Observed: Neither `InvalidHeadingLevel` nor `StringTooLongError` implements `Display` or `std::error::Error`. Using either conversion with `?` in a function returning `Box<dyn Error>` fails with E0277.

Consequence: Callers need manual error translation.

Evidence: Executed downstream compile probe. Fix: implement the standard traits and publicly re-export `StringTooLongError`.

### 17. Public documentation has omissions and an incorrect replacement example

Location: [src/parse.rs:180](/work/crate/pulldown-cmark/src/parse.rs:180), [src/html.rs:590](/work/crate/pulldown-cmark/src/html.rs:590), [src/lib.rs:674](/work/crate/pulldown-cmark/src/lib.rs:674).

The advisory pass produced **65 missing-documentation diagnostics in library source**, including a macro diagnostic covering multiple constants, plus **two missing `# Errors` sections**. `BrokenLink` fields lack documentation, including the meaning and units of `span`. Smart-punctuation documentation incorrectly says `--` becomes an em dash; implementation and existing fixtures use an en dash.

Fix: document public contracts, error propagation, and offset units, and correct the example. These are advisory findings; strict rustdoc itself passed.

### 18. Strict Clippy checks fail on the supplied toolchain

Location: [src/lib.rs:636](/work/crate/pulldown-cmark/src/lib.rs:636), [src/utils.rs:5](/work/crate/pulldown-cmark/src/utils.rs:5), [src/scanners.rs:1028](/work/crate/pulldown-cmark/src/scanners.rs:1028).

Expected: The requested strict lint gate completes successfully.

Observed: Package Clippy reports **23 errors**, including documentation indentation, an empty line after an attribute, complex types, and simplifiable expressions.

Consequence: Consumers or maintainers enforcing `-D warnings` cannot pass this gate.

Evidence: Executed baseline check with Clippy 0.1.99. Fix using MSRV-compatible changes. Rust 1.71.1 was unavailable, so these results do not establish failure on the declared MSRV.

### 19. SIMD unsafe boundaries lack complete invariant documentation

Location: [src/firstpass.rs:2693](/work/crate/pulldown-cmark/src/firstpass.rs:2693), [src/firstpass.rs:2703](/work/crate/pulldown-cmark/src/firstpass.rs:2703).

Source inspection found **one unsafe block without a SAFETY comment** and **three unsafe functions without `# Safety` sections**. Some bounds are described informally, but CPU-feature, mask, and indexing requirements are not collected at the relevant boundaries.

Consequence: Future scanner changes require reconstructing these obligations.

Fix: document the preconditions and why each call satisfies them. No runtime defect is asserted; x86 SIMD was not executed on this aarch64 host.

## Verification and limitations

Commands ran with `RUSTUP_AUTO_INSTALL=0`; subsequent Cargo checks used offline mode.

From `/work/crate`:

- `cargo test --locked -p pulldown-cmark` — **passed**, 1,255 tests including doctests.
- `cargo test --locked -p pulldown-cmark --all-features` — **passed**, 1,260 tests.
- `cargo test --locked -p pulldown-cmark --no-default-features` — **passed**, 79 tests.
- `cargo fmt --all --check` — **passed**.
- `cargo clippy --locked -p pulldown-cmark --all-targets --features html,serde,simd -- -D warnings` — **failed**, 23 library errors.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-deps --document-private-items --all-features` — **passed**.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-default-features --no-deps --document-private-items` — **passed**.
- `cargo check --locked -p pulldown-cmark --no-default-features --features html` — **passed**.
- `cargo check --locked -p pulldown-cmark --no-default-features --features serde,simd` — **passed**.
- `cargo check --locked -p pulldown-cmark --no-default-features --features getopts` — **failed**, E0432.
- `cargo +nightly-2026-10-01 miri test --locked -p pulldown-cmark --lib` — **passed**, 59 tests.

The requested `bash /work/skill/scripts/check.sh --audit /work/crate` ran. Workspace formatting passed; Clippy stopped on four supporting `build.rs` lint errors. Workspace documentation, tests, and advisory completion encountered a supporting `mozjs_sys` build blocked by missing `llvm-objdump`. That environment block is not counted as a crate defect. Package-only checks above establish the narrower results.

Additional executed checks:

- `/work/tmp/review`: `cargo test --test commonmark_default` — **652 bundled fixtures passed using `Parser::new`**.
- `/work/tmp/review`: `cargo test --test invariants -- --nocapture` — **270 ordinary syntax/block/option combinations passed** event-balance, range-validity, and renderer-consistency checks.
- `/work/tmp/review`: `cargo test --test contracts` — **15 assertions failed and two passed**, covering 12 distinct library defects and successful library I/O-error propagation.
- `python3 /work/tmp/cli_checks.py` — reproduced the CLI discrepancies.
- `/work/tmp/api-probe`: `cargo check` — reproduced both error-trait failures.
- `python3 -I /work/skill/scripts/audit_scan.py /work/crate` — completed; generic glob/TODO suggestions were not promoted without concrete impact.

No production source was edited. Harnesses were added under `/work/tmp`; enabling `gen-tests` regenerated `tests/suite/` through the existing build script. No dependencies or tools were installed. MSRV, cross-target execution, x86 SIMD execution, and exhaustive feature-powerset checks were unrun.

**Finding count: 19 — 15 Major, four Minor.** Coverage used source inspection, existing tests, and ordinary local regressions; dedicated security and stress testing remained outside scope. Last clock reading: **851.8 seconds active, 14.1935 of 45 minutes**.