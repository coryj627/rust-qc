{
  "candidate": "B",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap each pass in results/B/hidden/hidden-default.log and hidden-no-default.log."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap each pass in both hidden feature configurations."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing each pass in both hidden feature configurations."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing each pass in both hidden feature configurations."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "alacritty_terminal/tests/origin_mode.rs:18 and :46 test set/reset with margins CSI 3;5 r, asserting home coordinates and subsequent output destinations. Both pass in results/B/hidden/candidate-integration-1.log."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "origin_mode.rs:73 and :97 save enabled/disabled origin, change DECOM, restore, and assert saved position, restored mode, and subsequent addressing. Both pass in candidate-integration-1.log."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "saved_origin_mode_preserves_pending_wrap at origin_mode.rs:120 explicitly asserts restored wrap behavior and the next character's destination for both saved origins. Its successful execution appears in candidate-integration-1.log."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log records 128 passes; existing-lib-no-default.log records 127 passes. existing-ref-default.log records all 44 original reference names passing. Baseline comparison confirms all original test files and fixtures are byte-identical, with no removals, ignores, or weakened assertions."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All eight baseline-passing neighbor controls and all three additional neighbor tests pass in hidden-default.log and hidden-no-default.log, including default-margin homing, latest-save/repeated-restore behavior, and full-reset saved-origin clearing."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log each show successful compilation of the public-API integration binary; status.tsv records exit 0 for both."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "term/mod.rs:299 and :302 add private saved-origin booleans. Existing public declarations and signatures remain unchanged, including swap_alt at :722. No manifest, feature, serialization, or caller-obligation changes appear in the complete diff."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "The set/reset tests at origin_mode.rs:18 and :46 clearly demonstrate homing under each new origin. Tests at :73 and :97 demonstrate restoration of both saved origin states together with non-home positions. These accurately explain the implemented transitions."
    },
    {
      "id": "D2",
      "score": 0.25,
      "max": 0.5,
      "evidence": "Changed names and comments clearly describe saved state, buffer association, and transitions, earning 0.25. reports/B.md:5-11 gives outcomes corroborated by the suite and Miri logs, but supplies no reproducible command lines, so the validation-command component earns 0."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete baseline comparison finds exactly three changed paths: terminal CHANGELOG.md, src/term/mod.rs, and added tests/origin_mode.rs. Changes implement homing, saved-origin lifecycle, focused regressions, and relevant changelog entries; no graphical or unrelated edits occur."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests and lockfiles are byte-identical to baseline. Two private booleans preserve origin alongside each grid's saved cursor; swap/reset handling maintains that association. The test-only advance helper uses existing parser facilities. No unjustified dependency, feature, module, or public abstraction is introduced."
    }
  ],
  "quality_total": 11.75,
  "quality_interval": [11.75, 11.75],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The changed DECOM and cursor handlers introduce no fallible API or error types; no branching error contract is needed."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Complete baseline comparison confirms unchanged Cargo.toml files, features, and Cargo.lock; no runtime dependencies were added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependencies and transitive lockfile contents are unchanged. origin_mode.rs uses existing terminal and vte exports."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No production module or public abstraction is added. term/mod.rs:299 and :302 hold necessary saved state; origin_mode.rs:10 provides a useful escape-processing test helper."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The complete workspace diff contains only the terminal implementation, focused integration tests, and two relevant changelog entries."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "Original tests and fixtures remain byte-identical. All original reference tests execute and pass, as do both hidden configurations and candidate regressions. Inspected restore code changes only ORIGIN while retaining cursor cloning; buffer swaps and resets maintain saved-origin state. No test suppression or demonstrated behavioral regression was found."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "No error API is introduced or changed."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both runtime and test/build dependency harms are absent; 0 is the categorical reporting encoding."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both needless-module and unrelated-edit harms are absent; 0 is the categorical reporting encoding."
    }
  },
  "limits": [
    "Assessment used actual submitted source, tests, and separately executed logs; no behavioral rerun was necessary.",
    "The assessor's prepared.sha256 verification was blocked by the absent manifest. input-integrity.json separately records an operator byte comparison confirming supplied runtime assets match the frozen commit; this supplements provenance without providing behavioral credit.",
    "The gate failed strict Clippy and rustdoc checks at unchanged source locations, not behavioral assertions. No independent pristine gate reproduction was supplied. Gate formatting was skipped because its undated nightly detection failed; changed-file stable formatting passed, while package-wide stable formatting reported differences only in unchanged selection.rs.",
    "Without default features, the original serde-gated reference target executes zero tests; all 44 original references execute and pass with default features.",
    "Evidence does not establish exhaustive terminal conformance, all downstream API compatibility, graphical behavior, cross-platform behavior, or minimum-Rust-version support. No scored criterion remains unresolved."
  ]
}