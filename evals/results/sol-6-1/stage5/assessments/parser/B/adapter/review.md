Candidate B adapter review

Reviewed /work/candidates/B/src/lib.rs independently.
The frozen scoring.md contains no category adapter runner or category contract.
The frozen 24 public API tests are run unchanged regardless of adapter availability.
The preserved seven category tests are supplemental checks of optional public APIs;
they do not add requirements to the case and are excluded from frozen hidden counts.

Public ParseError::kind() returns the public non_exhaustive ParseErrorKind.
Each mapping is an exact match on that typed API:
- MissingEquals: documented missing separator.
- InvalidKey: documented empty or invalid key alphabet.
- DuplicateKey: documented repeated key.
- UnterminatedQuote: documented missing closing quote.
- UnknownEscape: documented unsupported escape.
- TrailingEscape: documented final backslash without an escape character.
- TrailingCharacters: documented non-space/tab following the closing quote.
The adapter returns None for future unknown enum variants. The line mapping calls
public ParseError::line(), documented as one-based physical input line.
Classification accepts only an error: it cannot inspect call input or expected test
category, and does not read Display or Debug strings or private representation.
No submission files are injected or changed; the adapter is a separate consumer.
