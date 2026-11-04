# Publication review triage

This checks the ten open Codoki inline threads carried into its summary on
evaluation data commit `dd0a38bca84c2c2f352a6ecbbac2cddeeb06a830`. The summary
reports seven high-priority findings; a successful GitHub check does not mean
the bot approved the PR. The threads remain available for independent review.

These are operator checks after the study, not additional evaluation agents,
grades or candidate Rust executions. The frozen reports, input commitments,
numerical outputs, skill and original sandbox are unchanged. The local
[probe record](publication-review-probes.json) includes the observed runtime
and results; it contains no host workspace paths.

| Inline finding | Current evidence and disposition |
|---|---|
| [Missing YAML import](https://github.com/coryj627/rust-qc/pull/24#discussion_r4460204207) | `test_key_append.py` imports `yaml`. Both key-append tests passed again on the published code. No missing-import failure is present. |
| [Wrong timeout exception](https://github.com/coryj627/rust-qc/pull/24#discussion_r4458943858) | In the observed Python 3.15.2 runtime, `asyncio.TimeoutError is TimeoutError`; a real `asyncio.wait_for` timeout was caught by the existing built-in exception handler. This claim does not apply to this run. Older Python runtimes are not newly certified. |
| [Undefined functional-review argument](https://github.com/coryj627/rust-qc/pull/24#discussion_r4459933620) | `prepare_grading.py` declares `--functional-review`, which becomes `args.functional_review`. Its `--help` check passed. `preserve_verification` is the called function's keyword, not the CLI flag name. |
| [Assessment copy dereferences links](https://github.com/coryj627/rust-qc/pull/24#discussion_r4458522679) | On the actual macOS host, `/bin/cp -cR` preserved both file and directory symlinks in a small temporary copy probe. The candidate root is also explicitly rejected when it is a symlink. This refutes the claimed automatic in-tree dereference; it does not turn preserved links into a general traversal permission. Existing sandbox and source/export checks still apply. |
| [Fixed-validation copy dereferences links](https://github.com/coryj627/rust-qc/pull/24#discussion_r4458522866) | The same actual-host copy probe refutes the stated `cp -R` behavior. These native preparation helpers remain macOS-specific; no portable replacement is claimed. |
| [Nonportable copy flag](https://github.com/coryj627/rust-qc/pull/24#discussion_r4462319719) | Valid portability limitation. The recorded operator host is macOS arm64, where `-cR` succeeds. The protocol now explicitly states that the preparation scripts require macOS copy semantics and that CI does not certify Linux preparation. Porting the full preparation workflow requires a separate validated change. |
| [Source-excerpt index may be out of range](https://github.com/coryj627/rust-qc/pull/24#discussion_r4458523088) | `controls/just/evidence/record_metadata.py` is retained author evidence for fixed inputs, not a general metadata API. All 41 cited references are in bounds in the pinned prepared source. A changed or malformed reference can fail; the script is preserved byte-for-byte rather than silently changing the experiment's executable evidence. |
| [Scan artifact exposes a personal name](https://github.com/coryj627/rust-qc/pull/24#discussion_r4460809368) | The cited JSON contains only `file`, `kind` and `line` for each hit, not a matched name or secret. `personal_name` is the scanner's category label. The historical review records zero unreviewed hits. Renaming a category to suggest redaction would misdescribe the evidence. |
| [Missing export scans yield `ValueError`](https://github.com/coryj627/rust-qc/pull/24#discussion_r4458557394) | Valid diagnostic limitation on an unmet preparation prerequisite. The historical recommendation helper requires an existing completed publication scan; 33 matching scan files are present. It completed with those inputs. This does not invalidate the run, and no claim is made that the helper gives polished errors for an empty dataset. |
| [Missing positional argument yields `IndexError`](https://github.com/coryj627/rust-qc/pull/24#discussion_r4462319827) | Valid CLI diagnostic limitation. `prepare_audits.py` requires its operator scratch argument; the recorded invocation supplied it. Improving errors for unsupported invocations is future harness work, not a reason to mutate frozen schedules or rerun treatment. |

The unresolved empirical study limitations remain the missing capacity attempt,
small project samples, same-family grading, imperfect blinding and sampled
judgment disagreements. This triage does not remove or weaken those disclosures.
