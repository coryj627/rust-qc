Candidate A

Frozen hidden public API: 24 passed, 0 failed.
Own tests: 14 passed, 0 failed.
Doctests: 1 passed, 0 failed.
Frozen library, format and Clippy checks: exit 0.
Reported strict Clippy and rustdoc checks: exit 0.
Separate archived gate: exit 0; format, Clippy, rustdoc and tests all pass.
Supplemental category adapter: unrun: no public typed category API.

Use frozen-isolated/ for definitive frozen logs. Initial frozen/ logs are
preserved; shared-target cache contamination caused wrong unit-test binaries
and B/D doctest compiler errors in initial runs. They are environmental
artifacts, not behavioral assertion failures. All checks were rerun unchanged
with empty build directories between candidates. The gate used a separate
candidate-specific target from the start.

No submission files changed. See submission-before.json, submission-after.json,
submission-changes.json, adapter/review.md, commands.jsonl and record.json.
No quality score or rubric points assigned.
