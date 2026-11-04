{
  "candidate": "A",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap each passed with default and no-default features in /work/results/A/hidden/hidden-default.log and hidden-no-default.log."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap each passed in both feature configurations in the two hidden execution logs."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing each passed in both feature configurations."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing each passed in both feature configurations."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "/work/candidates/A/alacritty_terminal/tests/origin_mode.rs:15 tests both DECOM directions with margins 3–6, asserting home coordinates, cleared pending wrap, and subsequent output destination. origin_mode_homes_cursor passed in /work/results/A/hidden/candidate-integration-1.log."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "origin_mode.rs:56 loops over saved-on and saved-off states, changes DECOM between ESC 7 and ESC 8, and asserts restored mode, complete cursor state, explicit position, and subsequent addressing. save_restore_cursor_origin_mode passed in candidate-integration-1.log."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Distinct neighboring tests saved_origin_mode_per_screen at origin_mode.rs:114 and reset_clears_saved_origin_mode at :135 explicitly check per-buffer saved state and RIS clearing both buffers' saved state. Both passed in candidate-integration-1.log."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log records 128 passed and existing-lib-no-default.log records 127 passed, with zero failed, ignored, or filtered tests. All 44 names in the pristine baseline's ref_tests! inventory match the 44 passing names in existing-ref-default.log. Complete workspace comparison confirms existing test sources and fixtures are unchanged."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Both hidden execution logs show all eight baseline-passing neighbor controls passed: changing margins, bottom clamping, region linefeed, rendition/charset, saved pending wrap, unrelated-mode restoration, unrelated-mode cursor stability, and current-margin retention. The three additional tests neighbor_default_margins_set_and_reset_home, neighbor_latest_save_wins_in_both_directions, and neighbor_full_reset_discards_saved_origin_state also passed in both configurations."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "/work/results/A/hidden/hidden-compile-default.log and hidden-compile-no-default.log both show successful compilation of the public-API integration suite; status.tsv records exit 0 for each."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Reviewed the complete workspace diff. term/mod.rs:299 and :302 add private boolean fields to Term, whose construction already involves private fields. Existing public declarations and signatures remain unchanged. Save/restore and mode-handler bodies change without new caller obligations; manifests, features, and serialization declarations are unchanged."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "/work/reports/A.md accurately describes homing on both DECOM directions and restoring origin alongside cursor state. origin_mode.rs:30 explicitly distinguishes region and screen homes; :57–107 explains and asserts restoration of both saved origin states together with position and addressing."
    },
    {
      "id": "D2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The report supplies the reproducible terminal-only command RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp cargo +1.99.1 test --locked --offline -p alacritty_terminal and reports 177 passing tests. /work/results/A/own-default-suite.log corroborates 128 library, 4 candidate, 44 reference, and 1 documentation test passing. Changed code and test names clearly describe the transitions."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Independent diff -qr of /work/baseline and /work/candidates/A finds exactly two changed paths: alacritty_terminal/src/term/mod.rs and the added tests/origin_mode.rs. Production changes cover DECOM homing, saved origin state, corresponding buffer swaps, initialization, and reset. No graphical application edits, unrelated refactors, or bulk formatting."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests, features, and Cargo.lock are byte-identical to baseline. The two private booleans track existing per-grid saved cursors; the test-only advance helper feeds escape sequences through the existing parser. No unjustified dependency, module, feature, or public abstraction is introduced."
    }
  ],
  "quality_total": 12,
  "quality_interval": [12, 12],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The submitted diff introduces no fallible public API or error changes. DECOM and cursor handlers remain infallible; there is no new failure representation callers must inspect."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Complete workspace comparison shows every Cargo.toml and Cargo.lock unchanged from baseline. No runtime dependency or feature additions or upgrades."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and transitive lockfile contents are unchanged. origin_mode.rs uses existing terminal and parser APIs without additional dependencies."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No new production module or public abstraction. term/mod.rs:299 and :302 hold necessary saved origin state for the two grids. origin_mode.rs:7 supplies a useful parser-feeding helper for the focused integration tests."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The full workspace diff contains only the terminal implementation and focused regression tests. Buffer-swap and reset changes maintain the new state alongside existing saved cursors; no unrelated files or behavior were changed."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No original tests, fixtures, assertions, or test configuration were removed, ignored, weakened, or bypassed. All original reference names executed successfully; library tests and all 23 hidden tests passed in both supported feature configurations. No candidate alteration of the frozen tests appears in the submission."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "No introduced or changed error API; categorical judgment is not_applicable."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both runtime_dependencies and test_build_dependencies are absent. Binary encoding only; no quality deduction."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both modules and unrelated_edits are absent. Binary encoding only; no quality deduction."
    }
  },
  "limits": [
    "The assessor's original frozen-bundle verification could not run because evidence/prepared.sha256 was absent. /work/input-integrity.json separately records that the operator's byte comparison confirmed the supplied runtime assets match the frozen commit. This supplements the retained provenance limitation and is not behavioral evidence.",
    "The archived gate failed strict Clippy and rustdoc checks on unchanged baseline code and documentation. Its tests passed. Gate formatting was skipped by nightly detection; changed-file stable formatting passed, while package-wide stable formatting reported differences only in unchanged selection.rs. These diagnostics establish no submitted regression.",
    "Results cover the recorded Rust 1.99.1 aarch64 Linux checks and inspected source. They do not establish exhaustive terminal conformance, every downstream API use, minimum Rust version, other platforms, or graphical behavior. No additional behavioral rerun was needed because source and recorded results were consistent."
  ]
}