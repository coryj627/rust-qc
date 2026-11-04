# Independent grading protocol

Give this same rubric separately to Grader A and Grader B. Each grades independently before seeing the other's scores. Give them only the task, baseline, anonymous candidate diff/source, candidate-added tests/report, and evaluator test results. Withhold treatment identity, arm labels, other graders' scores, and the upstream production patch. Do not infer quality from writing length, patch size alone, an implementation strategy, or similarity to upstream. Different correct algorithms receive the same behavioral credit.

Record one evidence citation per awarded or denied point: file/line, assertion, command result, or concrete diff observation. Score each item below as 0 or 1; do not use fractional points. An infrastructure failure is not an assertion failure: record affected items as unverified, explain why, and give a provisional score interval instead of inventing a pass/fail. Finalize after available source review or rerun resolves the uncertainty. Report both independent score sheets; reconcile disagreements by evidence without averaging away a behavioral failure.

# Quality: 12 points

| Item | Points | Measurable award rule |
| --- | ---: | --- |
| B1: retained lookup | 1 | The preserved upstream regression's retained-entry `get` assertions pass, with real entries and values rather than special casing the example. |
| B2: membership | 1 | After truncating colliding maps, `contains_key` is true for every retained key and false for every removed key in the specification matrix. |
| B3: collision generality | 1 | All hidden neighboring tests pass for alternate insertion orders, partially/full populated maps, and the custom colliding hasher. Source review shows no example-key/capacity-specific workaround. |
| B4: zero length | 1 | Truncating populated and empty maps to zero leaves length zero, empty iteration, and no old keys reachable. |
| B5: equal/larger lengths | 1 | Equal, larger, and `usize::MAX` lengths leave all existing entries, order, values, and lookup results unchanged, including an empty starting map. |
| B6: order and length | 1 | Intermediate truncation keeps exactly the original first `min(old_len, requested_len)` entries in their prior relative order; `len` and iterator count agree. |
| B7: values and removal | 1 | Retained values are unchanged, including owned/non-Copy values; every discarded key is absent from lookup and iteration. |
| R1: candidate regression | 1 | The candidate adds at least one executable assertion-based regression that fails for this bug on baseline production source and passes with its fix. It must check retained lookup after colliding entries are truncated; merely checking length or iteration does not qualify. Equivalent tests are welcome; duplication of evaluator tests is not required. |
| R2: candidate boundary coverage | 1 | Candidate-added tests additionally assert zero-length behavior and no-op behavior at an equal or larger length, plus retained order or value consistency. Can be in one table-driven test. Existing untouched tests and evaluator-only tests do not earn this point. |
| C1: compatibility | 1 | Public API/signatures and existing generic bounds remain compatible; non-Clone keys/values and a custom hasher compile, the default `no_std` library check succeeds, and relevant existing tests have no new failures. Do not require a specific internal algorithm or new toolchain. |
| D1: clarity/documentation | 1 | Changes are understandable from clear code/test names and a concise comment or report explaining the observable failure and verification. Existing truncate documentation remains accurate; explain any non-obvious new invariant if introduced. No prose quota, new docs file, or particular comment style is required. |
| S1: justified scope | 1 | All changed files/hunks serve the truncate fix, its tests, or necessary documentation/build support. No unrelated refactors, public APIs, dependencies, feature changes, or modules appear without a concrete necessity. A justified larger solution can earn the point. |

Maximum: 12. Do not impose a preferred patch, textual match, line limit, or exact algorithmic layout. Existing documentation's complexity promise and allocation-free/`no_std` contract remain relevant compatibility evidence; benchmarking or special performance tests are not required. Hidden test success is behavioral evidence, not a substitute for candidate regression coverage or source review.

# Harm: report separately from quality

Record each dimension independently with concrete newly introduced harm. Do not count baseline issues, retained dependencies, or necessary scaffolding as harm. Do not subtract the harm total from the 12-point quality score. A scope/API problem may be evidence for both a quality criterion and a harm dimension; state that overlap.

| Dimension | Score and rule |
| --- | --- |
| Unbranchable error types | **N/A for the normal task:** `truncate` returns `()` and needs no new error type. If the candidate introduces/changes an error surface, mark applicable: 0 = errors remain structurally distinguishable; 1 = an unnecessary new error surface erases distinctions into text/opaque values; 2 = an existing distinguishable public error is replaced so callers must parse messages or cannot branch reliably. Separately assess API compatibility. Do not demand new errors. |
| Unjustified dependencies, including tests | 0 = none, or each addition is justified by a concrete task need not reasonably served by existing facilities; 1 = unnecessary test-only/dev dependency; 2 = unnecessary runtime/build dependency, forced optional feature, or multiple unnecessary additions. A justified dependency may earn 0 even if upstream used none. Offline inability to fetch a candidate-added dependency is recorded separately as an execution limitation, not automatically proof it is unjustified. |
| Needless modules or unrelated edits | 0 = changes serve the task; 1 = isolated needless module or small unrelated edits (including avoidable broad formatting); 2 = substantial unrelated production changes, multiple needless modules, or unrelated deletion/reorganization. Do not penalize an internal helper/module required by a sound implementation. |

Report quality as `Q/12`, then each harm score/N/A and a sum over applicable dimensions, explicitly stating which dimensions apply. Do not treat N/A as a quality penalty.

# Grader response form

- Anonymous candidate identifier; grader A or B.
- B1–B7, R1–R2, C1, D1, S1: score and evidence for each.
- Quality total (or provisional interval and unresolved evidence).
- Harm: error types, dependencies, unrelated edits; evidence and applicability.
- Any observed behavior outside tested examples, and material validation limits. No speculative penalties.
