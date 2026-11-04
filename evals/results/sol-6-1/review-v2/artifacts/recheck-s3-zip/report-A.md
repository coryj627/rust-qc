Found **19 findings**. The most consequential are corruption of existing entries during append/merge, lossy raw copying, and unreadable results from `finish_into_readable()`.

All behavior findings below were reproduced with local regression tests against unchanged source. Test names refer to [review_regressions.rs](/work/tmp/review/tests/review_regressions.rs), unless otherwise specified.

1. **Append and merge lose required ZIP64 offsets — [src/write.rs:2094](/work/crate/src/write.rs:2094).** Existing entries should remain readable. Parsing removes ZIP64 extra fields, but central-directory writing does not regenerate them. A five-byte entry at offset `0x100000000` reads correctly initially; after append or merge, reading fails with `Invalid local file header`. Appending in place can therefore make existing contents inaccessible. Evidence: `append_preserves_zip64_header_offset`, `merge_generates_zip64_header_offset`; both used sparse files.

2. **Append and merge discard Implode decoding flags — [src/types.rs:864](/work/crate/src/types.rs:864).** Rewriting metadata should preserve flags needed to decode unchanged contents. `flags()` reconstructs only encryption, UTF-8 and descriptor bits, dropping Implode’s dictionary/tree flags. The existing `legacy/implode.zip` fixture reads successfully before rewriting; both a no-op append and a merge produce `too many codeword lengths`. Evidence: `append_preserves_implode_flags`, `merge_preserves_implode_flags`, with `legacy-zip` enabled.

3. **Raw copying removes the ZipCrypto encrypted flag — [src/read.rs:1832](/work/crate/src/read.rs:1832).** A raw copy should preserve the interpretation of its unchanged bytes. `options()` preserves AES metadata but loses ZipCrypto encryption state. Copying an untouched `by_index_raw()` entry succeeds, but the result reports `encrypted() == false` and fails with `Invalid checksum`. Evidence: `raw_copy_zipcrypto`, `raw_copy_zipcrypto_payload`.

4. **Raw copying a freshly opened decrypted AES entry silently produces an unreadable copy — [src/write.rs:1336](/work/crate/src/write.rs:1336).** The API should copy the complete entry or reject a reader whose required bytes have already been consumed. Password validation consumes the AES prefix before the caller reads anything; `take_raw_reader()` cannot recover it, and the copied byte count is not checked. Copying a fresh `by_name_decrypt()` result returns success, but reopening the destination returns `InvalidPassword`. Evidence: `raw_copy_decrypted_aes_file`. Existing copying through `by_index_raw()` passes.

5. **Raw copying loses symlink type and extra fields — [src/write.rs:1316](/work/crate/src/write.rs:1316).** The raw-copy documentation promises copied metadata. Reconstructing entries through `SimpleFileOptions` strips file-type bits and extra fields. A copied symlink becomes a regular file containing its target text; a custom `0xbeef` field disappears. Callers copying directory trees or metadata-bearing archives lose their original meaning. Evidence: `raw_copy_symlink`, `raw_copy_metadata`.

6. **`finish_into_readable()` exposes ZIP64 sentinels as actual sizes — [src/write.rs:2078](/work/crate/src/write.rs:2078).** The returned archive should behave like reopening `finish()` output. Finalization replaces the in-memory sizes with `4294967295`, then hands that metadata directly to the reader. A five-byte Stored entry written with `large_file(true)` reports both sizes incorrectly and fails with `Invalid checksum` when read to completion. Evidence: `zip64_finish_into_readable`, `zip64_finish_into_readable_payload`; `zip64_finish_reopen_control` passes.

7. **`finish_into_readable()` cannot read AES entries — [src/write.rs:785](/work/crate/src/write.rs:785).** The returned archive should support the same reads as reparsed output. Writer metadata retains `CompressionMethod::Aes`, whereas normal parsing substitutes the underlying compression method. Finalization succeeds, but `by_name_decrypt()` returns `Compression method not supported`. Evidence: `aes_finish_into_readable`; `aes_finish_reopen_control` passes.

8. **Valid Unicode comments can prevent opening an entire archive — [src/read.rs:1530](/work/crate/src/read.rs:1530).** The Unicode-comment checksum must use the original comment bytes. The implementation checks the already-decoded UTF-8 string instead. An archive with CP437 `caf\x82` and a correctly checksummed Unicode comment `café` is rejected as `Could not find EOCD`. Evidence: `unicode_comment_with_cp437_original`; Python’s `zipfile` successfully reads the fixture and reports no CRC failure.

9. **An overlong filename aborts the process instead of returning an error — [src/types.rs:902](/work/crate/src/types.rs:902).** A filename exceeding the ZIP format’s `u16` byte limit should produce a `ZipError`. A 65,536-byte name panics during conversion; unwinding then triggers another panic in central-directory writing at [src/types.rs:934](/work/crate/src/types.rs:934). Ordinary writer cleanup therefore aborts the process. Evidence: `too_long_filename_returns_error`; the standalone [review_long_name example](/work/tmp/review/examples/review_long_name.rs) exited **134** with “panic in a destructor during cleanup.”

10. **Stream visitors silently receive no central-directory metadata — [src/read/stream.rs:48](/work/crate/src/read/stream.rs:48).** `visit()` promises file contents and subsequent metadata. `read_zipfile_from_stream()` consumes 30 bytes of the first central header before returning `None`; metadata parsing starts at the wrong position, and its error is discarded. A two-entry archive returns success with two file callbacks and zero metadata callbacks. Comments and permission restoration are consequently lost. Evidence: `stream_metadata_count`.

11. **Extraction never restores directory permissions — [src/read.rs:898](/work/crate/src/read.rs:898).** Directories made writable during extraction should receive their archived permissions afterward, consistent with the implementation’s deferred-permission design. Directory entries `continue` before being added to that deferred list. An archived directory with mode `0500` extracts successfully as `0755`. Evidence: `extract_directory_permissions`, on Linux.

12. **Seekable entry reads return encrypted bytes as file contents — [src/read.rs:1094](/work/crate/src/read.rs:1094).** A normal entry-reading API should return decoded contents or an unsupported/password-required error. `by_index_seek()` checks compression but ignores encryption. For a Stored ZipCrypto entry containing five plaintext bytes, `by_name_seek()` returns 17 encrypted bytes, while `by_name()` correctly requires a password. Evidence: `seek_api_encrypted_entry`. This finding concerns incorrect returned data.

13. **Directory lookup through the Path API fails — [src/read.rs:1068](/work/crate/src/read.rs:1068).** An entry created using `add_directory_from_path("dir/subdir/")` should be retrievable using that same path. Normalization removes the trailing slash, so `by_path()` searches for a different name and returns `FileNotFound`; `by_name("dir/subdir/")` succeeds. Evidence: `directory_by_path`.

14. **Streaming `data_start()` panics despite documenting zero — [src/read.rs:1818](/work/crate/src/read.rs:1818).** The streaming-reader documentation explicitly specifies `data_start == 0`. The associated `OnceLock` remains empty, and the accessor unconditionally unwraps it. Reading this documented metadata field therefore panics on an ordinary streamed entry. Evidence: `stream_data_start`.

15. **Stream writers omit required descriptors for raw copies and directories — [src/write.rs:1088](/work/crate/src/write.rs:1088), [src/write.rs:1451](/work/crate/src/write.rs:1451).** Entries marked with general-purpose bit 3 require data descriptors. Both paths set that flag but bypass descriptor writing. Evidence: `stream_raw_copy`, `stream_directory_descriptor`; `stream_regular_file_descriptor_control` passes. The raw-copy result remains readable through this crate’s central-directory reader; the confirmed defect is inconsistent ZIP structure, with interoperability consequences for readers relying on descriptors.

16. **Access-only timestamps are misreported as modification times — [src/extra_fields/extended_timestamp.rs:51](/work/crate/src/extra_fields/extended_timestamp.rs:51).** A valid local field containing only an access timestamp should expose it through `ac_time()`. For flags `0x02`, length 5 and timestamp 42, the parser instead returns `mod_time() == Some(42)` and `ac_time() == None`. Callers receive the wrong timestamp category. Evidence: `access_only_extended_timestamp`.

17. **The documented Bzip2 default is incorrect — [src/write.rs:430](/work/crate/src/write.rs:430), [src/compression.rs:234](/work/crate/src/compression.rs:234).** Documentation promises Bzip2 when enabled without Deflate. Actual defaults use Stored whenever Deflate is absent, so callers relying on the documented default receive uncompressed output. Evidence: [review_bzip2_default.rs](/work/tmp/review/tests/review_bzip2_default.rs), executed with `--no-default-features --features bzip2`.

18. **Strict Clippy fails on the supplied toolchain — [src/write.rs:2173](/work/crate/src/write.rs:2173).** Rust/Clippy **1.99.1** rejects the nested condition with `clippy::collapsible_match` under `-D warnings`, for both all-features and no-default-features configurations. This blocks the project’s strict lint command. Allowing only that lint makes the all-features/all-targets check pass.

19. **Documentation contains broken links and fails strict rustdoc — [src/read.rs:1617](/work/crate/src/read.rs:1617), [src/spec.rs:95](/work/crate/src/spec.rs:95), [src/spec.rs:143](/work/crate/src/spec.rs:143).** All-features documentation has four unresolved links. No-default-features adds unresolved `BufWriter` links at [src/write.rs:660](/work/crate/src/write.rs:660) and line 661. Normal all-features documentation builds with warnings; `RUSTDOCFLAGS='-D warnings'` makes both configurations fail.

Validation used Rust/Cargo **1.99.1**, on `aarch64-unknown-linux-gnu`, with `RUSTUP_AUTO_INSTALL=0`. Commands ran from `/work/crate` unless stated otherwise.

| Command | Outcome |
|---|---|
| `cargo test --locked --all-features` | 178 unit/integration tests and 9 doctests passed. The known 4 GiB doctest was killed by SIGKILL; command exited 101. |
| `cargo test --locked --no-default-features` | 106 unit/integration tests and 9 doctests passed; same memory-blocked doctest. |
| `cargo test --locked -- --skip spec::ZIP64_BYTES_THR` | 174 passed; one filtered out. |
| `cargo test --locked --no-default-features --features deflate-flate2-zlib-rs -- --skip spec::ZIP64_BYTES_THR` | 127 passed; one filtered out. |
| `cargo test --locked --no-default-features --features deflate-zopfli -- --skip spec::ZIP64_BYTES_THR` | 116 passed; one filtered out. |
| `cargo test --locked --all-features --doc -- --skip spec::ZIP64_BYTES_THR` | 9 passed; one filtered out. |
| `cargo check --locked --all-features --all-targets` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | Failed as finding 18. The equivalent no-default-features command also failed. |
| Same all-features Clippy command with `-A clippy::collapsible_match` appended | Passed. |
| `cargo doc --locked --no-deps --all-features` | Passed with four warnings. |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps --all-features` | Failed with four broken links; no-default-features failed with six. |

Scratch verification ran from `/work/tmp/review`:

- `cargo test --locked --all-features --test review_regressions -- --nocapture`: **22 failed assertions and 5 passing controls/documented-behavior checks**.
- `cargo test --locked --no-default-features --features bzip2 --test review_bzip2_default`: one failed assertion.
- `cargo run --locked --all-features --example review_long_name`: exit 134.

Only the scratch copy received two test files, one generated ZIP fixture and one example. `diff -qr /work/crate/src /work/tmp/review/src` confirmed identical source trees. Logs remain under `/work/tmp/`. No production fixes were made.

**19 findings.** Coverage emphasized read/write round trips, metadata preservation, streaming and feature configurations. The memory-heavy doctest remains environment-blocked; its SIGKILL is not treated as a defect. MSRV 1.83, Miri, docs.rs configuration and other platforms were not checked. No dedicated security assessment was performed. Last clock reading: **769.5 seconds — 12.8206 of 45 minutes used**.