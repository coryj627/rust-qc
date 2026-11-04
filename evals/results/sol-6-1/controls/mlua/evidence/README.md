These controls target the supplied mlua 0.11.4 baseline in /work/crate/src.

All seven refutations were executed on aarch64 Linux with Rust/Cargo 1.99.1,
Lua 5.4 vendored, and features lua54,vendored,serde,macros. The executed command
is in probe-command.txt; the final log is probe-run.log. The small regression
suite is control_probes.rs, copied into the scratch crate's tests directory.
Each test is named for its corresponding control. All seven final tests passed.

The first run had six passes and one fixture failure because Lua::new() does
not load the debug library. The corrected environment probe uses only ordinary
Lua closures and lexical _ENV reassignment to prove initial cell sharing and
subsequent isolation. The initial result is retained in probe-initial.log.

The original crate was never edited. baseline-sha256.json records all original
file hashes; validation.log verifies original and scratch baseline files.
source-excerpts.txt preserves the numbered guard and invariant evidence.
prepare_controls.py writes the JSON and source excerpts. Validation is limited
to these controls; the full upstream test suite, other Lua backends, Miri, and
other feature configurations were not run. No network access was used.
