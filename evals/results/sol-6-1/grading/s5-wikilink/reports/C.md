Fixed the panic by rejecting wikilink openers already consumed as Markdown images, preventing overflow and invalid slicing without changing public APIs.

Added regressions for malformed delimiters, reference images, Unicode, following Markdown, and valid wikilink recovery.

All checks passed:

- Requested debug and release test suites
- No-default-features check and error tests
- Formatting check
- 100,000-input malformed-link probe