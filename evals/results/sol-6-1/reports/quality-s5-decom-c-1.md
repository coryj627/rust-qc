{
  "candidate": "C",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap each pass in results/C/hidden/hidden-default.log and hidden-no-default.log. term/mod.rs:1996 enables ORIGIN before calling goto(0, 0); goto clears pending wrap."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap each pass in both hidden feature configurations. term/mod.rs:2048 removes ORIGIN before calling goto(0, 0)."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing each pass in both hidden feature configurations. term/mod.rs:1633 saves origin state; :1642 restores its flag alongside the cloned saved cursor without homing."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing each pass in both hidden feature configurations. mode.set(ORIGIN, saved_origin) restores false as well as true."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate tests/origin_mode.rs:15 establishes non-default margins 3–6 on an eight-row terminal. origin_mode_set_homes_cursor (:23) and origin_mode_reset_homes_cursor (:44) assert home coordinates and subsequent output destinations. Both pass in hidden/candidate-integration-1.log and both own-suite logs."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "restore_cursor_restores_origin_mode_on and restore_cursor_restores_origin_mode_off, tests/origin_mode.rs:63 and :79, change DECOM between save and restore, assert exact saved positions and mode flags, and verify subsequent addressing/output. Both pass in default and no-default own-suite logs."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "restore_origin_preserves_cursor_state_and_unrelated_modes, tests/origin_mode.rs:95, explicitly checks saved pending wrap, rendition, charset, and preservation of current unrelated modes. Additional tests cover separate screen saves and full reset. All seven candidate tests pass in both own-suite logs and own-miri-regressions.log."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "hidden/existing-lib-default.log reports 128 passed and existing-lib-no-default.log reports 127 passed, with no ignored or filtered tests. All original unit tests remain unchanged. The unchanged baseline ref.rs inventory contains 44 names, all matched to passing entries in hidden/existing-ref-default.log; reference fixtures are unchanged. Its existing serde feature guard explains zero reference tests without default features."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Both hidden-default.log and hidden-no-default.log explicitly report all eleven neighbor_ tests passing: the eight baseline controls and the additional default-margin homing, latest-save/repeated-restore, and full-reset cases."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden/hidden-compile-default.log and hidden/hidden-compile-no-default.log both successfully produce the public-API decom_case_hidden integration executable; checks.json records exit 0 for each --no-run command."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The production diff adds only private saved_origin and inactive_saved_origin fields to Term (term/mod.rs:299–303) and updates existing initialization, screen swapping, reset, and handlers. Public declarations and signatures remain unchanged; manifests, feature definitions, serialization declarations, and lockfile are unchanged. No new caller obligation is introduced."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "reports/C.md accurately describes homing after both DECOM directions and restoring origin alongside the saved position. tests/origin_mode.rs:23–91 demonstrates both directions and both saved states with explicit coordinates and addressing assertions; the changelog accurately records both repairs."
    },
    {
      "id": "D2",
      "score": 0.25,
      "max": 0.5,
      "evidence": "Awarded 0.25 for understandable field comments, direct state transitions, and descriptive regression names. The report's totals of 180 default tests, 135 no-default tests, and seven Miri regressions are corroborated by raw logs. However, the candidate report supplies no reproducible validation command, so the combined commands-and-outcomes subcheck earns 0; assessor-provided checks.json does not establish that the candidate supplied commands."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete baseline comparison finds exactly three changed paths: alacritty_terminal/src/term/mod.rs, alacritty_terminal/tests/origin_mode.rs, and alacritty_terminal/CHANGELOG.md. Every change concerns homing, saved origin lifecycle, focused regression coverage, or its explanation. No graphical application edits, bulk formatting, or unrelated refactors appear."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests and the submitted Cargo.lock are byte-identical to baseline. The two private booleans preserve origin alongside each grid's saved cursor; existing screen-swap and reset paths maintain their lifecycle. The two small test helpers process escape sequences and establish margins. No unnecessary module, public abstraction, dependency, or feature is introduced."
    }
  ],
  "quality_total": 11.75,
  "quality_interval": [11.75, 11.75],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The changed DECOM and cursor handlers remain infallible. No error API, error type, or failure representation is introduced or changed."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Complete workspace comparison shows unchanged manifests, features, and submitted Cargo.lock; no runtime dependency is added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependency declarations and submitted lockfile are unchanged. origin_mode.rs uses existing terminal and parser APIs. Gate scratch lockfile pruning recorded in verification-file-changes.json did not alter the submission."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No production module or new type is introduced. Private saved-origin fields support the existing two-grid cursor lifecycle. tests/origin_mode.rs:8 and :15 provide useful parser and setup helpers for the focused integration tests."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The complete diff contains only the terminal handler/state changes, seven relevant regression tests, and two fix-related changelog entries. Graphical application files, existing fixtures, and unrelated source remain unchanged."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No pre-existing tests or assertions are removed, ignored, weakened, or bypassed. All 44 original reference names execute and pass with defaults; existing library tests and all eleven hidden neighboring checks pass. The submission contains no assessor-test edits, and verification records show no submission mutation."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "Not applicable: no fallible API or error representation changes."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Binary reporting encoding: both runtime_dependencies and test_build_dependencies are absent."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Binary reporting encoding: both modules and unrelated_edits are absent."
    }
  },
  "limits": [
    "The assessor's original prepared.sha256 verification could not run because that manifest was absent. input-integrity.json separately confirms that supplied runtime assets match the frozen commit; this supplements provenance and supplies no behavioral credit.",
    "The archived gate failed Clippy and strict rustdoc on unchanged baseline code. Its formatting step was skipped because the expected nightly setup was unavailable; changed-file stable formatting passed but does not verify nightly-only settings. These are distinct from assertion failures and do not create uncertainty in the scored behavioral checks.",
    "Evidence covers the supplied terminal tests and reviewed diff, not exhaustive terminal conformance, every downstream caller, graphical behavior, minimum Rust version, or other platforms."
  ]
}