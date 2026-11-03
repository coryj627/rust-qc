Fixed [wikilink parsing](/work/crate/pulldown-cmark/src/parse.rs:879) to reject consumed image openers and empty or reversed body ranges before subtraction or slicing. Public APIs remain unchanged.

Added [regression tests](/work/crate/pulldown-cmark/tests/wikilinks.rs) covering malformed images, Unicode, offsets, ordinary Markdown, and subsequent valid wikilinks. Both tests reproduce the overflow on the original parser.

All requested checks passed:

- `cargo test --offline --locked -p pulldown-cmark`
- `cargo test --offline --locked -p pulldown-cmark --release`
- `cargo check --offline --locked -p pulldown-cmark --no-default-features`

The parser regression also passes without default features, and changed files pass rustfmt checks.