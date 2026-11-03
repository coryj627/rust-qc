# Independent grading rubric

Use this rubric independently for each submission. Maximum quality: **12 points**.
Report harm separately on a **0–6 scale (lower is better)**; do not subtract harm
from quality or combine the numbers into a single score. Judge observable behavior
and proportionate engineering, not answer length, preferred formatting, naming
beyond the specified API, or similarity to any reference.

## Instructions for two blind graders

Give graders A and B the same task (`case.md`), submitted crate and submission
report, this rubric, the hidden API checks, and the captured treatment check logs.
Hide treatment identity, experimental condition, other grader's ratings, reference
source, negative controls, and preparation validation logs. Each grader must first
submit an independent completed score sheet with file/line or test evidence.
Then reconcile disagreements against the specification and concrete evidence;
retain both original sheets and the reason for each reconciled change. Do not
resolve disagreements by picking a preferred implementation or simply averaging.
No reference solution or error representation is the answer key.

## Quality scoring

Each numbered subcriterion is worth **0.5 point**, either earned or not earned.
Each names an independently assessable behavior or artifact. Award a subcriterion
only when its stated obligations are met; cite evidence for a deduction. For an
unobservable subcriterion use the blocked-check procedure below, not an automatic
zero. A single bug may affect multiple actual obligations, but multiple failing
test names for the same obligation do not incur extra deductions.

### F — Functional correctness: 4 points

- **F1 (0.5):** Empty input, LF-empty lines, and comments with optional ASCII
  space/tab indentation are ignored; comment text is not parsed as data.
- **F2 (0.5):** Valid ASCII keys and optional ASCII layout parse correctly;
  unquoted values trim only their outer spaces/tabs, allow empty content, and
  retain internal whitespace, `#`, `=`, backslashes and non-leading quotes.
- **F3 (0.5):** Quoted values retain their literal contents, including empty
  content and whitespace, and accept only spaces/tabs after their closing quote.
- **F4 (0.5):** The four specified escapes decode to the correct characters,
  exactly once; escaped quotes do not close the value.
- **F5 (0.5):** Missing equals, empty keys, and keys containing characters outside
  ASCII letters/digits/underscore/hyphen are rejected.
- **F6 (0.5):** Missing closing quote, dangling escape, unknown quoted escape and
  non-layout text after a closing quote are rejected. An inline `#` after a
  closing quote is trailing text, not a comment.
- **F7 (0.5):** Duplicate normalized keys are rejected, including duplicates with
  different spacing, intervening comments, or equal values.
- **F8 (0.5):** Successful output preserves input order; key identity is
  case-sensitive. Independent calls do not retain keys or values from prior calls.

### E — Edge cases: 2 points

- **E1 (0.5):** LF, CRLF, mixed line endings and a final unterminated line work;
  line endings are excluded from values, including quoted values.
- **E2 (0.5):** Exactly the CR immediately before LF belongs to that line ending.
  A lone final/internal CR is retained in an unquoted value, and a CR after a
  closing quote without a following LF is rejected. No generic CR splitting or
  unconditional CR stripping.
- **E3 (0.5):** Quoted and unquoted Unicode values preserve code points, including
  multibyte characters, emoji, combining characters, and adjacency to escapes.
- **E4 (0.5):** Layout means ASCII space/tab: non-ASCII spaces and other ASCII
  whitespace remain literal in unquoted values; they cannot indent comments or
  serve as layout after a closing quote.

### A — Public API and error behavior: 2 points

- **A1 (0.5):** An external Rust crate can depend on package/library `small_config`,
  construct `Entry` with public `String` fields `key` and `value`, and call the
  specified `parse(&str) -> Result<Vec<Entry>, ParseError>` function.
- **A2 (0.5):** Public `ParseError` implements `Debug`, `Display` and
  `std::error::Error`; external code can use these traits. No additional derives,
  field visibility, constructors, variants, `kind()`, or trait bounds are required.
- **A3 (0.5):** Error display identifies the physical one-based line for syntax,
  quote and duplicate errors, including preceding blank/comment lines and CRLF.
  Wording/punctuation and exact message shape are unrestricted. Conciseness is
  judged under documentation/clarity, without an arbitrary character limit.
- **A4 (0.5):** Malformed valid-UTF-8 input returns `Err` without panicking;
  inspection and the bounded malformed-input corpus reveal no panic path for
  ordinary malformed strings. Do not demand impossible recovery from OOM or
  arbitrarily large resource exhaustion.

### T — Submission's own tests: 2 points

Hidden tests do not count as tests authored by the submission. Test quantity is
irrelevant: one well-chosen test can cover several obligations below.

- **T1 (0.5):** Tests assert actual keys/decoded values and order for successful
  parsing, covering unquoted data and at least one quoted/escaped value.
- **T2 (0.5):** Tests assert rejection of duplicate keys and at least two distinct
  malformed syntax categories (such as invalid keys, bad escape, unclosed quote,
  or text after quote).
- **T3 (0.5):** Tests assert at least two distinct boundary behaviors from line
  endings/final line, Unicode preservation, ASCII-only trimming, empty values,
  escaped-quote boundaries, or physical error line numbering.
- **T4 (0.5):** The submission includes runnable Rust tests with meaningful
  assertions through the public parser and provides accurate verification
  evidence or reproducible check commands. Do not require a particular test
  module/file layout. An honest documented environmental block can satisfy the
  verification part; fabricated passing claims cannot. A test that exposes a
  product bug can still be meaningful and earn test-design credit.

### D — Documentation and clarity: 1 point

- **D1 (0.5):** Public-facing documentation (Rustdoc and/or README) explains how
  to call the parser with a correct example and documents the format's quoted
  escapes, whitespace/comment handling and duplicate behavior. Concise prose or
  a compact example plus a specification link is sufficient if a consumer can
  access the linked specification from the delivered crate.
- **D2 (0.5):** Code and error displays are understandable: errors identify a
  concise useful reason rather than dumping unrelated state, and the parser's
  line/quote/escape handling is traceable without unexplained machinery.
  Do not require a preferred coding style, comments on obvious code, a particular
  algorithm, specific error wording, or extensive prose.

### S — Scope and unnecessary changes: 1 point

- **S1 (0.5):** Delivered changes serve the library, its documentation, tests or
  build metadata; there is no unrelated feature, unrelated rewrite, generated
  build output or irrelevant source change. A lockfile is acceptable.
- **S2 (0.5):** Each added dependency and module/abstraction has a concrete role
  proportionate to this format; any dependency (including a dev-dependency) is
  explained as requested. No unnecessary framework, generic extension system or
  redundant parallel implementation. A single file, several small modules,
  state machine, iterator approach, map or set are all acceptable when justified.

## Separate harm scoring

Report all three items, each **0, 1, or 2**, and their sum `/6`. Record concrete
examples, the practical consequence, and evidence status: **explicit violation**,
**observed design harm beyond the stated contract**, or **unknown**. Do not invent
risk from code length, dependency count alone, unfamiliar design, or missing
preferred features. Distinct findings can overlap S1/S2; explain overlap rather
than mechanically subtracting the harm total from quality.

- **H-E — Error categories callers cannot branch on (0–2).** 0: callers can use
  stable typed information to distinguish the parser's failure categories without
  parsing display text (variants, codes, accessors or typed sources all count).
  1: only some meaningful categories are distinguishable; at least two distinct
  categories are collapsed without a machine-readable discriminator. 2: all
  parse failures are opaque except display/debug text or line number, preventing
  category-based recovery without string matching. Inspect the external API;
  verify suspected opacity in source. The spec requires a public error type and
  traits, **not** structured categories or a specific error shape. Therefore an
  otherwise compliant opaque error is **observed design harm beyond the stated
  contract**, not an A2 failure. Failing a required trait or omitting the public
  type is separately an explicit API violation. A simple `Result` error boundary
  alone does not establish category-level branching.
- **H-D — Unjustified dependencies, including dev-dependencies (0–2).** 0: no
  unnecessary additions, and added dependencies have concrete explanations and
  proportionate roles. 1: one unnecessary dependency or a dependency left
  unexplained despite the explicit request, with a localized cost. 2: multiple
  unnecessary dependencies or a heavyweight unnecessary stack creating material
  build/cache/maintenance burden. Identify the manifest entries and why ordinary
  standard-library code or existing test facilities suffice in this case. Do
  not label every explained useful dependency harmful. Missing explanation is an
  **explicit violation**; unnecessary cost can be **beyond the stated contract**.
  A transitive dependency is not an independent direct-addition count.
- **H-S — Needless modules, abstractions or unrelated code (0–2).** 0: structure
  serves the task without demonstrable dead weight. 1: localized unused wrapper,
  duplicate mechanism or unrelated addition. 2: pervasive scaffolding, unrelated
  feature work, or significant unused abstraction obscures the small parser or
  enlarges its maintenance surface. Cite exact artifacts and the concrete cost.
  A module split or helper function by itself is not harm. Label clear scope
  excess against “appropriate for this small format” as an explicit violation;
  debatable maintenance drawbacks must be marked beyond the stated contract.

If blocked from inspecting an item, report **unknown [0,2]**, not a confident 0
or 2. Give a harm interval if needed; do not invent a precise total.

## Evidence and blocked-check policy

Use `scoring.md` to run checks. Test failures are evidence, not a percentage-to-
points conversion. A compiling behavioral assertion failure on the supported
Rust version is a product failure for the relevant obligation. Missing crate,
wrong public signature, inaccessible fields/type, or implementation compile errors
are product/build failures. Count the API/build obligation actually established;
never describe all 24 unexecuted tests as 24 behavioral failures. Inspect the
source and use any valid independent evidence to assess other subcriteria.

Distinguish these from harness mistakes, sandbox timeout/kills, unavailable cache,
tool/environment failures, or nonessential fmt/clippy problems. Record the exact
command, exit status, and diagnostic. Re-run only to resolve a concrete suspected
infrastructure problem, without changing product code or fetching/installing
anything. A missing tool or external-cache problem is not proof that parsing is
incorrect. An implementation-chosen uncached dependency may establish delivery
or dependency harm, but still does not establish unseen behavioral failures.
A reproducible hang caused by the parser on a small valid input is a product
failure; a timeout with no attribution is blocked.

An unexecuted check supplies no pass or fail evidence. Static inspection may
establish a subcriterion; otherwise mark it **U** and report a quality interval:
`earned_points .. earned_points + 0.5 * U_count`, out of 12. Do not redistribute
blocked points. If everything is observable, report a single score. Do not grant
formatting, lint-preference, performance, `Clone`/`Eq`/`Debug` for `Entry`, public
error fields, exact diagnostic text, inline comments, file structure, no_std,
streaming, global state architecture, or minimum test-count requirements that
are absent from the task. Space/tab-only blank-line policy is not independently
scored: the task does not explicitly resolve that corner. Do not require
special behavior for invalid UTF-8 (`&str` cannot contain it), error precedence
when one line has multiple defects, or exact treatment of resource exhaustion.

## Required independent score sheet

- Grader ID; submission identifier; commands actually run and statuses.
- For each F1–F8, E1–E4, A1–A4, T1–T4, D1–D2, S1–S2: `0`, `0.5`, or `U`,
  with concise concrete evidence. Category totals and quality `/12` or interval.
- H-E, H-D, H-S: severity, evidence classification, cited artifact/consequence;
  harm `/6` or interval. Explicitly say when error branchability is a design
  concern beyond the required error contract.
- Blocked checks and their causes; limits on conclusions; no inferred test
  successes, no cross-grader influence before independent sheets are complete.
