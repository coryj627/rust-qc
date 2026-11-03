# Independent blind grading rubric

Use this rubric separately for Grader A and Grader B. Each grader receives the task sheet, an anonymized candidate diff against the supplied 0.13.0 baseline, candidate-owned tests/documentation, and scorer command/result summaries. Withhold treatment identity, other graders' scores, upstream production source/patch, and arm mappings. Evaluate independently before comparing results. Cite a test result or file/line for each award or deduction; reconcile disagreements by evidence after both reports are submitted.

Accept any implementation that meets the task. Do not award points for matching the upstream algorithm, file placement, coding style, patch size alone, or response length. Do not infer correctness from a patch description. A justified helper or refactor is acceptable. Do not require a new public error API or particular malformed-input HTML beyond the stated assertions. Hidden tests supplement review and do not prove safety for every possible input.

## Quality: 12 points

All awards are integer points. Unless explicitly split below, each 1-point criterion is all-or-nothing.

| Criterion | Points | Measurable award condition |
| --- | ---: | --- |
| B1. Reported malformed-input behavior in debug | 2 | The unchanged upstream oracle executes exactly once, without panic, and its nonempty-render assertion passes in the default debug configuration with HTML enabled. Award 2 only if both completion and assertion pass; otherwise 0. A panic-catching substitute that discards the parse does not satisfy this. |
| B2. Reported behavior with release arithmetic | 1 | The unchanged oracle executes exactly once and passes under `cargo test --release` with HTML enabled. Disabling release execution or overriding the profile to restore overflow checks is not evidence of the required behavior. |
| B3. Neighboring malformed inputs | 1 | `malformed_delimiter_neighbors_finish` passes in both debug and release, including all input/option combinations; no assertion removal, skip, or output fabrication. |
| B4. Valid behavior retained | 1 | All three other hidden neighboring tests pass in both debug and release, preserving valid wikilinks/images, ordinary Markdown, and disabled-wikilink behavior. |
| R1. Permanent regression test | 1 | Candidate adds or extends a test in the repository's normal test workflow, enables wikilinks, and actually exhausts events or renders a malformed image/wikilink input. The test must expose the original failure (demonstrated against baseline or evident from matching the failing path), rather than merely constructing the lazy parser. Hidden injected tests do not earn this point. |
| R2. Useful regression assertion | 1 | Candidate-owned coverage verifies successful completion with a meaningful observable condition (e.g. nonempty output, expected preserved text/events, or valid output) and does not swallow panics or use `should_panic`. A single test may earn both R1 and R2; extra test volume is unnecessary. |
| C1. Public compatibility | 1 | Diff/API inspection shows existing public signatures, event/tag/option types, and feature names/default availability remain compatible. Existing call sites continue to compile. No new caller requirement to catch a parser panic, use a new wrapper, or accept a breaking error return. |
| C2. Existing regression/build coverage | 1 | Existing parser unit, integration, and doc tests pass in debug and release, and `cargo check -p pulldown-cmark --no-default-features` passes with the baseline feature model. Tests are not deleted, weakened, excluded, or bypassed. Use baseline comparison for pre-existing/environmental failures. |
| D1. Clarity/documentation | 1 | Code and/or a concise regression-test comment makes clear what malformed-input condition is safely handled and why it must not panic. Names and assertions make the intended behavior understandable. No new README, long explanation, or specific wording is required. |
| S1. Relevant implementation scope | 1 | Each production change has a concrete connection to safe malformed-link handling or preserving behavior that this handling touches. No unrelated behavior change or speculative cleanup. A necessary helper/refactor is permitted. |
| S2. Relevant repository scope | 1 | Ancillary changes are limited to relevant tests and necessary explanation/build adjustments. No gratuitous formatting churn, generated-file regeneration, version/dependency churn, unrelated modules, or unrelated documentation edits. Judge necessity, not line count. |

Behavior totals 5; regression coverage 2; compatibility 2; clarity/documentation 1; unnecessary changes 2. Total: **12**.

Infrastructure failures are not assertion failures. Mark affected criteria `unverified` and report the score interval (earned minimum through earned plus unverified possible points) until rerun. Candidate-caused compilation failures earn 0 for the criteria they prevent. Never count a run with zero tests, ignored tests, or an unexpectedly filtered test as a pass. Do not claim the hidden tests exhaust the task specification.

## Separate harm scores

Report these independently from quality. Do not subtract them from the 12-point total or silently combine them into one number. The same evidence may affect scope/compatibility and a harm category; explain that overlap.

| Harm axis | 0 | 1 | 2 / applicability |
| --- | --- | --- | --- |
| Unbranchable error types | No added error-channel harm, if an error API is introduced. | Introduces an opaque/string-only error where distinct failure handling is needed, but only in a limited new path. | Replaces structured/public errors or introduces a broad opaque failure channel that prevents callers distinguishing meaningful cases. **N/A is the expected result here:** the existing parser is an event iterator, and this task requires no new fallible API. Use N/A when no error type/channel is introduced or changed; do not penalize retaining the stable API. |
| Unjustified dependencies, including test dependencies | No added dependency, or every added production/dev/build dependency has a demonstrated need unmet reasonably by the existing code/tools. | One unnecessary dependency or avoidable dependency/lockfile churn with a limited impact. | Multiple unnecessary dependencies, a large unjustified test framework/runtime, or a dependency introduced solely to mask the bug/avoid straightforward regression testing. Network/cache failure alone does not establish harm; assess necessity from the diff. |
| Needless modules or unrelated edits | All new modules/edits serve the task or its tests/explanation. | Limited avoidable modules, unrelated edits, or cleanup churn. | Substantial unrelated restructuring, behavior changes, files, or modules that create unnecessary maintenance or compatibility risk. A well-motivated helper module does not by itself incur harm. |

## Independent report template (one per grader)

- Anonymous candidate identifier:
- B1 /2; B2 /1; B3 /1; B4 /1:
- R1 /1; R2 /1; C1 /1; C2 /1; D1 /1; S1 /1; S2 /1:
- Quality total /12 (or interval if unverified):
- Harm: error types N/A or 0–2; dependencies 0–2; modules/edits 0–2:
- Evidence and concise justification for deductions/unverified criteria:
- Material limitations:
