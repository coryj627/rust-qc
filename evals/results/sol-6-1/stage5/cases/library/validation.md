# Preparation validation record — operator only

Prepared independently from /work/spec.md. No treatment/candidate run was made;
/work/crate was not created or evaluated. No evaluation outputs, answer keys,
skills, or other agents were consulted. All file/command work used sandbox_exec.
No tools/toolchains/dependencies were installed or fetched, and no explicit file
reads outside /work or network operations were performed. Rust/Cargo 1.99.1 and
Python 3.11.2 were used. The test runner forces offline Cargo and disables rustup
auto-installation. Executable build/harness outputs are under /work/tmp.

## Artifact integrity

`cmp /work/spec.md /work/prepared/case.md` succeeded. Both SHA-256 values:

```
6c04e442124177b27e0d345c9d54a0bb22598fab0da26ccd9a47fce3f0356c94
```

case.md contains the entire unchanged task and no preparation instructions,
hidden tests, rubric, reference, or negative controls. Only that file should be
given to the treatment. The remaining prepared artifacts and all of /work/reference
must remain outside the treatment case/view.

## Final executed checks

| Check | Actual result | Evidence |
| --- | --- | --- |
| Full hidden suite against independent reference | Exit 0; 11 passed, 0 failed (8 public API + 3 category tests) | logs/reference.log and .exit |
| Public suite without an error adapter | Exit 0; 8 passed, 0 failed; category checks explicitly not run | logs/reference-public-only.log and .exit |
| Wrong bounds/UTF-8 failure precedence control | Expected exit 101; 10 passed, 1 failed; named precedence assertion fails, public API suite passes | logs/negative-precedence.log and .exit |
| Inclusive-end containment control | Expected exit 101; 9 passed, 2 failed; ordinary and MAX containment probes fail | logs/negative-contains.log and .exit |
| Reference crate's own Cargo test/rustdoc invocation | Exit 0; 1 rustdoc example passed; 0 authored unit tests | logs/reference-own-tests.log and .exit |
| Exact task copy | cmp exit 0; identical SHA-256 | above |

The assertion failures in controls are confirmed product-behavior differences,
not dependency/build errors. The controls use the same unmodified hidden tests
and the same public error classifier as the reference. make_negative_controls.py
preserves the exact two isolated mutations reproducibly under /work/reference.

Final full-suite invocations used:

```
python3 /work/prepared/hidden/run.py --crate /work/reference --run-dir /work/tmp/reference-complete.PiyOJL --error-adapter /work/reference/error_adapter.rs
python3 /work/prepared/hidden/run.py --crate /work/reference/negative-precedence --run-dir /work/tmp/negative-precedence-final.d7FgKg --error-adapter /work/reference/error_adapter.rs
python3 /work/prepared/hidden/run.py --crate /work/reference/negative-contains --run-dir /work/tmp/negative-contains-final.Gqt9CT --error-adapter /work/reference/error_adapter.rs
python3 /work/prepared/hidden/run.py --crate /work/reference --run-dir /work/tmp/reference-public-only.ed8RBB
```

Each runner printed the exact cargo command into its log. The final runner uses
`cargo test --offline --no-fail-fast`, serial test execution, and a fresh target
directory in its fresh /work/tmp harness. Use the mktemp commands in scoring.md
for repeat runs because existing nonempty run directories are intentionally
rejected. The reference's separate doc check used:

```
RUSTUP_AUTO_INSTALL=0 CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/work/tmp/reference-own-target cargo test --offline --no-fail-fast --manifest-path /work/reference/Cargo.toml
```

A preparatory rustfmt call initially could not resolve the intentionally absent
candidate adapter module. Re-running formatting on error_contract.rs with
`--config skip_children=true` succeeded; the final integration builds include the
real adapter. That formatting setup issue was unrelated to a product failure.
No clippy check was needed or run. Candidate scoring commands and the virtual-
workspace fallback are documented for future use, not claimed as executed here.

## Limits and manual evidence

The behavior tests use only the specified public API plus required standard
traits. They do not prescribe error messages, enum names, payloads, Clone/Copy/Eq,
or internal representation. An error adapter is necessary to test category and
precedence across deliberately unconstrained error designs. A grader must review
its implementation; a missing/bad adapter is not evidence of a product defect.
The public error representation, all safe construction/mutation routes,
candidate-authored tests, docs, dependencies, and scope require source review.

Finite behavior samples do not prove all usize values or all strings. Small
intervals and short UTF-8 strings are exhaustively sampled within their bounded
ranges; usize::MAX probes cover extremes on the installed target. No other
architecture/toolchain was tested, no treatment performance is claimed, and the
two deliberately broken variants are sensitivity checks rather than proof of
complete fault detection. The 12-point rubric defines blocked-evidence reporting
and separate caller/dependency/scope harm scores for both blind graders.
