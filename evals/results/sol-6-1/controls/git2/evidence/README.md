The controls apply to the supplied git2 0.20.2 baseline at /work/crate.
All claim text is separated from refutation text in ../controls.json.

- control_probes.rs: seven safe public-API regression probes, one per control.
- probes.log: complete captured Cargo compilation and test output.
- probes-summary.log: concise test output, including each probe's observations.
- source-excerpts.txt: numbered baseline source establishing the decisive guards and ownership invariants.
- baseline-sha256.json: inventory captured before copying or probing the baseline.
- validate_controls.py and validation.log: structural checks, source-reference checks, baseline integrity verification, scratch-copy comparison, and recorded probe-result checks.

Executed probe command:

    cd /work/tmp/git2-control-probe && RUSTUP_AUTO_INSTALL=0 cargo test --locked -p git2 --no-default-features --test control_probes -- --nocapture --test-threads=1

The scratch directory contains a complete baseline copy with only tests/control_probes.rs added. Target output is under /target. No original crate files were edited. No network operations, installations, external repositories, or harmful external triggers were used. The treebuilder test intentionally panics inside a guarded callback and catches the resumed panic in Rust; its panic-hook output is expected.

All seven probes passed. Runtime coverage is aarch64 Linux with stable Rust 1.99.1, vendored libgit2 fallback, and no default features. No Miri, other-platform, SSH, HTTPS, or full-suite run was performed. The source proofs, rather than the absence of a crash alone, establish the ownership and panic invariants. Known-defect descriptions were used only to exclude those real defects; all seven controls address separate APIs/mechanisms.
