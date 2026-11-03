{
  "candidate": "C",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap each pass in results/C/hidden/hidden-default.log and hidden-no-default.log. term/mod.rs:1996 updates ORIGIN before calling goto, which clears pending wrap."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap each pass in both hidden feature configurations. term/mod.rs:2048 removes ORIGIN before calling goto."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing each pass in both hidden feature configurations. term/mod.rs:1632 saves the origin flag alongside the cursor; line 1642 restores the flag without homing."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing each pass in both hidden feature configurations. Restoring the saved boolean explicitly clears ORIGIN when the saved state was off."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate tests/origin_mode.rs:15 establishes margins 3–6 on an eight-row terminal. origin_mode_set_homes_cursor and origin_mode_reset_homes_cursor assert both cursor home and printable output destination. Both pass in candidate-integration-1.log and both own-suite logs."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "restore_cursor_restores_origin_mode_on and restore_cursor_restores_origin_mode_off change DECOM between ESC 7 and ESC 8, then assert saved position, mode, output, and subsequent addressing at tests/origin_mode.rs:63 and :79. Both pass in candidate-integration-1.log and both own-suite logs."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "restore_origin_preserves_cursor_state_and_unrelated_modes at tests/origin_mode.rs:95 distinctly checks saved pending wrap, rendition, charset, and preservation of unrelated mode changes. Its explicit assertions pass in candidate-integration-1.log, both own-suite logs, and own-miri-regressions.log."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log records 128 passed; existing-lib-no-default.log records 127 passed. existing-ref-default.log records all 44 original reference tests passing, with no ignored or filtered tests. Baseline comparison confirms unchanged existing test bodies, fixtures, and inventory; all 44 baseline reference names match the logged executions."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All eight baseline-passing neighbor_ controls and all three additional neighbor_ checks pass in hidden-default.log and hidden-no-default.log, including default-margin homing, latest-save/repeated-restore behavior, and full reset."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log each show successful compilation of the public-API integration test binary; status.tsv records exit 0 for both."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete diff inspection finds only two new private Term boolean fields at term/mod.rs:299 and :302, their lifecycle handling, and changed handler bodies. Existing public signatures and Cursor/Grid declarations remain unchanged. No caller obligations, serialization changes, or feature changes are introduced."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "reports/C.md:3 accurately describes homing in both DECOM directions. The explanation and the saved-on/saved-off tests accurately describe restoring origin together with the saved position. Changelog lines 19–20 describe both fixes."
    },
    {
      "id": "D2",
      "score": 0.25,
      "max": 0.5,
      "evidence": "Changed field comments and test names clearly explain the transitions, earning 0.25. The report's 180 default-feature tests, 135 no-default-feature tests, and seven Miri tests match the execution logs. However, the candidate supplies no reproducible validation commands in its report or changed documentation, so the command-and-outcome subcheck earns 0."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Independent complete workspace comparison finds exactly three changed paths: alacritty_terminal/src/term/mod.rs, alacritty_terminal/tests/origin_mode.rs, and alacritty_terminal/CHANGELOG.md. All changes concern DECOM, saved-state lifecycle, or focused regression coverage; no graphical application edits or bulk formatting occur."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All workspace manifests and Cargo.lock are byte-identical to baseline. The two private saved-origin fields maintain state for the existing active/inactive grids; the two test helpers construct terminals and process escape sequences. No unjustified dependency, feature, module, or public abstraction is added."
    }
  ],
  "quality_total": 11.75,
  "quality_interval": [11.75, 11.75],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The modified cursor and mode handlers remain infallible. No error type, fallible public API, or string-only failure mechanism is introduced."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "All Cargo.toml files and the submitted Cargo.lock match baseline byte-for-byte; no runtime dependency or feature is added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and the submitted lockfile are unchanged. origin_mode.rs uses existing terminal and vte APIs. Gate-only lockfile pruning occurred in scratch, not in the submission."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No production module or public abstraction is added. term/mod.rs:299 and :302 hold necessary saved state for the two grids. tests/origin_mode.rs:8 and :15 provide useful local test helpers."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The complete baseline comparison identifies only the terminal implementation, focused integration tests, and two relevant changelog entries. Screen-swap and reset changes maintain the new saved-origin state consistently."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No existing tests or fixtures were deleted, ignored, weakened, or bypassed. All 44 original reference tests execute and pass; both hidden configurations pass all 23 checks. Candidate tests additionally verify alternate-screen saved state, rendition, charset, wrap, unrelated modes, and reset. Verification records no submission mutations."
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
      "evidence": "Needless-module and unrelated-edit harms are both absent. Binary encoding only; no quality deduction."
    }
  },
  "limits": [
    "The assessor's prepared.sha256 integrity manifest is missing, so its original integrity check was unrun successfully. input-integrity.json separately records an operator byte comparison confirming supplied runtime assets match the frozen commit; this supplements provenance, not behavioral evidence.",
    "The gate failed on seven Clippy diagnostics and three strict-rustdoc diagnostics in unchanged baseline code. Its formatting step was skipped because its nightly detection required an undated alias. Stable package formatting failed only in unchanged selection.rs; changed-file formatting passed. These are diagnostic outcomes, not behavioral assertion failures.",
    "The no-default-feature reference target executes zero tests because its unchanged source requires serde; all 44 original reference tests execute with default features.",
    "Evidence is finite and does not establish exhaustive terminal conformance, every downstream API use, graphical behavior, minimum supported Rust compatibility, or cross-platform behavior. No rubric points remain unresolved."
  ]
}