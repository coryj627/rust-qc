{
  "candidate": "A",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap pass in both hidden-default.log and hidden-no-default.log under /work/results/A/hidden. term/mod.rs:1996 enables ORIGIN before calling goto(0, 0), which applies the scrolling-region offset and clears pending wrap."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap pass in both hidden feature configurations. term/mod.rs:2048 removes ORIGIN before goto(0, 0), selecting screen origin and clearing pending wrap."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing pass in both hidden feature configurations. term/mod.rs:1629 saves origin state; :1637 restores the saved cursor and origin flag without rehoming."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing pass in both hidden feature configurations. term/mod.rs:1642 uses mode.set with the saved boolean, correctly restoring false as well as true."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate test origin_mode_homes_cursor, tests/origin_mode.rs:15, exercises both DECOM directions through escape processing with margins 3–6, asserting home position, cleared wrap, and subsequent output destination. It passes in hidden/candidate-integration-1.log."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "save_restore_cursor_origin_mode, tests/origin_mode.rs:56, loops over saved-on and saved-off states, changes DECOM between ESC 7 and ESC 8, and asserts restored cursor, mode, and subsequent addressing. Logged execution passes."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Distinct neighboring tests saved_origin_mode_per_screen (:114) and reset_clears_saved_origin_mode (:135) assert per-buffer saved state and RIS clearing both saved states. Both pass in hidden/candidate-integration-1.log. Additional assertions cover saved rendition, charset, wrap, and retained margins."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "hidden/existing-lib-default.log records 128 passes; existing-lib-no-default.log records 127 passes, with no ignored or filtered tests. existing-ref-default.log records all 44 original reference tests passing. Baseline reference names exactly match logged passes. Complete baseline comparison shows existing test sources, fixtures, manifests, and test configuration unchanged; no tests were removed or bypassed."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All 11 neighbor_ tests pass in both hidden-default.log and hidden-no-default.log: the eight established controls and the three additional default-margin, latest-save/repeated-restore, and full-reset checks."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log both show successful compilation of the public-API integration suite; status.tsv records exit 0 for both --no-run commands."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Diff inspection finds only two private Term boolean fields at term/mod.rs:299 and :302, their initialization/lifecycle handling, and existing handler-body changes. Public signatures, public cursor/grid representations, serialization, manifests, and features remain unchanged; no new caller obligations are introduced."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "/work/reports/A.md accurately explains homing under the new origin in both directions and saving/restoring origin with cursor state. tests/origin_mode.rs:16–17 and :57 explicitly exercise both boolean states, with position and addressing assertions."
    },
    {
      "id": "D2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The report supplies the reproducible terminal-only command RUSTUP_AUTO_INSTALL=0 TMPDIR=/work/tmp cargo +1.99.1 test --locked --offline -p alacritty_terminal and reports 177 passes, corroborated by own-default-suite.log: 128 library, 4 candidate, 44 reference, and 1 documentation test. Field comments and test names clearly describe the state transitions. Changed-file stable formatting passed, with nightly-option limitations disclosed."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete diff against /work/baseline contains only alacritty_terminal/src/term/mod.rs and the added alacritty_terminal/tests/origin_mode.rs. Production changes implement homing and preserve saved-origin state through save/restore, screen swaps, and reset. No graphical application or unrelated files changed."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests and Cargo.lock are byte-identical to baseline. The two private booleans track origin state alongside each buffer's saved cursor; the small test advance helper feeds escape sequences through the existing parser. No unjustified dependency, feature, module, or public abstraction was added."
    }
  ],
  "quality_total": 12,
  "quality_interval": [12, 12],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "No error types or fallible APIs are introduced or changed. The modified cursor and mode handlers retain their existing signatures."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Complete baseline comparison shows all manifests, features, and Cargo.lock unchanged; there are no new or upgraded runtime dependencies."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and Cargo.lock are unchanged. tests/origin_mode.rs uses existing terminal APIs and dependencies."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No production modules or types were added. Private saved-origin fields at term/mod.rs:299–302 support existing per-buffer cursor state; tests/origin_mode.rs:7 adds a useful parser helper."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The complete workspace diff is confined to terminal DECOM/save-state handling and its regression tests. Screen-swap and reset edits maintain the new saved state consistently; graphical code and unrelated behavior are untouched."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No existing tests, fixtures, or assertions were deleted, ignored, weakened, or bypassed. All 44 original reference tests, existing library tests, 11 hidden neighboring checks in both configurations, and candidate regressions pass. No behavioral regression was observed."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "Not applicable: no error API changes."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Binary reporting encoding only: both runtime_dependencies and test_build_dependencies are absent."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Binary reporting encoding only: both modules and unrelated_edits harms are absent."
    }
  },
  "limits": [
    "The retained frozen-bundle integrity check could not run because evidence/prepared.sha256 was absent. /work/input-integrity.json separately records the operator's byte comparison confirming supplied runtime assets match the frozen commit; this supplements provenance and is not behavioral evidence.",
    "The gate failed on seven Clippy diagnostics and three strict rustdoc diagnostics in unchanged code. Its tests passed. These diagnostic failures do not establish a candidate behavioral failure, and no independent pristine gate rerun was performed.",
    "Gate formatting was skipped because its nightly-alias detection did not recognize the installed dated nightly. Changed-file stable formatting passed; package-wide stable formatting reported differences only in unchanged selection.rs. Neither establishes full nightly-format validation.",
    "Evidence covers the supplied feature configurations and sampled terminal behaviors, not exhaustive conformance, every downstream caller, minimum supported Rust, other platforms, or graphical behavior. No rubric-scored portion remains unresolved."
  ]
}