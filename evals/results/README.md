# Answer-key evaluation results

Skill version tested: `v1.0.0`. Forty audits ran, four on each of 10 Rust crates at an old release: two with the skill and two without. All ran on Opus 5.5 at high effort with a 45-minute budget.

## Result

**On this evaluation, the skill did not help audits find more bugs.** Audits with and without it found about the same number of known bugs, and about the same number of real bugs overall. The skill's clear effect was adding items that are not defects: lint counts, missing documentation and toolchain drift.

| Measure, 20 runs per arm | With skill | Without skill | Sign-flip p |
|---|---|---|---|
| Known bugs found, of 174 reproduced run-bug pairs | 131 (75%) | 135 (78%) | 0.70 |
| Known bugs found or partly found | 139 (80%) | 141 (81%) | 0.88 |
| Real defects reported, verified union | 855 | 838 | 0.78 |
| Real high-severity defects reported | 203 | 194 | 0.20 |
| Items that are not defects | 111 | 12 | 0.002 |
| Items refuted | 1 | 0 | n/a |
| Mean minutes used of 45 | 26.2 | 23.6 | n/a |

The p values come from an exact test that flips the sign of each project's difference between the arms, with 10 projects. They are not corrected for testing several measures.

The gap between the arms is no larger than ordinary variation between runs. Two runs in the same arm differed by 0.9 known bugs on average, and the arm means differed by 0.8 per project, in mixed directions: 3 projects favoured the skill, 5 favoured no skill and 2 tied. With h2 left out, the totals are 123 and 122.

Of the 87 reproduced bugs, both arms found 67, only the skill arm found 6, only the other arm found 9, and neither found 5.

| Bug kind, share of run-bug pairs found | With skill | Without skill |
|---|---|---|
| Crash from input | 33/40 (82%) | 35/40 (88%) |
| Soundness | 32/40 (80%) | 35/40 (88%) |
| Wrong behaviour | 50/64 (78%) | 51/64 (80%) |
| Resource exhaustion | 16/30 (53%) | 14/30 (47%) |

### How to read this

- **It does not show the skill is useless.** The evaluation scored bug finding. The skill's other aims, such as how code is written and changed, were not tested. Some "not a defect" items, such as missing `# Errors` sections, are things the skill asks for on purpose. They count as noise only for finding bugs.
- **The comparison may leave little room for the skill to help.** Both arms already found about three quarters of the known bugs. The prompt told every run to audit for defects, and every run had a sandbox, Miri and time left over. A weaker model, a tighter budget or a vaguer prompt might show a different result, but this evaluation did not test those.
- **This is one model at one effort.** Two runs per arm can't detect a small effect.

The verified union holds 623 real items, against 87 reproduced key bugs, so most real findings in both arms lie outside the keys. The audits' accuracy looks high, with 1 of 701 items refuted, but see the limits below.

## Limits and departures from the method

- **Web access was forbidden, not turned off.** The tools could not be disabled in a running session, so the brief forbade them. No transcript shows a web tool, skill or subagent used, or cargo run on the host.
- **Blindness rested on instruction.** Agents could technically read other folders. Transcripts show none read the keys, other runs or the arm map when not given them.
- **Grading was only partly blind.** Reports were lettered at random, severity words were mapped to one scale, and the skill's headings, suggested fixes and verification sections were removed. Style and lint-type findings can still suggest the arm. The session running the evaluation knew the arms, but separate agents did the grading.
- **Redaction dropped 28 lines outside verification sections.** They are lint, tooling or workspace notes, and none is a finding a key depends on.
- **Graders and verifiers are the same model as the auditors.** The two key graders agreed on all 368 scores, which shows stability, not independence. A hand check of 24 random scores found no errors, but it was done by the same model family. Verifiers may lean towards confirming what they read. No person who writes Rust has reviewed these results.
- **Verifier labels were not all checked.** Each verifier wrote and ran real probes, but no one has checked its 701 labels one by one. Which reports contain each union item is the verifier's judgement.
- **The keys favour findable bugs.** They hold bugs chosen because they could be reproduced at the release, which may raise scores for both arms.
- **The time budget was self-enforced.** Runs read a clock and stopped themselves. No run reached the limit.
- **A hang cost some runs time.** Four runs wrote a stray `cat > /tmp/null`, which hung for 6 to 13 minutes. One was with the skill and three without. It created an empty file outside the workspace, which each run removed. A warning added to the last 24 briefs did not stop it.
- **Briefs changed during the evaluation, the same for both arms.** A tooling rule stops subagents writing report files, so after the 4 h2 runs the report became the final message. The last 24 briefs also added the warning about standard input.
- **Small rule breaks, none affecting audit evidence.** Two runs without the skill loaded TaskStop through ToolSearch, and one of them used it. One verifier listed the sandbox wrapper's path outside its workspace. One key grader used `cat` and `ls` instead of Read on its brief.
- **Everything ran on aarch64.** x86-only code was only read.
- **The sandbox image gained clippy and rustfmt for this run,** because the skill's check needs them. The Rust version did not change.

## Files

- `runs.tsv`: each run's project, arm, grading letter, minutes used and whether it hit the hang.
- `key_scores.json`: each known bug's score for each run.
- `findings.jsonl`: the verified union, with status, severity and the runs that reported each item.
- `reports/<project>/<arm>-<run>.md`: each run's report as returned, with workspace paths shortened.
- `method/`: the briefs each kind of agent received, and the scripts that built the workspaces and checked the transcripts.
- `score.py`: prints every table above. Run it with `python3 -I evals/results/score.py`.

Each key's `runs` list in `../keys/` records the skill version, arm, model and bugs found for that project.
