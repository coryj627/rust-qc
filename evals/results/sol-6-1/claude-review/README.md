# Claude review of the Sol 6.1 evaluation

Issue 23 asked Claude to review Sol's evidence and grading from a second model family. This review ran on Opus 5.5 after PR 24 merged, at commit `1381aa3`.

**The main result holds: Sol 6.1 shows no reliable gain in finding known bugs with the skill.** The numbers, cited evidence and privacy checks held up. Two limits needed stating more plainly, and the review found a scoring issue that affects how to read the recall numbers. No score, report or data file changed. No person who writes Rust has reviewed this.

## What held up

| Check | Result |
|---|---:|
| Recall recomputed from `review-v2/key_scores.json` | 34/80 vs 32/80 (p=0.8125); 10/36 vs 13/36 (p=0.5) |
| Claude's totals rerun with `evals/results/score.py` | 131 vs 135, unchanged |
| Skill runs whose logs reference `SKILL.md` and both scripts | 26 of 26 |
| Baseline runs whose logs mention `rust-qc` or its scripts | 0 of 26 |
| Quotes in `review-v2/recommended-skill-fixes.md` found in the linked evidence | 14 of 14 |
| Proposed patches that apply to `v1.0.0` and to `main` | 2 of 2 |
| Blind regrade agreeing with the primary score | 62 of 62 |

One baseline run, `v2-s3-pulldown-cmark-811a0b06`, ran a `find` for files named `SKILL.md` as part of its own review. None of its logs mention `rust-qc` or the skill's scripts.

**Privacy.** A search of the merged Sol tree found no email address, home path or token. The matches were the export scanner's own patterns, its test fixtures and upstream test names. One `git2` log contains an RSA private key that appears to be an upstream test fixture, so a secret scanner may flag it.

**Probes.** Two cited probes were rerun through `evals/sandbox/sb` from fresh clones of the release tags:

- **moka, `s3-moka-U001`:** `async_compute_serialization` printed `compute values: 1,2,2` and failed its assertion, matching the recorded log.
- **just, `s3-just-U002`:** all 51 cases in `verify.py` gave the same exit codes as the recorded log, including exit 101 for the `datetime`, `datetime_utc` and `--timestamp-format` panics.

## The blind regrade

Two fresh Opus 5.5 agents scored 62 pairs using [this brief](regrade-brief.md). The sample used a fixed seed and was stratified: 20 `found` and 10 `missed` scores from each arm, plus both `partial` scores among the reproduced entries. Reports were given random names, but they were unredacted, so style could hint at the arm. The results are in [`regrade.jsonl`](regrade.jsonl).

Agreement was complete, but **it shows stability, not strictness**. The brief told graders that a symptom alone is `partial` when a rule requires a mechanism. They still gave `found` for `just-3` on report `v2-s3-just-fc32aea1`, which describes the panic and never names its cause.

## Symptom credit on cause rules

The `just-3` and `just-10` match rules require a panic "caused by format(...).to_string() on a fallible chrono formatter". Primary graders gave `found` on both rules for all eight `just` reports, but those reports differ. This table is one reviewer's reading of each finding, done knowing the arms:

| Report | Arm | Stage | How it explains the panic |
|---|---|---:|---|
| `v2-s3-just-35a02b05` | baseline | detailed | Names `to_string()` |
| `v2-s3-just-6836e627` | baseline | detailed | Symptom only |
| `v2-s3-just-ff5edbd3` | with skill | detailed | "Unchecked formatting error"; "fallible formatting" appears only in the fix |
| `v2-s3-just-fc32aea1` | with skill | detailed | Symptom only; the fix says to "handle formatting errors explicitly" |
| `v2-s4-just-ad428667` | baseline | short | Names `to_string()` |
| `v2-s4-just-9ccfdd80` | baseline | short | Names `.to_string()` |
| `v2-s4-just-6ea76a32` | with skill | short | "Fallible formatting" appears only in the fix |
| `v2-s4-just-877cc60d` | with skill | short | Names the cause: "panic while converting Chrono's formatted value to a string" |

Requiring the named cause on those two rules for all eight reports, and changing nothing else, gives:

- **Detailed:** 30/80 vs 30/80, down from 34 vs 32.
- **Short:** 8/36 vs 13/36, down from 10 vs 13.

This is one project and two rules, so it is not an estimate of anything broader. Other rules were not reread this way. It does support three points:

- The recheck's sampled changes are not obviously a sampling accident. An earlier draft of this review argued that they were, and reading these reports did not support that.
- Primary recall credits observed failures even where a rule asks for the cause. Decide before any future study whether the scoring unit is the failure or the cause.
- Skill reports named the cause less often here, 1 of 4 against 3 of 4. That is too small to conclude anything, but it is worth watching, because the skill's report format asks for a "short problem and fix".

## Limits now stated in the study's READMEs

- **Key scope.** The eligible set keeps none of the 21 soundness entries and 4 of 16 resource-exhaustion entries, and the prompt tells the skill arm that its scope overrides the skill. The comparison does not measure the skill's soundness and unsafe guidance.
- **False-finding controls.** Verifier inputs are retained for 23 of the 51 reports. In those, 7 of 10 injected findings were one "title. file:line. explanation" paragraph, a shape that matched 0 of 354 real passages, and none of the 10 cited evidence. The study disclosed leftover count cues but not this. The verifiers did run real refutations, but 65 of 65 refuted does not show they would catch a false finding that looks real.
- **Blocked audits.** "No blocked audit was retried or rephrased" holds only within the original condition. After 14 `cyberPolicy` errors, v2 reviewed the same crates under a narrower prompt. That prompt removes the security work rather than disguising it, and the study discloses it, but the cause of the blocks is unknown.

## The recommendations

These are review opinions, not measured results. The document in `review-v2/` is that agent's output and stays as written.

- **Separate hygiene counts: test it.** More hygiene items is the one effect seen on both models: 86 vs 28 here and 111 vs 12 on Opus 5.5. The change affects reporting only.
- **Evidence for create and modify summaries: test it.** The support is small, a 0.25-point deduction on the two DECOM skill runs, but the cost is also small.
- **Evidence for each audit finding: hold it.** The v2 prompts already asked both arms for commands and evidence, so v2 cannot show the need. The wording also conflicts with the skill's "short problem and fix" format and "shortest report" rule. The table above is a reason to look at this again, not a result that settles it.

## Not checked

- the 60 harness tests, which CI does not run
- the coding-stage grades
- verifier labels beyond the two probes
- the decoy score counts
- the first-round audit data
- the bot's review threads on PR 24, apart from the YAML import claim, which is correctly refuted
