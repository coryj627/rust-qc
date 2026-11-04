Update on the last open check: the Tree Borrows Miri rerun (`MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test --features sync --lib -- common:: cht::`) hit the background time limit and was stopped before it printed anything. It produced no result either way, so the Miri line in the report's Verification section should read:

- `cargo +nightly miri test --features sync --lib -- common:: cht::`:
  - **Stacked Borrows:** failed, but the violation is inside crossbeam-epoch 0.9.21's own list code, not in moka's code.
  - **Tree Borrows:** the rerun was stopped at the time limit with no output, so it is inconclusive. This is not a pass.

None of the findings change, since they all came from the probes and from reading the code, not from Miri. Miri has still not shown whether moka's own `unsafe` code is sound. The best next step is to rerun Tree Borrows one test module at a time (for example `common::deque`, then `common::timer_wheel`) so each run finishes within the time limit. I didn't restart it because the audit was already finished.

Last `./clock` reading: 5 minutes used of 45.
