# Audit report: zip 5.1.1 (`crate/src/`)

**Method**
- The baseline passes: `cargo test --locked --all-features --lib --tests`, and doctests skipping the 4 GiB one.
- Unless a finding says otherwise, evidence comes from my throwaway crates `probe/` and `probe2/`, run through `./run`. `crate/` was not changed.
- The profile was dev with opt-level 1 and overflow checks on. I re-ran only some cases in release, and those are marked "release:".
- Other tools used:
  - a mutation fuzzer: archive mode about 1.4M iterations per run, legacy decoder mode about 800k per run
  - a writer model test: 1500 seeds
  - single-feature `cargo check`
  - Miri, on `spec::` only
- Entries "over 4 GiB" were crafted by claiming the size in Zip64 metadata; I did not write real 5 GiB data.
- Nothing in the repository addressed me as an agent in the files I read. I did not open `.amazonq/`.

## High

1. **AES HMAC check can be skipped: `aes.rs:171-172`, `aes.rs:178-188`.**
   - The authentication code is only checked when `data_remaining` reaches 0. If the reader hits EOF first, `read` returns `Ok(0)` with no check. If `compressed_size` equals the overhead, `read` returns `Ok(0)` immediately. AE-2 has no CRC backstop (`crc32.rs:48`).
   - Caller sees tampered or truncated plaintext as a successful read.
   - Ran: a normal-layout tamper gives "Invalid authentication code". The same tamper with the auth code removed and `compressed_size` past EOF (EOCD placed before the data) returned `Ok(15)` "pay 900 dollars" (original "pay 100 dollars"). With the body removed it returned `Ok(0)`.

2. **`ZipWriter::new_append` and `merge_archive` destroy Zip64 sizes: `write.rs:2094-2110`, with the field stripped at `read.rs:1479` and `read.rs:1433-1441`.**
   - The parser removes the Zip64 extra field and the central directory writer never regenerates it, so sizes are clamped to 0xFFFFFFFF.
   - Ran: entry of 5368709120 bytes became (4294967295, 4294967295) after both append and merge. Offsets at or above 4 GiB take the same path (read only).

3. **`raw_copy_file*` zeroes sizes of entries over 4 GiB: `write.rs:919-920`, `write.rs:1088`.**
   - `start_entry` builds the Zip64 field with sizes 0, 0 and raw mode never updates it.
   - Ran: source (5368709120, 5368709120) became destination (0, 0).

4. **`extract` creates symlinks pointing anywhere, contrary to its docs: `read.rs:907-908`, `read.rs:465`; docs at `read.rs:768-770`.**
   - The docs say links are "only created and followed if the target is within the destination directory". `make_symlink` does no check.
   - Ran: `abs -> /etc/passwd` and `rel -> ../../../../../etc/hostname` were created with `extract=Ok`, and reading `abs` returned the host file (878 bytes).

5. **Process abort on a crafted symlink entry: `read.rs:894`.**
   - `Vec::with_capacity(file.size())` uses an attacker-chosen size.
   - Ran: declared size 1<<62 gave "memory allocation of 4611686018427387904 bytes failed" and an abort that cannot be caught. u64::MAX gave a "capacity overflow" panic.

6. **Prepended stub plus a stored nested zip lists the wrong archive: `spec.rs:681-699`.**
   - When the guessed offset misses, the first `PK\1\2` found scanning forward is trusted, and that can be inside file data.
   - Ran: outer names `["inner.zip"]`. With a 4096-byte stub prepended, `ZipArchive::new` returned `Ok(["inner-secret"], offset 4016)`.

7. **`extract` applies setuid and setgid bits from the archive: `read.rs:918-919`, `read.rs:942`.**
   - Ran as uid 1000: an entry with mode 0o106755 was extracted as 106755.

## Medium

8. **Restrictive file modes are applied only at the end of extraction: `read.rs:912`, `read.rs:941`.**
   - Ran: a 0600 `secret.key` followed by an entry with a bad CRC gave `extract=Err`, and the secret stayed on disk at 0644 with its contents.

9. **Directory modes are never applied, and the doc's guarantee is false: `read.rs:898-900`, `read.rs:321-326`; docs at `read.rs:773-775`.**
   - The docs say partial directories are not readable by other users.
   - Ran: a directory with archive mode 0700 came out 0755.

10. **`ZipStreamReader` never delivers central directory metadata: `read.rs:1944`, `read.rs:1948`, `stream.rs:48`.**
    - The first 30 bytes of the first central header are consumed, the next parse fails, and `while let Ok` hides the failure. The docs at `stream.rs:124` call the visit "guranteed".
    - Ran: `visit_additional_metadata` was called 0 times for 3 entries, and a 0751 file was extracted as 0644.

11. **Stream extraction never creates symlinks: `stream.rs:70`, `types.rs:832`.**
    - `external_attributes` is always 0 in streaming mode, so the branch is dead.
    - Ran: the link was extracted as a regular file.

12. **`finish_into_readable` and `deep_copy_file` break `large_file` entries: `write.rs:2078-2079`, `write.rs:684-686`.**
    - In-memory sizes are overwritten with 0xFFFFFFFF.
    - Ran: readable archive `size=4294967295`, read returned "Invalid checksum" after 177 bytes. After `deep_copy_file` both source and copy report compressed size 41 and fail with "Invalid checksum", so the source is damaged too.

13. **`finish_into_readable` cannot read AES entries: `write.rs:954-964`.**
    - `compression_method` stays `Aes` in memory.
    - Ran (model test): `by_name_decrypt` gave "Compression method not supported". Reopening from bytes is the path that normally works.

14. **`raw_copy_file` drops the ZipCrypto encrypted flag: `types.rs:716-725`, `read.rs:1832-1851`.**
    - Ran: `dst_encrypted=false`, and reading returned ciphertext bytes with "Invalid checksum".

15. **`raw_copy_file` drops the symlink type and the comment: `read.rs:1836`, `write.rs:1316`.**
    - The docs say metadata is copied.
    - Ran: symlink src=true, dst=false. Comment "my comment" became "".

16. **`start_file(name, aes_file.options())` writes plaintext flagged as AES: `read.rs:1845-1849`, `write.rs:939-957`.**
    - Ran: `encrypted=true` and the plaintext marker appears literally in the output bytes.

17. **Rewriting the central directory corrupts metadata: `types.rs:849-865`, `write.rs:2107`, `types.rs:917`, `types.rs:910-912`.**
    - `flags()` keeps only 3 bits, comments are written as decoded UTF-8, and system and invalid dates are replaced.
    - Ran on `tests/data/legacy/implode.zip`: after append or raw copy it fails with "too many codeword lengths".
    - Ran: cp437 comment "café" became "caf├⌐". A UTF-8 comment on an ASCII name also became "caf├⌐".
    - Ran: made-by system 19 became 4. DOS date 0 became the current time.

18. **Panic then abort in the writer: `types.rs:902`, `types.rs:934`, `types.rs:938`, with the double panic from `write.rs:1671-1676`.**
    - Unchecked `try_into().unwrap()` panics, then `Drop` runs `finalize` and panics again.
    - Ran, all aborted with "panic in a destructor during cleanup":
      - `start_file` with a 70000-byte name
      - `shallow_copy_file` with a 70000-byte name
      - `new_append` then `finish` on a 40000-byte cp437 comment that expands past 65535 bytes as UTF-8

19. **`new_stream` plus ZipCrypto is unreadable: `write.rs:1141-1142`, `zipcrypto.rs:168`, `read.rs:412-416`.**
    - The writer stores the CRC check byte, while the reader expects the time byte for data-descriptor entries.
    - Ran: the correct password gave "provided password is incorrect".

20. **`new_stream` plus AES always fails at finish: `write.rs:1117`, `write.rs:2019`.**
    - It seeks to patch the extra field.
    - Ran: `finish` returned an I/O error, with "seek is not supported" printed from drop.

21. **`new_stream` with `large_file(true)` rejects files over 4 GiB: `write.rs:2053-2057`.**
    - The check is in the wrong branch, so it fires exactly when Zip64 is in use. Non-large entries get truncated with `as u32` at `types.rs:967-968` (read only).
    - Ran: 4097 MiB to a sink, `finish` failed with "Large file option has not been set".

22. **Quadratic parse on crafted input: `read.rs:650-666`.**
    - Every EOCD candidate re-parses the whole central directory.
    - Ran: 3.1 MB file with 400 candidates took 2.8 s, about 7 ms per candidate. Extrapolated (not run), about 5.3 MB would take about 12 minutes.

23. **Valid archives rejected because of optional extra fields: `zipinfo_utf8.rs:20`, `read.rs:1530`, `read.rs:1538-1541`, `ntfs.rs:33-53`.**
    - Unicode field CRCs and NTFS tags are treated as fatal, and the comment CRC is computed over the decoded text rather than the raw bytes.
    - Ran, all gave `ZipArchive::new` failure:
      - correct CRC on a cp437 comment
      - stale Unicode path CRC
      - invalid UTF-8 in a path field
      - NTFS attribute tag 2

24. **Real errors replaced by "Could not find EOCD": `read.rs:660-666`.**
    - Ran: a multi-disk archive and the cases in finding 23 all reported this message. The "Support for multi-disk" message at `read.rs:689` is never seen.

25. **Shrink off-by-one excludes code 8191: `shrink.rs:29`, `shrink.rs:91`.**
    - Ran: a hand-built stream using code 8190 decoded correctly; code 8191 failed with "invalid code". Long Shrink files that fill the table would be affected.

26. **Legacy decoders abort on declared size: `shrink.rs:227`, `shrink.rs:340`, `reduce.rs:107`, `implode.rs:97`, `implode.rs:214`.**
    - Ran: Shrink with 1<<62 gave an allocation-failure abort. Shrink, Reduce and Implode with u64::MAX gave a "capacity overflow" panic.

27. **`ZipCrypto` rejects the correct password on invalid DOS dates: `read.rs:414`.**
    - A data-descriptor entry with an invalid date uses time 0 as the validator.
    - Ran: valid date reads "hello"; DOS date 0 gives "provided password is incorrect".

28. **`by_name_seek` and `by_index_seek` skip the encryption check: `read.rs:1094-1097`.**
    - Ran: `by_name` said a password is required, while `by_name_seek` returned 53 bytes of salt, ciphertext and auth code as the content of a 25-byte file.

29. **ZIP64 comment lost on reopen: `spec.rs:365-367`, `write.rs:1569`.**
    - The reader ignores EOCD64 unless a 32-bit field is saturated, but the writer emits EOCD64 for the comment alone.
    - Ran: `zip64_comment()` returned `None`.

30. **Any symlink on an extraction path is an error: `read.rs:1718-1723`, `path.rs:13`.**
    - `simplified_components(base_path)` is always `None` for the absolute base. The logic at `read.rs:1725-1747` is effectively dead. This is contrary to the "followed" and "overwriting" wording in the docs.
    - Ran: an in-tree `d -> real` then `d/f`, and a second extract over an existing link, both gave "Invalid base path". A second extract of absolute links gave "Invalid symlink target path".

## Low

31. **`ZipFile::data_start()` panics in streaming mode: `read.rs:1818`.** The docs at `read.rs:1935` say it is set to 0. Ran: `unwrap` on `None`.

32. **Arithmetic on archive-controlled sizes: `aes.rs:70`, `read.rs:996`, `read.rs:296`.**
    - Ran: all three panic with overflow in debug.
    - Release: the AES case gives an I/O error, `has_overlapping_files` returns `Ok(false)`, and `seek(End(0))` returns u64::MAX.

33. **Implode tree size overflow: `implode.rs:29`.** `byte + 1` on a u8. Ran: debug panic. Release: "not enough codeword lengths", so a legal 256-byte table is rejected.

34. **Huffman position underflow: `huffman.rs:141`.** Found by the fuzzer through `implode.rs:138`; debug panic. The release path was not run.

35. **Attacker-reachable `debug_assert!`: `read.rs:1668`.** `root_dir` uses a different path form from extraction. Ran with `a/../b/x`: debug panic. Release: Ok, extracted without unwrapping.

36. **Stream extract fails on a valid empty zip: `read.rs:1944`.** Ran: a 22-byte empty archive gives an I/O error; `ZipArchive` opens it.

37. **`new_stream` raw copy omits the data descriptor: `write.rs:1088`.** Ran: local flags 0x8, sizes 0, no descriptor present.

38. **`unix_mode` returns 0 for Unix-made entries with only DOS attribute bits: `types.rs:622`.** Ran: attr 0x20 extracted as a file with mode 000.

39. **DOS read-only mode loses the file type: `types.rs:632`.** Ran: 0o444 versus 0o100664 for a normal file.

40. **Documented compression level 0 is rejected: `write.rs:445-447` versus `write.rs:1975`, `write.rs:1989`.** Ran: Deflate and Bzip2 level 0 give "Unsupported compression level".

41. **Zopfli-only build panics on levels the range accepts: `write.rs:1766`, `write.rs:1726`, `write.rs:1977`.** Ran with `deflate-zopfli` only: levels 9, 5 and 1 panic. The range starts at 1.

42. **`merge_archive` silently replaces duplicates and leaves the writer open: `write.rs:1267`, `write.rs:1259`.** Ran: my "x" was replaced by "theirs", `is_writing_file` was true, and a stray `write` returned `Ok(5)`.

43. **Failed `add_extra_data` leaves the bad field in place: `write.rs:315-316`.** Ran: a later `start_file` fails with the same error.

44. **`add_symlink_from_path` corrupts targets: `write.rs:1526`.** Ran: `a/b/link -> ../c` stored "c", and `/opt/tool` stored "opt/tool".

45. **Path lookup cannot find directories: `read.rs:1069`, `unstable.rs:116-117`.** Ran: names `["dir/"]`, and `index_for_path` returns `None` for both "dir/" and "dir".

46. **Zero-byte Xz entries are unreadable: `write.rs:1836-1842`.** The root cause appears to be the `lzma_rust2` writer. Ran: "index CRC32 mismatch", and Python's `lzma` also rejects the bytes.

47. **Vacuous security test: `read/stream.rs:383-389`.** `writer.finish()` returns a cursor at its end, so `is_err` comes from EOF. Ran: after a rewind, extract returns `Ok`, so the test's `is_err` assertion would fail.

## Outside this code, not counted

- Malformed Xz entries reach `lzma_rust2-0.13.0/src/xz.rs:890` through `compression.rs:444`, causing a huge-allocation abort or a capacity panic.
- `lzma2_reader.rs:147` overflows in debug.

## Checked with no defect found

- `cp437` table compared against Python: 128 pairs, no mismatch.
- ZipCrypto CRC table matches a computed reference.
- Single-feature builds pass, except `deflate-flate2` with no backend, which fails inside `flate2` and matches that feature's comment in `Cargo.toml`.
- Lib tests pass with no default features.
- Miri passes on `spec::`.

**Findings: 47. Last `./clock` reading: 28 minutes used of 45.**
