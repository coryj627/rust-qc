Final artifact: /work/prepared/controls.json
Baseline: heapless 0.9.1 in /work/crate, audited source under src/.
Seven final controls: 2 defect-key decoys and 5 verifier findings.
Every final claim has a passing ordinary public-API counterexample and decisive baseline source references.

Probe source: control_probes.rs
Probe log: probes.log (7 passed, 0 failed)
Execution: scratch copy /work/tmp/heapless-controls; original crate was never edited.
Toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 (5f94df478 2026-08-27), aarch64 Linux.
Command (identical for all seven refutations; all tests ran together):
cd /work/tmp/heapless-controls && RUSTUP_AUTO_INSTALL=0 CARGO_HOME=/work/.cargo-home CARGO_TARGET_DIR=/target/heapless-controls cargo test --offline --locked --features=alloc,defmt,mpmc_large,portable-atomic-critical-section,serde,ufmt,bytes --test control_probes -- --nocapture --test-threads=1 > /work/prepared/evidence/probes.log 2>&1

Scope: small safe functional regression probes in the supplied sandbox, stable debug profile, supplied feature set. No complete upstream test-suite run or Miri run is claimed. No security assessment was performed. The known-defect list was read to exclude its IndexMap and extreme-capacity SPSC defects; none of the final controls concerns those defects.

baseline-sha256.json records original file hashes before any probes; validation.log records final integrity/schema/reference checks. write_controls.py preserves artifact construction. The discarded/ directory preserves an exploratory candidate run that was excluded and a probe compilation error; neither is evidence for a final control.
