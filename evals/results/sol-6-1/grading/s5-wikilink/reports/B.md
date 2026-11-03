Fixed the panic by rejecting consumed wikilink openers and invalid body ranges before subtraction or slicing. Added regressions covering malformed delimiters, Unicode, reference images, and preservation of valid Markdown and wikilinks. Public APIs remain unchanged.

Passed:

- Requested debug and release test suites.
- Requested no-default-features check.
- Additional no-default-features and serde/SIMD tests.
- Formatting, package rustdoc, and targeted Miri regression.
- 2,474,869 generated inputs under two option configurations, without panics or invalid event ranges.

Full workspace QC still encounters pre-existing Clippy warnings and a `mozjs_sys` build failure.