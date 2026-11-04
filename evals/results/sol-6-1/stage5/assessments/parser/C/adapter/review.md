Candidate C adapter review

Reviewed /work/candidates/C/src/lib.rs independently.
The frozen scoring.md contains no category adapter runner or category contract.
The frozen 24 public API tests are run unchanged regardless of adapter availability.
The preserved seven category tests are supplemental checks of optional public APIs;
they do not add requirements to the case and are excluded from frozen hidden counts.

Public ParseError::line() supplies one-based lines, but the reason/message
field is private and no public typed category accessor or downcastable source is
provided. The adapter records None for category. Reading Display/Debug text or
inferring category from the caller's input would violate the assessment rules.
All seven category adapter tests are deliberately UNRUN (not compile/test failures).
The missing optional category API is not a failure of the requested case API.
