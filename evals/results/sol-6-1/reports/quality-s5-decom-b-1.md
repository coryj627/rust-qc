{
  "candidate": "B",
  "criteria": [
    {
      "id": "F1",
      "score": 1.5,
      "max": 1.5,
      "evidence": "hidden/hidden-default.log and hidden/hidden-no-default.log both pass set_homes_nondefault_margins, set_writes_at_region_home, and set_home_clears_pending_wrap. src/term/mod.rs:1996 enables ORIGIN before calling goto(0, 0), which also clears pending wrap."
    },
    {
      "id": "F2",
      "score": 1.5,
      "max": 1.5,
      "evidence": "Both hidden feature configurations pass reset_homes_nondefault_margins, reset_writes_at_screen_home, and reset_home_clears_pending_wrap. src/term/mod.rs:2048 disables ORIGIN before calling goto(0, 0)."
    },
    {
      "id": "F3",
      "score": 1.5,
      "max": 1.5,
      "evidence": "Both hidden feature configurations pass saved_on_restores_origin_mode, saved_on_restores_position_without_rehoming, and saved_on_restores_origin_addressing. src/term/mod.rs:1629–1643 saves the origin flag and restores it alongside the cloned cursor without homing."
    },
    {
      "id": "F4",
      "score": 1.5,
      "max": 1.5,
      "evidence": "Both hidden feature configurations pass saved_off_restores_origin_mode, saved_off_restores_position_without_rehoming, and saved_off_restores_screen_addressing. Restoring ORIGIN with mode.set at src/term/mod.rs:1642 handles saved false as well as true."
    },
    {
      "id": "R1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate tests/origin_mode.rs:18 and :46 exercise DECOM set and reset through ansi::Processor with margins 3;5, asserting home coordinates and subsequent output destinations. Both pass in hidden/candidate-integration-1.log and the independently executed default/no-default suites."
    },
    {
      "id": "R2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate saved_origin_mode_on_is_restored and saved_origin_mode_off_is_restored, tests/origin_mode.rs:73 and :97, change DECOM between ESC 7 and ESC 8 and assert restored position, mode, and subsequent addressing. Both pass in candidate-integration-1.log and both independently executed feature configurations."
    },
    {
      "id": "R3",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Candidate saved_origin_mode_preserves_pending_wrap, tests/origin_mode.rs:120, explicitly checks retained last-column content, WRAPLINE, next-row output, and final cursor position for both saved origin states. Its successful execution is recorded in candidate-integration-1.log and own-miri-regressions.log."
    },
    {
      "id": "R4",
      "score": 1,
      "max": 1,
      "evidence": "existing-lib-default.log passes 128 tests; existing-lib-no-default.log passes 127, with no ignored tests. existing-ref-default.log passes all 44 original reference tests. Direct comparison of the pristine reference inventory with logged passing names found no missing or additional names. The complete workspace diff removes no files and changes no existing test bodies, fixtures, manifests, or test selection."
    },
    {
      "id": "R5",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-default.log and hidden-no-default.log each pass all eleven neighbor_ tests: the eight controls covering margins, clamping, linefeed, rendition/charset, saved wrap, unrelated modes, cursor stability, and current margins; plus default-margin homing, latest-save/repeated restore, and full reset."
    },
    {
      "id": "A1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "hidden-compile-default.log and hidden-compile-no-default.log both successfully compile the public-API integration suite with --no-run; status.tsv records exit 0 for each."
    },
    {
      "id": "A2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Complete diff inspection finds only two new private bool fields in Term at src/term/mod.rs:298–302 and internal state handling. Term's public declarations, swap_alt signature, Grid/Cursor interfaces, serialization, features, and manifests remain unchanged. No new caller obligations or public abstractions are introduced."
    },
    {
      "id": "D1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "tests/origin_mode.rs:18–68 clearly specifies region-home on set and screen-home on reset. Its saved-on/off tests at :73–116 explain and assert restoration of both origin states together with position and addressing. The changelog at :19–20 and candidate report accurately summarize these changes."
    },
    {
      "id": "D2",
      "score": 0.25,
      "max": 0.5,
      "evidence": "Award 0.25 for understandable state names, comments, and executable transition tests. The report's 180 all-feature, 135 no-default, and seven Miri passes are corroborated by the corresponding execution logs. However, /work/reports/B.md supplies no actual reproducible validation command, only descriptions and counts; the separately executed assessor commands do not satisfy the candidate-provided-command requirement, so the validation half earns 0."
    },
    {
      "id": "U1",
      "score": 0.5,
      "max": 0.5,
      "evidence": "Independent complete-file comparison finds exactly two modified files, alacritty_terminal/src/term/mod.rs and alacritty_terminal/CHANGELOG.md, and one added file, alacritty_terminal/tests/origin_mode.rs. Production changes address homing, saved origin state, its screen-buffer association, initialization, and reset. No graphical application edits, unrelated refactors, or bulk formatting appear."
    },
    {
      "id": "U2",
      "score": 0.5,
      "max": 0.5,
      "evidence": "All manifests, features, and Cargo.lock are byte-identical to baseline. The two private bool fields provide saved state for the existing active/inactive grids. The sole test helper, origin_mode.rs:10, feeds escape sequences through the existing parser. No unjustified dependency, module, or public abstraction is added."
    }
  ],
  "quality_total": 11.75,
  "quality_interval": [11.75, 11.75],
  "harm_checklist": [
    {
      "id": "error_api",
      "state": "not_applicable",
      "evidence": "The complete diff introduces no fallible API or error-type changes. DECOM and cursor handlers retain their existing signatures; no new error API is required."
    },
    {
      "id": "runtime_dependencies",
      "state": "absent",
      "evidence": "Every workspace manifest, feature declaration, and the submitted Cargo.lock is byte-identical to baseline. No runtime dependency is added or upgraded."
    },
    {
      "id": "test_build_dependencies",
      "state": "absent",
      "evidence": "Dev/build dependencies and lockfile are unchanged. tests/origin_mode.rs uses existing alacritty_terminal APIs and facilities. Gate-only scratch lockfile pruning recorded in verification-file-changes.json is not a submitted change."
    },
    {
      "id": "modules",
      "state": "absent",
      "evidence": "No production module or new type is added. Private saved-origin fields at src/term/mod.rs:298–302 track existing per-grid saved cursors; the test helper at tests/origin_mode.rs:10 avoids repeated parser-driving code and has a concrete purpose."
    },
    {
      "id": "unrelated_edits",
      "state": "absent",
      "evidence": "The full baseline comparison identifies only terminal origin-state handling, its focused regression test file, and two relevant changelog entries. All other workspace files are unchanged."
    },
    {
      "id": "suppressed_evidence_or_regressions",
      "state": "absent",
      "evidence": "No existing tests or fixtures are deleted, ignored, weakened, or bypassed. All 44 original reference tests execute and pass; library tests and all eleven hidden neighboring checks pass. The no-default reference target's zero tests follows its unchanged pre-existing serde feature guard, rather than a candidate suppression. Supplied hidden assets match the operator's recorded hashes."
    }
  ],
  "harm": {
    "error_api": {
      "score": null,
      "max": null,
      "applicable": false,
      "evidence": "Not applicable: no error API is introduced or changed."
    },
    "dependencies": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both runtime_dependencies and test_build_dependencies are absent. Binary encoding records the categorical judgment without deducting quality points."
    },
    "structure": {
      "score": 0,
      "max": 1,
      "applicable": true,
      "evidence": "Both needless modules and unrelated edits are absent. The added private state and regression-test helper serve this fix."
    }
  },
  "limits": [
    "The retained assessment could not verify evidence/prepared.sha256 because that manifest was not supplied. The operator's input-integrity.json supplements this limitation; independent hashing confirmed that all seven supplied runtime assets match its recorded hashes. This is integrity evidence, not behavioral credit.",
    "The gate exits 1 for Clippy diagnostics and strict rustdoc errors in code unchanged from baseline; these are diagnostic failures, not test assertion failures or automatic quality deductions. Gate formatting was skipped because its nightly-alias detection did not recognize the installed dated toolchain. Changed-file stable formatting passes, but complete project-format verification remains incomplete.",
    "The evidence covers the supplied finite tests and inspected interfaces. It does not establish exhaustive terminal conformance, every downstream caller, minimum supported Rust, other platforms, or graphical behavior. No rubric-scored behavior remains unresolved, so the quality interval is exact."
  ]
}