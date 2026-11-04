Found **10 reproduced correctness issues and 3 build/documentation issues**. Release sources were unchanged. All behavioral reproductions are in [review_regressions.rs](/work/tmp/review/tests/review_regressions.rs), added only to a scratch copy.

1. **Raw-copying ZipCrypto entries produces unreadable output** — [src/write.rs:1316](/work/crate/src/write.rs:1316), [src/read.rs:1833](/work/crate/src/read.rs:1833).  
   Raw copying promises to preserve metadata and compressed contents. The options copied from the source omit ZipCrypto encryption metadata, so the destination marks encrypted bytes as plaintext. Reproduced with a five-byte stored entry: `encrypted()` becomes false, and reading with the original password returns 17 bytes followed by `Invalid checksum`. Evidence: `raw_copy_preserves_zipcrypto` and `raw_copy_zipcrypto_stays_readable`.

2. **Streaming ZipCrypto archives reject their correct password** — [src/zipcrypto.rs:168](/work/crate/src/zipcrypto.rs:168), [src/read.rs:412](/work/crate/src/read.rs:412).  
   Writer-produced encrypted archives should reopen with their password. `new_stream` sets the data-descriptor flag, which makes readers validate against modification time, but the writer always stores the CRC check byte. `finish()` succeeds; reopening a five-byte entry with the correct password returns `InvalidPassword`. Evidence: `streaming_zipcrypto_roundtrip`, using a fixed modification time.

3. **`finish_into_readable` misreports ZIP64 sizes and reads beyond file contents** — [src/write.rs:2078](/work/crate/src/write.rs:2078).  
   The returned archive should behave like `ZipArchive::new(writer.finish()?)`. Header updating replaces both in-memory sizes with `0xffffffff`, and the direct reader reuses those values. A five-byte stored file with `large_file(true)` reports 4,294,967,295 bytes; reading returns 94 bytes, including archive metadata, then fails its checksum. Reopening the serialized archive works correctly. Evidence: both `readable_zip64_*` tests and `ordinary_finish_controls`.

4. **`finish_into_readable` cannot read AES entries** — [src/write.rs:777](/work/crate/src/write.rs:777), [src/read.rs:537](/work/crate/src/read.rs:537).  
   Writer metadata retains compression method `Aes`; normal archive parsing replaces that marker with the underlying compression method. The direct construction skips this conversion. Opening an entry with the correct password returns `UnsupportedArchive("Compression method not supported")`. The equivalent serialized archive reopens successfully. Evidence: `readable_aes_roundtrip` and `ordinary_finish_controls`.

5. **Raw copying loses required Implode compression flags** — [src/types.rs:849](/work/crate/src/types.rs:849).  
   Raw copying should preserve the information required to decode unchanged compressed bytes. Header serialization reconstructs flags without Implode’s dictionary/literal-tree bits. The existing `tests/data/legacy/implode.zip` fixture has flags `6` and reads successfully before copying; the copied entry fails with `too many codeword lengths`. Evidence: `raw_copy_legacy_archives`, with `--all-features`.

6. **Raw copying converts symlinks into regular files** — [src/read.rs:1836](/work/crate/src/read.rs:1836).  
   The raw-copy contract says metadata is copied. Passing the original mode through `unix_permissions` strips its file-type bits, and normalization assigns regular-file type. A source entry created with `add_symlink` has `is_symlink() == true`; its raw copy returns false. Ordinary extraction consequently materializes the target text as a regular file. Evidence: `raw_copy_preserves_symlink`.

7. **Streaming readers silently omit central-directory metadata callbacks** — [src/read/stream.rs:43](/work/crate/src/read/stream.rs:43).  
   `visit` documents visiting files and their metadata. `read_zipfile_from_stream` consumes 30 bytes when recognizing the first central-directory header; parsing then resumes partway through that header, and its error is discarded by `while let Ok(...)`. A valid two-file archive produces two file callbacks and zero metadata callbacks, while returning success. Evidence: `stream_visits_all_metadata`. Existing assertions inside metadata callbacks do not detect callbacks being skipped entirely.

8. **`data_start()` panics on streamed files** — [src/read.rs:1818](/work/crate/src/read.rs:1818).  
   `read_zipfile_from_stream` explicitly documents an unavailable data offset as zero. Its metadata leaves the `OnceLock` empty, while `data_start()` unconditionally unwraps it. Calling this accessor on an ordinary valid streamed entry panics. Evidence: `streamed_file_data_start_is_documented_zero`.

9. **ZIP64 comments disappear when reopening small archives** — [src/spec.rs:365](/work/crate/src/spec.rs:365), [src/write.rs:1567](/work/crate/src/write.rs:1567).  
   A comment supplied through `set_zip64_comment` should survive serialization. The writer emits ZIP64 records for the comment, but the reader only checks for them when selected ZIP32 fields contain sentinel values. A small one-file archive reopens with `zip64_comment() == None`. Evidence: `zip64_comment_roundtrip`.

10. **Oversized entry names panic instead of returning `ZipError`** — [src/types.rs:902](/work/crate/src/types.rs:902).  
    A name exceeding the ZIP field’s capacity should produce a recoverable validation error. `start_file` instead unwraps the failed conversion to `u16`. The 65,535-byte boundary passes; 65,536 bytes panics with `TryFromIntError(PosOverflow)`. Evidence: `maximum_name_length_control` and `too_long_name_returns_error`. The latter catches the panic and deliberately forgets the partially initialized writer.

11. **The advertised docs.rs configuration fails on the installed nightly** — [src/lib.rs:32](/work/crate/src/lib.rs:32).  
    Documentation builds using the package’s `--cfg docsrs` setting should compile. `feature(doc_auto_cfg)` was removed in Rust 1.92. Executing the advertised configuration with `nightly-2026-10-01` fails with E0557 before documentation generation.

12. **Strict Clippy fails on Rust 1.99.1** — [src/write.rs:2173](/work/crate/src/write.rs:2173).  
    `cargo clippy --locked --all-features --all-targets -- -D warnings` fails on `clippy::collapsible_match`. This is a pre-existing lint failure on that toolchain, not a behavioral defect.

13. **Public documentation contains four broken links** — [src/read.rs:1617](/work/crate/src/read.rs:1617), [src/spec.rs:95](/work/crate/src/spec.rs:95), [src/spec.rs:143](/work/crate/src/spec.rs:143).  
    The `PathBuf::Component::ParentDir`, `ZipWriter`, and `ZipWriter::finish` links do not resolve. Normal documentation generation succeeds with four warnings; `RUSTDOCFLAGS='-D warnings'` makes it fail. This breaks navigation and strict documentation checks.

Checks ran from `/work/crate` with `RUSTUP_AUTO_INSTALL=0`, using Rust 1.99.1 on aarch64 Linux unless specified otherwise:

| Command | Outcome |
|---|---|
| `cargo test --locked --all-features` | 136 unit and 42 integration tests passed; nine doctests passed; the known 4 GiB doctest was killed by SIGKILL. |
| `cargo test --locked --no-default-features` | Unit/integration tests passed; the same doctest was killed. |
| `cargo test --locked --lib --tests` | Passed: 126 unit, 39 integration tests. |
| `cargo test --locked --no-default-features --lib --tests` | Passed: 87 unit, 19 integration tests. |
| Above command with `--features deflate-flate2-zlib-rs` | Passed: 92 unit, 26 integration tests. |
| Above command with `--features deflate-zopfli` | Passed: 88 unit, 19 integration tests. |
| `cargo test --locked --all-features --doc -- --skip spec::ZIP64_BYTES_THR` | Nine passed; one explicitly skipped. |
| Same filtered doctest command with `--no-default-features` | Nine passed; one skipped. |
| `cargo check --locked --all-features --all-targets` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | Failed as finding 12. |
| `cargo doc --locked --all-features --no-deps` | Succeeded with four warnings; failed with `RUSTDOCFLAGS='-D warnings'`. |
| `RUSTDOCFLAGS='--cfg docsrs' cargo +nightly-2026-10-01 doc --locked --all-features --no-deps` | Failed as finding 11. |

In `/work/tmp/review`, `cargo test --locked --all-features --test review_regressions -- --nocapture` ran **17 tests: four passed and 13 failed**. Two findings have multiple failing assertions/tests. One additional failure records streaming AES finishing with `seek is not supported`; I excluded it from the finding count because `new_stream` documents errors for operations requiring earlier bytes to be overwritten.

The source findings above were corroborated by executed regressions or checks; explanations of their mechanisms come from source inspection. The memory-killed doctest is an environment limitation, not evidence of a code defect. Nightly Clippy and rustfmt were attempted but unavailable as installed components. MSRV 1.83, Miri, other platforms, and exhaustive feature combinations were not checked. No dedicated security testing was performed.

**Finding count: 13 — ten correctness issues and three build/documentation issues. Last `/work/clock` reading: 8.19735 minutes used of 45.0.**