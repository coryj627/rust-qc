Created under `/work/prepared`:

- [case.md](/work/prepared/case.md): standalone treatment prompt.
- [rubric.md](/work/prepared/rubric.md): 12-point rubric and separate harm checklist.
- [Hidden integration tests](/work/prepared/hidden/decom_case_hidden.rs): 23 behavioral tests.
- [Assessor runner](/work/prepared/hidden/run.sh).
- [scoring.md](/work/prepared/scoring.md): exact execution commands and isolation instructions.
- [Validation evidence](/work/prepared/evidence/validation.md), with logs, pristine source archive, and integrity manifests.

Hidden tests compile in both feature configurations; each baseline run has **10 passes and 13 expected failures**. Existing tests pass: **128 unit, 44 reference, and 1 doctest**; without default features, **127 unit tests pass**.

All 324 original file hashes remain unchanged. No fix was implemented.

Remaining limitation: no repaired implementation was validated. The assessor bundle must remain isolated from treatment agents.