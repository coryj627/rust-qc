{
  "candidate": "D",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap pass in both hidden-default.log and hidden-no-default.log under /work/results/D/hidden."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap pass in both hidden feature configurations."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing pass in both hidden feature configurations."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing pass in both hidden feature configurations."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "alacritty_terminal/src/term/mod.rs:2551 origin_mode_homes_cursor processes both DECOM directions with margins 3–6, asserting home position and subsequent output destination. It passes in existing-lib-default.log and existing-lib-no-default.log."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "term/mod.rs:2588 save_restore_origin_mode exercises saved-on and saved-off states, changes DECOM between ESC 7 and ESC 8, and asserts restored cursor, mode, subsequent addressing, and bounds. Both library logs record a pass."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Distinct neighboring tests include unrelated_modes_preserve_cursor at term/mod.rs:2671, which asserts cursor and pending-wrap preservation across unrelated mode changes. saved_origin_mode_per_screen and reset_clears_saved_origin_mode add explicit screen-switch and reset expectations. All pass in both library configurations."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log records 133 passes and existing-lib-no-default.log records 132, with no failures or ignored tests. All 128 baseline unit-test declarations are represented in default execution; the original term test section remains byte-identical. The unchanged ref.rs inventory contains 44 tests, and its names exactly match the 44 passes in existing-ref-default.log. No original tests or fixtures were removed or weakened."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All eight baseline-passing neighbor_ controls pass in both hidden logs. The three additional tests—neighbor_default_margins_set_and_reset_home, neighbor_latest_save_wins_in_both_directions, and neighbor_full_reset_discards_saved_origin_state—also pass in both configurations."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log each show successful compilation of the public-API integration test executable; status.tsv records exit 0 for both."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete diff inspection finds only private saved_origin_mode and inactive_saved_origin_mode fields added to Term at term/mod.rs:289–293, their lifecycle handling, handler body changes, and tests. Existing public declarations and signatures remain intact; no serialization, feature, or caller-obligation changes were introduced."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The report accurately describes homing on set/reset. origin_mode_homes_cursor explicitly asserts region home versus screen home. save_restore_origin_mode loops over both saved states and checks restored position and addressing, accurately explaining the second transition."
    },
    {
      "id": "D2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "/work/reports/D.md identifies the exact requested locked/offline terminal-only command by reference to the case and reports 133 unit, 44 reference, and one documentation test passing; own-default-suite.log corroborates those outcomes. Test names, state loops, and comments explain homing, restoration, wrapping, and screen lifecycle without misleading behavioral claims."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Independent comparison of all 332 workspace files finds only alacritty_terminal/src/term/mod.rs changed, with no added or removed files. Changes implement DECOM homing, saved-origin lifecycle, and focused regressions; graphical application files and unrelated implementation remain unchanged."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests, features, and Cargo.lock are byte-identical to baseline. Two private booleans track origin alongside each grid's saved cursor, including switching and reset. The small test-only process helper exercises existing escape processing. No module, dependency, or public abstraction was introduced."
    }
  ],
  "quality_total": 12,
  "quality_interval": [12, 12],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The changed DECOM and cursor handlers remain infallible and introduce or modify no error API."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Complete workspace comparison confirms all Cargo.toml files and Cargo.lock are unchanged; no runtime dependency or feature was added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and lockfile are unchanged. Added tests use existing terminal, parser, and standard-library facilities."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No modules or files were added. The private saved-state fields at term/mod.rs:289–293 support the existing two-grid lifecycle; process at line 2543 is a useful test helper."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "Only term/mod.rs differs across the complete 332-file workspace inventory. Its production changes concern DECOM, cursor saving/restoring, and required initialization, switching, and reset handling."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "Original tests and reference fixtures are unchanged, all 44 reference tests execute, and no tests are ignored or bypassed. Both hidden runs pass all 23 checks, including neighboring behavior. Verification records no submission changes. Gate failures concern unchanged lint/documentation sites, not demonstrated behavioral regressions."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "No new or changed fallible public API; error API harm is not applicable."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both runtime_dependencies and test_build_dependencies are absent; manifests and lockfile are unchanged."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both modules and unrelated_edits are absent. The private saved state and test helper have concrete purposes within the fix."
    }
  },
  "limits": [
    "The assessor's full frozen-bundle integrity check could not run because evidence/prepared.sha256 was absent. /work/input-integrity.json separately records an operator byte comparison confirming supplied runtime assets match the frozen commit; this supplements provenance and earns no behavioral credit.",
    "The archived gate exited 1: seven Clippy diagnostics and three strict rustdoc diagnostics concern unchanged source. Its formatting stage was skipped by nightly-alias detection. Changed-file stable formatting passed; package-wide stable formatting failed only in unchanged selection.rs. These are diagnostic limitations, not behavioral assertion failures.",
    "Assessment uses inspected source and recorded executions; no behavioral rerun was necessary. The available checks do not establish exhaustive terminal conformance, every downstream API use, graphical behavior, other platforms, or minimum-supported-Rust compatibility."
  ]
}