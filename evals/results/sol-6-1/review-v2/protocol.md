# Revised comparison protocol

The readiness pilot is excluded regardless of outcome. The comparison uses the
committed v2 templates, original pinned project releases and archived rust-qc
v1.0.0. All model agents use fresh isolated profiles, `gpt-6.1-sol` and high
effort. Treatment receives only the archived skill; baseline and blind grading
profiles receive none. Execution remains inside the unchanged `sb` sandbox.

The private frozen order contains 40 detailed 45-minute reviews and 12 concise
15-minute reviews, each with two runs per arm per project. Slot waits do not
count. The two stages remain separate. Prompt hashes and the schedule commitment
are in `comparison-plan.json`; individual prompt hashes and arm mappings remain
private until grading ends. No primary review has the pilot's profile or outputs.

The primary known-bug measure is functional-contract recall against the 50
eligible entries in `comparison-eligibility.json`. The other 42 entries are
outside that denominator, not misses. As in the original scorer, report the
47 reproduced entries as the primary subset and all 50 eligible entries as a
separate secondary measure. With two replicates, the detailed cell has 94
primary run/bug pairs per arm; the short cell has 36 per arm. These are operator scope decisions made
before reading the pilot report. They do not imply an included bug has no
possible security impact. Reviewers receive no key or eligibility answers.

Before grading, separate clean agents prepare two confirmed false functional key
entries and five confirmed false functional findings per project, without reading
review outputs. Two independent blind key graders and one verifier assess each
stage/project cell. False controls are mixed into their inputs and excluded from
every substantive total. Redaction replacements and drops are logged. In v2,
verification sections are retained because they can contain regression evidence;
tool names are redacted within them. The original study's section-dropping mode
is unchanged. Overall finding counts are normalized before control insertion,
with each change logged and numeric defect evidence preserved. Verification
uses ordinary functional evidence only; claims requiring dedicated security work
are left unverified with the scope reason, not subjected to that investigation.

Agreed key judgments are primary. Disagreements require a fresh blind adjudicator;
unresolved judgments stay unknown. A separate agent rechecks a frozen random
ceil(10%) sample of independent key/report judgments against unredacted reports,
including decoys in the population. This diagnostic does not silently overwrite
primary scores; all disagreements and any sensitivity reading are reported.

Use the original exact sign-flip function, report uncorrected p-values and both
replicates/within-arm variation beside every gap. Require two scored runs per
arm for a project comparison. Missing, invalid and ungraded reviews are not zero.
Report non-defects and control outcomes alongside functional recall. New results
are separate from the original Sol/Claude studies because task and denominator
differ. Any new skill recommendations wait for final grading and come from a
fresh agent that did not review or grade and has not previously seen the mapping.

The dispatcher uses `--stop-on-policy`: the first provider block stops further
dispatch across this whole comparison; already-running independent reviews may
finish. Do not reword or resubmit blocked tasks, switch accounts/models, or try
alternative access settings. Usage/disk limits retain the existing global stop.
Preserve all attempted/missing observations and the reason dispatch stopped.
