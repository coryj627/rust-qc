# Audit report: zip 5.1.1

**Method.** I read all of `crate/src/` except the test modules. I ran probes through `./run` from a throwaway crate at `probe/`, in the workspace. `crate/` was not changed. Unless a finding says otherwise, "probe" means a handcrafted or crate-written archive run in that binary, debug build, with the observed output quoted.

**Baseline.** `cargo test --locked --all-features --lib --tests` passes, so none of these are caught by the existing tests. I did not run the doctests.

**Other checks that found nothing further:**
- Mutation fuzzing of 10 sample archives. Three were capped: `legacy/implode.zip` and `lzma.zip` ran 1,500 mutations, and `deflate64.zip` hit the 60- and 100-second limits with no result, so its only completed evidence is a 40-iteration timing run with no panics.
- A model-based random test of writer operations.
- A feedback-guided search for Shrink decoder panics.
- A comparison of the CP437 table against Python's.
- `cargo check` across feature sets. The only failure was `deflate-flate2` alone, which is the documented "choose your own backend" case.

No text in the repository addressed me.

## High

**1. A 300-byte archive aborts the process during `extract` — `read.rs:894`**
- `Vec::with_capacity(file.size() as usize)` trusts the declared size of a symlink entry.
- Probe: a symlink entry with ZIP64 size `1<<62` gave `memory allocation of 4611686018427387904 bytes failed`, `Aborted`, exit 134. The caller cannot catch this.

**2. Legacy decoders abort on a declared size (`legacy-zip` feature, not on by default) — `legacy/shrink.rs:340,227`, `legacy/implode.rs:214,97`, `legacy/reduce.rs:107`**
- `reserve(uncompressed_size)` runs on the first read.
- Probe: methods 1, 2 and 6 with a declared size of `1<<62` all aborted the same way (exit 134).

**3. Stack overflow in the Shrink decoder (`legacy-zip`) — `legacy/shrink.rs:141`**
- `read_code` recurses once per control code.
- Probe: 450 KB of repeated `256,2` codes gave `has overflowed its stack`, exit 134, in both debug and release.

**4. Prepended data can make `ZipArchive::new` silently return another archive's contents — `spec.rs:681-699`, `read.rs:660-665`**
- The archive offset comes from the first `PK\1\2` found forward from the relative central directory offset, not from the real CD position.
- If that scan lands in the last `prefix_len` bytes of stored data, the outer directory parse goes wrong. The loop then falls back to an earlier EOCD.
- Probe: an archive holding `outer.txt` and a stored `nested.jar`, with a 300-byte prefix, listed only `["inner.txt"]`, `offset=384`, and read `INNER`. The same archive without the prefix listed both outer entries. A self-extracting archive or polyglot therefore shows different contents from other tools.

**5. AES authentication is skipped on empty and truncated entries — `aes.rs:171-172`, `aes.rs:178-188`**
- The HMAC is only checked when `data_remaining` reaches 0 during a read that had data. An empty entry returns early, and a short underlying read returns `Ok(0)` with bytes outstanding.
- AE-2 has no CRC, so nothing else catches this.
- Probes:
  - Every auth-code byte flipped on an empty entry gave `Ok(0)`. The control, a tampered non-empty entry, correctly gave `Invalid authentication code`.
  - An AE-2 entry placed after the EOCD with its last 5 bytes and auth code removed gave `Ok(5)` and `"01234"`.

**6. Appending drops all ZIP64 records, corrupting large archives — `write.rs:2094-2105`, with stripping at `read.rs:1396-1398`**
- The reader removes the ZIP64 extra field. On rewrite, `block()` clamps values to `0xFFFFFFFF` and nothing writes the field back.
- Probe: a sparse file with an entry at 5 GiB read fine before `new_append`. Afterwards the old entry failed with `Invalid local file header`.
- By reading only: `merge_archive` shares this path.

**7. Appending or raw-copying breaks Imploded entries — `types.rs:849-865`, `types.rs:919`, `types.rs:715`**
- `flags()` keeps only bits 0, 3 and 11, but Implode needs bits 1 and 2.
- Probe with `tests/data/legacy/implode.zip`: flags went from `0x6` to `0x0`, and reads then failed with `i/o error`, both after `new_append` and after `raw_copy_file`.

## Medium

**8. `raw_copy_file` loses metadata, despite the doc saying "Any `ZipFile` metadata is copied" — `read.rs:1832-1851`, `types.rs:716`, `write.rs:477`, `write.rs:920`**
- ZipCrypto: the copy has `encrypted=false`, and `by_index` returns ciphertext and then `Invalid checksum`.
- Symlinks become regular files (`symlink=false`, mode `100777`).
- The file comment and extra fields are dropped.
- Declared size 5 GiB became `dest size=0`, because the ZIP64 field is written with zeros and never updated.

**9. `large_file` entries are corrupted in memory after writing — `write.rs:2078-2079`, `write.rs:684-687`**
- Sizes are overwritten with `0xFFFFFFFF` after finishing.
- `finish_into_readable` reported `csize=4294967295`, read 96 bytes and returned `Invalid checksum`.
- `deep_copy_file` on a `large_file` entry that isn't last copies the following entries too. Both source and copy then read `helloPK...world...` with `csize=58`, and the read fails.
- With `large_file` removed, the model-based writer test ran 1,500 seeds clean.

**10. `finish_into_readable` cannot read AES entries — `write.rs:957-961`, `read.rs:1184`, `compression.rs:460`**
- The method stays `Aes`, so the result is `UnsupportedArchive("Compression method not supported")`.
- The same bytes through `ZipArchive::new` work and report `Deflated`.

**11. `extract` keeps setuid and setgid bits, and can set mode 000 — `read.rs:941-943`, `types.rs:622`**
- Probe: an entry with mode `0o106755` extracted as `106755`.
- A Unix-host entry with only a DOS attribute (`0x20`) extracted as `100000`, unreadable to its owner.

**12. `ZipStreamReader` never delivers central directory metadata — `read.rs:1944-1948`, `read/stream.rs:48`, `types.rs:832`, `read/stream.rs:70`**
- Detecting the end of entries consumes 30 bytes of the first central header, so the next parse fails and `while let Ok` exits silently.
- Probe: `files=3 metadata_calls=0`. `extract` left a `0o755` file at `100644` and a symlink as a regular file. `ZipArchive::extract` on the same bytes gave `100755` and a real symlink.

**13. Symlink behaviour contradicts the `extract` doc — doc at `read.rs:768-770`, code at `read.rs:907-908`, `read.rs:1718-1723`, `path.rs:13`**
- The doc says links are only "created and followed if the target is within the destination".
- Targets are never checked at creation. Probe: `evil -> /etc/passwd` was created, and 878 bytes could be read through it.
- Following always fails, because `simplified_components` rejects the absolute base path.
  - An in-tree `link -> real` plus `link/f` gave `Invalid base path`.
  - Re-extracting any archive containing a symlink fails, despite "overwriting files if they already exist".
- I found no way to write outside the destination.

**14. Streaming writer defects (`new_stream`)**
- ZipCrypto entries can't be decrypted with the correct password: `InvalidPassword`. The writer stores the CRC check byte (`zipcrypto.rs:168`), but the reader checks the time byte when bit 3 is set (`read.rs:412-415`).
- Directories and raw copies set the data-descriptor flag (`write.rs:1009`) but write no descriptor (`write.rs:1451`, `write.rs:1088`). Probe: flags `0x8`, and the next bytes are `PK\3\4`, so sequential readers would misparse. Central-directory reads still work.
- The check is inverted at `write.rs:2049-2057`. In release, a 4097 MiB entry with `large_file(true)` failed with `Large file option has not been set`. By reading only: a non-large entry with compressed size over 4 GiB gets a truncated `u32` descriptor.
- AES always fails at finish with `seek is not supported`, because `update_aes_extra_data` seeks (`write.rs:1117`, `write.rs:2019`).

**15. Writer panics on bad input, then aborts in `Drop` — `types.rs:902,934,938`, `write.rs:1670`**
- `start_file` with a 70,000-byte name panics at `types.rs:902`. `Drop` panics again at `types.rs:934`, giving `panic in a destructor during cleanup`, exit 134. It should return `Err`.
- The same abort comes from input: a 40,000-byte CP437 comment expands past 65,535 bytes, so `new_append` then `finish` aborts at `types.rs:938`.

**16. `new_append` rewrites other metadata — `types.rs:849-865`, `types.rs:917`, `write.rs:2107`**
- Comments are re-encoded to UTF-8 but the UTF-8 flag follows only the name. Both the UTF-8 and CP437 `café` comments became `caf├⌐`.
- Probe: a Darwin host byte (19) became 4.
- A no-op append grew the file from 294 to 458 bytes because the old central directory stays in place (`write.rs:635-650`).

**17. A deep copy after append writes a malformed local header — `write.rs:712`, `types.rs:868`**
- Probe: a `large_file` entry, `new_append`, then `deep_copy_file` gave a local header with both sizes `ff ff ff ff` and `extra_len=0`.
- Central-directory reads still work. By reading only: sequential readers would get a wrong size.

**18. Valid archives are rejected over optional extra fields, with a misleading error — `read.rs:1528-1530`, `extra_fields/zipinfo_utf8.rs:20-22`, `extra_fields/ntfs.rs:33-53`, `read.rs:660-665`**
- The 0x6375 comment CRC is checked against the already decoded comment instead of the raw bytes. A correct field on a CP437 comment is rejected.
- A stale 0x7075 CRC fails the whole archive, where the Info-ZIP format (from my own knowledge, not checked in this workspace) says to ignore the field.
- NTFS fields with a second or different attribute tag fail the whole archive.
- All four probes returned `InvalidArchive("Could not find EOCD")`, because the real error is swallowed.

**19. Quadratic time when opening an archive — `read.rs:650-666`**
- Each EOCD candidate re-parses the central directory.
- In release: 204 KB took 0.18 s, 408 KB took 0.74 s, and 815 KB took 2.9 s. Time quadruples per doubling, so a 16 MB file would extrapolate to about 20 minutes. I did not run that size.

**20. Declared uncompressed size is never enforced — `read.rs:436-445`**
- Probe: stored `size()=3` read 100,000 bytes with `Ok`. Deflate `size()=1` produced 10,000,000 bytes with `Ok`.
- A size check before extraction doesn't protect against a bomb.

**21. ZipCrypto uses a fixed header — `write.rs:1049`**
- The "random" header is 12 zero bytes apart from the CRC byte.
- Probe: two runs produced byte-identical archives, which leaks equal plaintexts and gives 11 known plaintext bytes per file. ZipCrypto is documented as weak, but this makes it weaker.

## Low

**22. `has_overlapping_files` overflows — `read.rs:996`.** `start + compressed_size` panics in debug with `attempt to add with overflow`. In release it wraps and returned `Ok(false)`.

**23. `AesReader::new` underflows — `aes.rs:70-71`.**
- With `compressed_size` below the overhead, debug panics, including through `get_aes_verification_key_and_salt`.
- In release, a 4-byte entry returned `UnexpectedEof`.
- By reading only: a size large enough to pass header validation (at least 18 bytes for AES-256, below the 28-byte overhead) wraps `data_remaining`, and the read then ends through the unauthenticated path in finding 5.

**24. `ZipFile::data_start()` panics on streamed entries — `read.rs:1818`, `types.rs:827`.** The doc at `read.rs:1935` says it is set to 0. Probe: `Option::unwrap()` on `None`.

**25. `by_name_seek` and `by_index_seek` ignore encryption — `read.rs:1087-1103`.** `by_name` returns `Password required`, but the seek variants return `Ok` with ciphertext (18 and 34 bytes in the probe).

**26. `read_exact` skips the CRC check — `crc32.rs:49`.** The check only runs on a zero-length read. Probe: a bad CRC gave `read_exact` `Ok(())`, while `read_to_end` gave `Invalid checksum`.

**27. Streamed empty archive fails — `read.rs:1944`.** Probe: `ZipArchive` gave `Ok(0)`, while the stream `visit` gave `UnexpectedEof`.

**28. Implode table length overflow — `legacy/implode.rs:29`.** `byte + 1` on a `u8` panics in debug at 255. In release it wraps to 0 and the table is rejected with `not enough codeword lengths`. By reading only: a 256-entry table is legal in the format. Mutation fuzzing also hit this line.

**29. Archive contents can trip a debug assertion — `read.rs:1668`.** With `a/x` and `a/../b`, `extract_unwrapped_root_dir` panics with `Root directory filter should not match at this point`. The comment blames only unstable filters. Release extracts `["b","x"]`.

**30. Streamed entry skip fails after an LZMA header error — `compression.rs:320`, `read.rs:1914`.** The reader was taken and lost, so `Drop` can't exhaust the entry. Probe: position 8225 instead of 40037, and the next call gave `Invalid local file header`. By reading only: PPMd has the same pattern at `compression.rs:364`.

**31. `add_symlink_from_path` corrupts relative targets — `write.rs:1526`, `unstable.rs:107-110`.** Probe: `a/b -> ../c` stored `"c"`, which points at `a/c`. The doc's exception only covers paths outside the root.

**32. `add_directory_from_path(".")` creates an entry named `/` — `unstable.rs:77-78`, `write.rs:1445-1448`.** Extracting that archive then fails with `Invalid file path`.

**33. `merge_archive` silently replaces duplicate names — `write.rs:1267`.** Probe: `Ok(())`, names `["x"]`, content `second`. `start_file` rejects duplicates.

**34. DOS read-only mode loses its type bits — `types.rs:630-633`.** Probe: `444` versus `100664` for a writable entry. By reading only: `version_needed` also relies on these bits.

**35. `add_extra_data` problems — `write.rs:315-316`, `write.rs:364`.**
- A rejected field stays in the options. In the probe, a rejected ZIP64 ID was still written by `start_file` (local extra length 20), and later adds keep failing.
- A valid 0x7075 field is rejected, because its CRC is checked against an empty name.

**36. Compression level docs are wrong — `write.rs:445,447` versus `write.rs:1975,1989`.** Level 0 is documented for Deflate and Bzip2, but both return `Unsupported compression level`.

**37. The partial-extraction permission promise is false — `read.rs:773-775` versus `read.rs:315-326`.** The doc says partly extracted directories are private to you. Probe: `40755` after a failed extraction.

**Findings: 37. Last `./clock` reading: 30 minutes used of 45.**
