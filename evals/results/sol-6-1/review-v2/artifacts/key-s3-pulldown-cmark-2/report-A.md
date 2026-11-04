The existing test suites pass, but ordinary regression probes found incorrect parsing, rendering, source ranges, CLI behavior, and one feature-specific build failure. Findings: **1 High, 12 Medium, 3 Low**. Behavioral findings below were reproduced locally; the SIMD-contract finding is from source inspection.

## High

### 1. Enabling `getopts` without `html` breaks the binary build

`pulldown-cmark/src/main.rs:25`; supporting configuration: `pulldown-cmark/Cargo.toml:27`.

Expected: an enabled binary has its required dependencies. Observed: `cargo check --locked -p pulldown-cmark --no-default-features --features getopts` fails with unresolved import `pulldown_cmark::html`. The binary requires only `getopts`, although it unconditionally imports the feature-gated renderer.

Consequence: this selectable package configuration cannot build. Require both features for the binary, or make the CLI feature enable HTML. Add this configuration to build checks.

## Medium

### 2. CLI input and output failures are mishandled

Locations: `pulldown-cmark/src/main.rs:172`, `:184`, `:219`.

Expected: unsuccessful reads or writes produce a diagnostic and unsuccessful exit status. Executed probes found:

- Invalid UTF-8 stdin: empty output, exit **0**.
- Output redirected to `/dev/full`: exit **0**.
- Missing input file: panic, exit **101**.

Consequence: pipelines can report success despite producing no usable output. Propagate I/O errors consistently, return a result from the rendering helper, and explicitly flush its `BufWriter`; drop-time flush errors otherwise remain invisible.

### 3. Processing multiple files repeats earlier content

`pulldown-cmark/src/main.rs:167`.

Expected: each file contributes its contents once. Observed: files containing `Alpha\n` and `Beta\n` produce:

```html
<p>Alpha</p>
<p>Alpha
Beta</p>
```

`read_to_string` appends to the reused buffer, which is rendered after every file.

Consequence: batch conversion duplicates content and can merge unrelated Markdown structures. Clear the buffer per file, or concatenate all inputs and render once. Add a two-file CLI regression.

### 4. CRLF becomes two spaces inside code spans

`pulldown-cmark/src/parse.rs:1340`.

Expected: CommonMark treats CRLF as one line ending, converted to one space inside a code span. Observed: input `` `a\r\nb` `` produces `<code>a  b</code>`; LF and standalone CR produce one space. The same problem occurs inside blockquotes.

Consequence: Windows-authored Markdown changes code text. Consume the complete line ending using `scan_eol`, then append one space. Extend the existing newline-in-code tests with CRLF.

### 5. A trailing tab prevents a code fence from closing

`pulldown-cmark/src/scanners.rs:528`.

Expected: CommonMark permits spaces or tabs after a closing fence. Observed: `"```\ncode\n```\t\nafter\n"` renders the closing fence and `after` inside the code block.

Consequence: an ordinary trailing tab changes the remainder of the document’s structure. Scan both spaces and tabs before checking the line ending; test both fence characters.

### 6. Case-insensitive footnote matches produce broken HTML links

Locations: `pulldown-cmark/src/html.rs:53`, `:139`; related reference emission: `pulldown-cmark/src/parse.rs:768`.

Expected: references accepted as matching the same definition share its destination and number. Observed:

```markdown
ref [^FOO] [^foo]

[^foo]: Note
```

produces references to `#FOO` numbered 1 and `#foo` numbered 2, while only `id="foo"` exists. Unicode case-folding variants also receive separate numbers and unmatched destinations.

Consequence: accepted footnote references fail to navigate and display inconsistent numbering. Resolve references to a canonical definition identifier and use the same case-folded identity for numbering. Add ASCII and Unicode case-variant tests.

### 7. Standalone CR line endings are inconsistently recognized

`pulldown-cmark/src/scanners.rs:508`.

Expected: CommonMark accepts LF, CRLF, and standalone CR. Observed: `scan_nextline` searches only for LF. With heading attributes enabled, `"# heading\r\rpara\r"` becomes one heading containing the following paragraph. Executed probes also found incorrect fenced-code, indented-code, and HTML-block behavior with CR-only input.

Consequence: valid documents can acquire substantially different block structure. Make line scanning recognize all documented endings and verify affected block consumers with equivalent fixtures.

### 8. Link-label length counts non-ASCII bytes and ignores ordinary ASCII

`pulldown-cmark/src/linklabel.rs:122`.

Expected: labels contain at most 999 Unicode characters. Observed boundary probes:

- 499 `é` characters resolve; 500 do not.
- 249 crab emoji resolve; 250 do not.
- 1,000 ASCII `a` characters still resolve.

The counter increments for every non-ASCII byte and does not increment for ordinary ASCII letters.

Consequence: valid internationalized reference labels are rejected, while over-limit ASCII labels are accepted. Count Unicode characters consistently, including escaped and whitespace portions. Add boundary tests for several UTF-8 widths.

### 9. Wikilink source ranges omit the final bracket

`pulldown-cmark/src/parse.rs:944`.

Expected: `OffsetIter` returns the corresponding complete source range. Observed: `[[Wiki]]` produces link start/end ranges `0..7`, although the construct occupies `0..8`. Embedded wikilink images have the same off-by-one error.

Consequence: editors and source transformations leave a bracket behind or select incomplete syntax. Use the end of the second closing delimiter and assert complete ranges for links and images.

### 10. Empty wikilink aliases render delimiter text

`pulldown-cmark/src/parse.rs:900`.

Expected: closing delimiters remain syntax when handling an empty display label. Observed: `[[Wiki|]]` renders:

```html
<p><a href="Wiki">]]</a></p>
```

The selected child starts at a closing-bracket node.

Consequence: boundary-case input generates visibly incorrect link text. Handle an empty alias explicitly—either according to a documented empty-label behavior or as literal Markdown—and keep delimiter nodes outside its children.

### 11. Single-character heading attributes are silently dropped

`pulldown-cmark/src/firstpass.rs:2548`.

Expected: documented custom attributes without values are retained regardless of name length. Observed: with heading attributes enabled, `# heading {x}` renders `<h1>heading</h1>`, losing `x`; longer attribute names work.

Consequence: callers lose attributes from both parsed events and rendered HTML. Apply the length restriction only to prefixed ID/class forms, and test one-character custom attributes.

### 12. Literal NUL is retained instead of replaced

`pulldown-cmark/src/firstpass.rs:2355`.

Expected: the bundled CommonMark contract requires U+0000 to become U+FFFD. Observed: ordinary input `"a\0b"` renders `"<p>a\0b</p>\n"`.

Consequence: output text contradicts the claimed CommonMark normalization. Normalize literal NUL while preserving source offsets, and add event and renderer regressions. This was checked solely as a correctness case; no security impact was investigated.

### 13. SIMD unchecked-access contracts are incomplete

Locations: `pulldown-cmark/src/firstpass.rs:2644`, `:2693`, `:2703`, `:2733`.

Source inspection found three unsafe functions without complete safety contracts and one explicit unsafe block without a SAFETY justification. In particular, `process_mask` does not document the relationship required between mask bits, offset, and `bytes.get_unchecked`.

Consequence: future scanner changes lack a written basis for preserving unchecked-access preconditions. Document the bounds, mask, and CPU-feature obligations and justify each unsafe call. No invalid access was demonstrated; this x86_64 implementation was not executed on the aarch64 host.

## Low

### 14. Tab-separated fence metadata becomes extra CSS classes

`pulldown-cmark/src/html.rs:261`.

Expected: the renderer’s established first-word behavior treats a tab like a space. Observed: a fence info string `rust\tcustom` produces `class="language-rust\tcustom"`; `rust custom` produces only `class="language-rust"`.

Consequence: metadata unexpectedly becomes additional class tokens. Split on the intended whitespace class and extend the existing info-string tests. This concerns renderer consistency; CommonMark leaves info-string rendering discretionary.

### 15. Public documentation has omissions and an incorrect punctuation example

Locations: `pulldown-cmark/src/parse.rs:181`, `pulldown-cmark/src/html.rs:590`, `:625`, `pulldown-cmark/src/lib.rs:674`.

The advisory pass reported **65 library missing-documentation warning sites** and two missing `# Errors` sections. The consequential omissions include offset units on `BrokenLink::span` and writer-error behavior.

Separately, smart-punctuation documentation promises that `--` becomes an em dash. Executed input `a -- ---` produces an en dash followed by an em dash, consistent with the existing implementation’s convention.

Clarify the API contracts and correct the example.

### 16. Strict Clippy fails on the supplied toolchain

Example locations: `pulldown-cmark/src/lib.rs:636`, `pulldown-cmark/src/utils.rs:5`, `pulldown-cmark/src/parse.rs:468`.

Expected: the requested strict lint gate completes. Observed on Rust **1.99.1**:

- All features: four build-script diagnostics stop compilation.
- `serde,simd` without `gen-tests`: 23 library diagnostics, including documentation-list formatting, unnecessary borrows, and simplifiable expressions.

Consequence: strict lint checks cannot currently pass. These are pre-existing baseline failures under the review configuration. Older toolchains were not run, so toolchain drift alone is not established.

## Verification and evidence

Commands ran offline with `RUSTUP_AUTO_INSTALL=0`, from `/work/crate` unless specified.

| Command | Outcome |
|---|---|
| `cargo test --locked -p pulldown-cmark` | Passed: 1,255 tests including doctests |
| `cargo test --locked -p pulldown-cmark --all-features` | Passed: 1,260 |
| `cargo test --locked -p pulldown-cmark --no-default-features` | Passed: 79 |
| `cargo fmt --all --check` | Passed |
| `cargo clippy --locked -p pulldown-cmark --all-targets --all-features -- -D warnings` | Failed: four build-script diagnostics |
| `cargo clippy --locked -p pulldown-cmark --all-targets --features serde,simd -- -D warnings` | Failed: 23 library diagnostics |
| `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p pulldown-cmark --no-deps --document-private-items --all-features` | Passed |
| `cargo check --locked -p pulldown-cmark --no-default-features --features getopts` | Failed: unresolved `html` import |
| `cargo +nightly-2026-10-01 miri test --locked -p pulldown-cmark --lib` | Passed: 59 unit tests |
| `cargo test --offline --manifest-path /work/tmp/review-harness/Cargo.toml` | Intentional regression failures: 11 failed, one passed |
| `python3 /work/tmp/cli-regressions.py` | Four expected-behavior checks failed |

The required `bash /work/[tooling]/scripts/[tooling] --audit /work/crate` passed formatting, but workspace rustdoc/tests were blocked by supporting `mozjs_sys` tooling: `llvm-objdump` was unavailable. Workspace Clippy also encountered the build-script failures above. These supporting-package/tool failures do not establish parser defects.

The advisory lint pass and heuristic scan completed. Style-only scan suggestions without a concrete consequence were excluded. Scratch Rust formatting, Clippy, and documentation checks passed. Reproductions and logs remain under `/work/tmp`, including [regression tests](/work/tmp/review-harness/tests/regressions.rs), [CLI probes](/work/tmp/cli-regressions.py), and [gate output](/work/tmp/gate.log).

Review-authored changes were confined to scratch files. All-features commands enabled `gen-tests`, which regenerated files under `pulldown-cmark/tests/suite`; production source was not edited.

**Final count: 16 findings—1 High, 12 Medium, 3 Low.** Coverage was focused rather than exhaustive. No dedicated security, fuzzing, or performance assessment was performed. MSRV, cross-target builds, and x86 SIMD execution were unrun; those toolchains/targets were unavailable. Feature-powerset tooling was unavailable. Last `/work/clock` reading: **509.1 seconds of 2,700**, at **2026-11-03 19:58:20 UTC**.