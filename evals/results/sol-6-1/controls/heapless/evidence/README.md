Evidence for heapless 0.9.1 controls

- `controls_probe.rs`: seven independent public-API regression probes. Copied verbatim into `/work/tmp/heapless-controls/tests/controls_probe.rs`; all pre-existing scratch files match the original crate byte for byte.
- `probes-stable.log`: captured compiler output, observations for every control, and the successful seven-test result.
- `source-excerpts.txt`: line-numbered guards and implementations read directly from `/work/crate`.
- `environment.txt`: actual compiler, Cargo, and architecture output.
- `baseline-sha256.json`: hashes of all 53 original crate files, recorded before building.
- `validate.py` and `validation.log`: reproducible count/schema/reference checks, checks for executed probes in the log, original-crate integrity, and scratch-baseline identity.

Executed probe command (stdout and stderr captured in `probes-stable.log`):

```sh
CARGO_HOME=/work/.cargo-home RUSTUP_AUTO_INSTALL=0 CARGO_TARGET_DIR=/target/heapless-controls cargo test --offline --locked --manifest-path /work/tmp/heapless-controls/Cargo.toml --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes --test controls_probe -- --nocapture --test-threads=1
```

Executed artifact validation command:

```sh
python3 /work/prepared/evidence/validate.py
```

Scope: targeted debug-mode probes on aarch64 Linux with stable Rust 1.99.1, combined with inspection of the exact baseline guards. The full upstream test suite and Miri were not run. Queue probes are sequential operations through split producer/consumer handles; their claim concerns index arithmetic rather than concurrent scheduling. Probe coverage is finite; the accompanying source invariants are the decisive general refutations. All thirteen supplied known-defect descriptions were read to avoid overlap; no candidate relies on changing or exercising them.
