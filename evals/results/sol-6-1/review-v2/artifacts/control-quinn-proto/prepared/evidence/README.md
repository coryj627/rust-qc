Evidence for quinn-proto 0.11.13 controls

- Baseline: /work/crate; audited paths are under quinn-proto/src.
- Scratch workspace: /work/tmp/control-probe, copied from baseline.
- Toolchain used: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 (5f94df478 2026-08-27), aarch64 Linux.
- Features: package defaults; locked offline dependencies; RUSTUP_AUTO_INSTALL=0.
- Only test-module declarations were appended to six scratch source files. Production functions and manifests were not edited. The complete added test sources are in probes/.
- prepare_probes.py recreates those declarations from baseline and the preserved probe sources. It never writes into /work/crate.
- Executed preparation: python3 /work/prepared/evidence/prepare_probes.py
- Executed regression command: cd /work/tmp/control-probe && RUSTUP_AUTO_INSTALL=0 cargo test --locked -p quinn-proto negative_control_ -- --nocapture
- Result: seven tests passed; zero failed; 263 existing tests filtered out. Compilation output is in build.log; test output and command are in probes.log. The logs are two portions of the same execution, not separate runs.
- Two probes use the existing in-memory connection-pair harness and public APIs. Five directly exercise production helpers inside test modules. No network sockets, external traffic, or harmful external behavior were used by these probes.
- source-excerpts.txt preserves numbered baseline source supporting all seven refutations.
- baseline-sha256.json records every baseline file before probing; validation.log records the final unchanged-baseline check and structural/source-reference validation.
- write_controls.py preserves the construction of controls.json; validate.py checks it.

Limits: This is seven targeted functional regressions with source confirmation, not an exhaustive correctness assessment. The complete existing suite and Miri were not run. Idle-timeout negotiation was tested directly, not by simulating a full expiration. No other feature combinations or architectures were exercised.
