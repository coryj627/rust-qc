# Evaluations

Answer-key evaluation scores audit findings against real bugs that upstream fixed after a chosen release. Issue #2 chose the projects, and issues #12 to #21 track each one.

- `candidates.md`: the 10 projects and how they were chosen, with results.
- `keys/`: one answer key per project, in the format described in `candidates.md`.
- `repro/`: the reproducer for each key bug and its recorded output. `repro/rerun.tsv` holds the clean re-run results.
- `sandbox/`: the container used to build and run upstream code.

Never give an audit agent this folder. A workspace for an audit holds only the crate at its baseline tag.

## Running a reproducer

Building or testing these crates runs their build scripts, proc macros and tests, so do it only through `sandbox/sb`. It runs with no network, as non-root, with a read-only root filesystem, no capabilities, resource limits, and only one project folder mounted.

```bash
docker build -t rqc-eval:1 evals/sandbox
```

`sb` keeps per-project folders next to itself, under `projects/<project>/`, mounted as `/work`. Each reproducer expects:

- `/work/repro/`: a copy of `repro/<project>/`
- `/work/base/`: the crate at its baseline tag, from `git archive`. Where the tag has no lockfile, copy `repro/<project>/base-Cargo.lock` to `/work/base/Cargo.lock`.
- dependencies fetched first with `sb fetch <project> sh -c 'cd /work/base && cargo fetch'`

Then:

```bash
sb run <project> sh /work/repro/<bug-id>/run.sh
```

It exits 0 when it has demonstrated the bug. A key's `fix_control` field says how that bug was checked against the fix; each project's scripts name the folders that needs, such as `/work/fixes/`.

A few projects need more:

- **git2:** libgit2 is a submodule, so fetch it into `base/libgit2-sys/libgit2`.
- **quinn-proto:** copy `base` to `reprotree` and apply `repro/quinn-proto/reprotree.patch`, which adds the tests.
- **simd-json:** some entries use an x86_64 Miri sysroot at `/work/miri-sysroot-x86`.

The recorded re-run used the original working folders with build output removed. These setup steps have not been tested from a fresh clone.
