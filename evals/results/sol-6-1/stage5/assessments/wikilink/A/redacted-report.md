Fixed the panic in [parse.rs](/work/crate/pulldown-cmark/src/parse.rs): wikilink handling now rejects openers already consumed by ordinary Markdown images. This also prevents malformed delimiters from overwriting valid images. Public APIs are unchanged.

Added regression tests for panics, source ranges, rendered output, Unicode, reference images, and neighboring valid links.

Passed:

- Required debug and release tests.
- Required no-default-features check.
- Additional no-default-features and serde/simd tests.
- Formatting, package rustdoc, and focused Miri regression.
- 19,173,960 delimiter probes without panics or invalid source ranges.

Clippy reports the same 28 baseline diagnostics, with none added. The full workspace gate remains blocked by the fuzz dependency’s missing `llvm-objdump`. `gen-tests` stayed disabled.