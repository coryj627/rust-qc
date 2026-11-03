{
  "candidate": "D",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap all pass in both hidden-default.log and hidden-no-default.log under /work/results/D/hidden. Each earns 0.5."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap all pass in both hidden feature configurations. Each earns 0.5."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing all pass in both hidden feature configurations. Each earns 0.5."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing all pass in both hidden feature configurations. Each earns 0.5."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate test origin_mode_homes_cursor, alacritty_terminal/src/term/mod.rs:2551, exercises both DECOM directions through ansi::Processor with margins 3;6. It asserts the appropriate home, cleared pending wrap, and subsequent output destination. Both directions earn 0.25; the test passes in existing-lib-default.log and existing-lib-no-default.log."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "save_restore_origin_mode at term/mod.rs:2588 loops over saved-on and saved-off states, changes DECOM between ESC 7 and ESC 8, and asserts restored cursor equality, origin flag, subsequent addressing, and clamping. Both states earn 0.25; execution passes in both library configurations."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "unrelated_modes_preserve_cursor at term/mod.rs:2671 distinctly asserts that cursor-key and line-wrap mode changes preserve the cursor, pending wrap, and margins. This supplies meaningful neighboring coverage (0.25), and its named pass appears in both library logs (0.25). Additional candidate tests cover screen switching and reset."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log records 133 passes and existing-lib-no-default.log records 132, with no failures, ignored tests, or filtering. The original term test block is byte-for-byte preserved; all other library source files are unchanged. tests/ref.rs and fixtures are unchanged, and all 44 original reference names match the 44 passes in existing-ref-default.log. Library and reference portions each earn 0.5."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Both hidden execution logs show all 11 neighbor_ tests passing: the eight controls for margins, clamping, region linefeed, rendition/charset, pending wrap, unrelated-mode restoration/stability, and retaining current margins (0.25), plus default-margin homing, latest-save/repeated-restore behavior, and full reset (0.25)."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log both show successful compilation of the public-API integration test executable, with exit 0 in status.tsv. Each configuration earns 0.25."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Diff inspection finds only two new private bool fields in Term at term/mod.rs:290 and :293, initialized by the existing constructor. Public declarations and handler signatures remain unchanged. Cursor/Grid representations, serialization, manifests, and features are unchanged; callers acquire no new obligations."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The report accurately describes homing on DECOM set/reset; branches at term/mod.rs:1996 and :2048 update the mode before goto(0, 0). Tests at :2551 explicitly distinguish region and screen homes (0.25). save_restore_origin_mode at :2588 clearly demonstrates restoring both saved origin states together with position and later addressing (0.25)."
    },
    {
      "id": "D2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The report identifies the exact terminal-only locked/offline command supplied in the task by reference: RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp cargo +1.99.1 test --locked --offline -p alacritty_terminal. Its reported 133 unit, 44 reference, and 1 doctest passes are reproduced in own-default-suite.log (0.25). Private-field comments, direct mode transitions, and descriptive tests make the behavior understandable; the report accurately acknowledges formatting limitations (0.25)."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Independent full-workspace comparison finds 332 regular files in both baseline and submission, no added or removed files, and exactly one changed file: /work/candidates/D/alacritty_terminal/src/term/mod.rs. Changes implement DECOM homing, saved-origin lifecycle handling, and five focused regression tests. No graphical-application edits, bulk formatting, or unrelated refactors appear."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Every Cargo.toml and Cargo.lock is byte-identical to baseline. No dependencies, features, modules, or public abstractions are added. The two private saved-origin flags track the existing per-screen saved cursors through save, restore, screen swaps, and reset; the small test-only process helper feeds escape sequences."
    }
  ],
  "quality_total": 12,
  "quality_interval": [12, 12],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The submitted diff introduces no fallible API or error type. Existing DECOM and cursor handler signatures remain unchanged."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Full-workspace comparison confirms all manifests, feature declarations, and Cargo.lock are unchanged. No runtime dependency is added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and the lockfile are unchanged. Candidate tests use existing ansi::Processor and terminal test facilities without additional dependencies."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No modules or types are added. The private flags at term/mod.rs:290 and :293 associate origin state with existing saved cursors; the helper at :2543 directly supports escape-sequence tests. These additions serve the fix."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "Only term/mod.rs differs from baseline. Its production changes concern DECOM homing and maintaining saved origin state across established cursor/screen/reset operations; remaining additions are focused tests."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No original tests, fixtures, assertions, or ignore settings are removed or weakened. All 44 original reference tests execute, both library configurations pass, and all 23 hidden tests pass in both configurations. Verification-file-changes.json records no submission modifications during checking."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "No new or changed error API; categorical judgment is not_applicable."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Runtime and test/build dependency harms are both absent. Binary encoding only; no quality deduction."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Needless modules/abstractions and unrelated edits are both absent. Binary encoding only; no quality deduction."
    }
  },
  "limits": [
    "The assessor's frozen-bundle integrity check could not run because evidence/prepared.sha256 was absent. /work/input-integrity.json separately records operator byte comparisons confirming supplied runtime assets match the frozen commit; this supplements the retained provenance limitation and is not behavioral evidence.",
    "The gate exits 1 for seven strict Clippy diagnostics and three strict rustdoc diagnostics in unchanged baseline code/comments. Its tests pass. Package-wide stable formatting fails on unchanged selection.rs; changed-file stable formatting passes, while the gate skips nightly formatting. These are diagnostic outcomes, not hidden assertion failures or automatic quality deductions.",
    "Evidence covers the sampled terminal behavior and inspected API changes, not exhaustive conformance, every downstream caller, minimum Rust version, graphical behavior, or cross-platform execution. No rubric-scored check remains unresolved."
  ]
}