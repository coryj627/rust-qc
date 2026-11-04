Seven independently specified functional negative controls for mlua 0.11.4.

controls.json uses baseline-relative source paths, rooted at /work/crate.
negative_controls.rs is the preserved public-API probe source. An identical copy
was added as tests/negative_controls.rs in /work/tmp/mlua-controls; production
source and manifests were unchanged. probe.log records the actual seven-test
run, with compilation progress omitted. source-excerpts.txt preserves numbered
baseline lines. baseline-sha256.json records all original crate file hashes.
validate_controls.py checks counts, schema, source references, successful test
identities, original-tree integrity, and scratch-source identity.

Runtime scope: aarch64 Linux, stable Rust 1.99.1, Lua 5.4 with vendored, serde,
and macros features. Other Lua backends, Miri, and the full upstream test suite
were not run. The environment control specifically concerns Lua 5.4.
All probes were small, local, synchronous functional API calls.

Known-defect exclusion review: none of the controls concerns invalid raw_set
keys, table-to-slice equality, integer-set round trips, or mixed integer/float
Value equality. The table probe uses valid integer insertion positions and
collects values into a Rust Vec for comparison. The integer probe concerns
checked conversion to u8, not Value equality.
