# Audit of zip 5.1.1 (`crate/src/`)

**Verdict:** the crate handles ordinary archives, but it has serious defects in AES integrity, extraction safety and archive rewriting.

- A forged AES entry decrypts as success.
- `extract` creates symlinks that point outside the destination.
- `new_append`, `merge_archive` and the raw-copy paths can corrupt entries, including two of the repository's own fixtures.

 I did not change `crate/`. All probes are throwaway crates under `tmp/probe`, run through `./run`. Unless a finding says otherwise, I ran the input shown and saw the stated result. No repository text addressed me.

## High

**AES authentication is skipped when the source ends early**
`src/aes.rs:178-188`.
- **Problem:** if the inner read returns 0 while `data_remaining > 0`, `read` returns `Ok(0)` and never checks the HMAC. AE-2 has no CRC either, so the forgery passes.
- **Established:** I took a real AES-256 entry, flipped a ciphertext bit and dropped the auth code. I placed the data after the EOCD (the end-of-central-directory record, tolerated as trailing garbage) and declared `compressed_size` one byte too large. `by_index_decrypt` then returned `Ok(14)` with `"pay alice $900"`; the original text was `"pay alice $100"`.

**`extract` creates symlinks pointing outside the destination**
`src/read.rs:907-908`, `src/read.rs:448-465`. The contrary documentation is at `src/read.rs:768-770`.
- **Problem:** the docs say links are "only created" if the target is inside the destination, but `make_symlink` checks nothing.
- **Established:** entries `evil -> /etc/passwd` and `evil2 -> ../../../etc` both extracted with `Ok(())`. Reading `dest/evil` returned the 878-byte host file.

**Rewriting an archive with an Info-ZIP Unicode path makes it unopenable**
`src/read.rs:1538-1541`, `src/write.rs:2098-2101`, `src/extra_fields/zipinfo_utf8.rs:20`.
- **Problem:** the reader replaces `file_name_raw` with the Unicode name but keeps the 0x7075 field, whose CRC covers the old name. On re-read the CRC check fails and the whole archive is rejected.
- **Established:** I ran the repo's `tests/data/chinese.zip` and `windows-7zip.zip` through `new_append` + `finish`, through `merge_archive`, and through `new_append` + `deep_copy_file` + `shallow_copy_file` + `finish`. Reopening gave `Could not find EOCD` in every case. Python's `zipfile` still lists the entries.

**`new_append` destroys ZIP64 sizes and offsets**
`src/read.rs:1433-1441` (strips the field), `src/write.rs:2094-2110` (never re-emits it), `src/types.rs:924-946` (clamps the values).
- **Established:**
  - An entry written at offset 5 GiB in a sparse file read correctly. After `new_append` + `start_file` + `finish` it failed with `Invalid local file header`.
  - A crafted entry with size 5 GiB became 4294967295.

**`large_file(true)` overwrites in-memory sizes with 0xFFFFFFFF**
`src/write.rs:2078-2079`, used by `src/write.rs:683-687` and `src/write.rs:777-786`.
- **Established:**
  - After `finish_into_readable`, an 11-byte entry reports size and compressed size 4294967295. Reading it returns the next local header too, then `Invalid checksum`.
  - `deep_copy_file` on such an entry stores `csize=52` for both source and copy, so both fail with `Invalid checksum` after a normal reopen.

**Raw-copying an entry over 4 GiB records size 0**
`src/write.rs:919-925`, `src/write.rs:1088`.
- **Problem:** the ZIP64 field is written with zero sizes, and the update step is skipped when `writing_raw` is set.
- **Established:** a source entry declaring 5 GiB reads back as `size=0` after `raw_copy_file`. A real entry with both sizes over the limit would take the same path, but I did not run one.

**Raw-copying a ZipCrypto entry clears its encrypted flag**
`src/read.rs:1832-1851`, `src/types.rs:716`.
- **Established:** after `raw_copy_file`, `encrypted()` is false. `by_name_decrypt` returns ciphertext as plaintext and then `Invalid checksum`.

**A symlink entry with a huge declared size panics `extract`**
`src/read.rs:894`. The same pattern is at `src/read/stream.rs:71`, but that path is unreachable.
- **Established:** a symlink entry with a ZIP64 size of `u64::MAX` gives `capacity overflow` from `extract`, in debug and release.

**Legacy decoders panic on a huge declared size**
`src/legacy/reduce.rs:107`, `src/legacy/shrink.rs:227`, `src/legacy/shrink.rs:340`, `src/legacy/implode.rs:97`, `src/legacy/implode.rs:214`. Applies to the `legacy-zip` feature.
- **Established:** methods 1, 2 and 6 with size `u64::MAX` each panic with `capacity overflow` on the first read.

**Shrink control codes overflow the stack**
`src/legacy/shrink.rs:141`. Applies to the `legacy-zip` feature.
- **Problem:** `read_code` recurses once per control code.
- **Established:** 200,000 `(256, 2)` pairs, about 450 KB of compressed data, abort the process with a stack overflow in debug and release. This cannot be caught.

**Over-long names and comments panic, then abort in `Drop`**
`src/types.rs:902`, `src/types.rs:934`, `src/types.rs:938`.
- **Established:**
  - `start_file` with a 70,000-byte name panics, then panics again in `Drop`, and the process aborts (exit 134).
  - `new_append` + `finish` on an entry with a 40,000-byte CP437 comment does the same, because the UTF-8 form no longer fits in a `u16`.

## Medium

**A prepended stub plus a stored nested zip returns the wrong archive**
`src/spec.rs:681-706`.
- **Problem:** only the first CDFH (central directory header) signature after the relative offset is tried.
- **Established:** a 4096-byte stub + `[a.txt, nested.zip]` opens with `Ok`, offset 4178 and names `["inner.txt"]`. Python lists the real two entries. With a single outer entry, the listing opens but `by_index` fails with `Invalid local file header`.

**`visit_additional_metadata` is never called**
`src/read.rs:1944-1948`, `src/read/stream.rs:48`.
- **Problem:** the reader consumes 30 bytes of the first central header, then `while let Ok` silently ends.
- **Established:** a visitor saw `files=2 metadata=0`, although the docs guarantee the call. `ZipStreamReader::extract` leaves a 0755 script at 0644.

**Stream extraction turns symlinks into regular files**
`src/read/stream.rs:70`, `src/types.rs:832`.
- **Problem:** `is_symlink` is always false here, because `external_attributes` is 0.
- **Established:** `lnk` becomes a file containing `real.txt`, while `ZipArchive::extract` makes a real link.

**One unusual extra field rejects the whole archive**
`src/read.rs:1396-1402`, `src/read.rs:1488`, `src/extra_fields/ntfs.rs:33-52`, `src/extra_fields/extended_timestamp.rs:24-46`, `src/extra_fields/zipinfo_utf8.rs:20`.
- **Established:** each of these opens in Python but fails with `Could not find EOCD`:
  - NTFS length 8
  - UT (extended timestamp) field that is empty, or has flags 1 with length 13
  - stale 0x7075 CRC

  I also ran AES-field length 8 here, with the same error from this crate, but did not compare it in Python.

**Unicode comment CRC is computed over decoded text**
`src/read.rs:1530`.
- **Problem:** the CRC is checked against the decoded comment instead of the raw bytes.
- **Established:** a CP437 comment `caf\x82` with a correct 0x6375 field makes the archive unopenable.

**Real parse errors are replaced by "Could not find EOCD"**
`src/read.rs:660-666`, unreachable message at `src/read.rs:689`.
- **Established:** multi-disk archives, bad extra fields and bad entries all report `Could not find EOCD`. Callers cannot tell these apart.

**EOCD retry cost is quadratic**
`src/read.rs:650-666`.
- **Established:** many EOCD records point at one central directory with a count one too high. Release timings:

| File size | Time to reject |
|---|---|
| 0.42 MB | 1.3 s |
| 0.85 MB | 5.1 s |
| 3.2 MB | 72 s |

**`has_overlapping_files` is quadratic**
`src/read.rs:997-999`.
- **Established:** 200k entries take 5.4 s and 400k take 21.8 s in release.

**A truncated EOCD comment is rejected**
`src/spec.rs:334-337`.
- **Established:** a comment length 5 bytes past EOF gives `EOCD comment exceeds file boundary`. Python opens the same file.

**Entry count ignores the central directory size**
`src/read.rs:516`.
- **Established:**
  - 65,537 entries with a wrapped count of 1 give `len() == 1` with no error. Python lists 65,537.
  - `on_this_disk = 0` with a total of 1 gives `len() == 0`.

**A zero-file EOCD skips all offset checks**
`src/spec.rs:666-672`.
- **Established:**
  - A 46-byte file with CD offset 0xFFFFFFFF opens with `central_directory_start() == 4294967295`. `new_append` + `start_file` then writes at 4 GiB, and a `Cursor<Vec>` target was OOM-killed.
  - A comment containing a fake empty EOCD makes a real archive read as 0 entries.

**Directory modes are dropped, and the docs claim otherwise**
`src/read.rs:898-901`, claim at `src/read.rs:773-775`.
- **Established:** `priv/` with mode 0700 extracts as 0755, and `ro/` with 0500 extracts as 0755.

**Setuid bits are applied from the archive**
`src/read.rs:941-942`.
- **Established:** mode 0o104755 extracted as `104755`. I recall that Info-ZIP strips these bits by default, but I did not run it.

**Unix entries with no mode extract as mode 000**
`src/types.rs:622`.
- **Established:** made-by Unix with attributes 0x20 gives `unix_mode() == Some(0)` and on-disk mode `100000`.

**Any existing symlink on the path fails with "Invalid base path"**
`src/read.rs:1718-1723`.
- **Problem:** `simplified_components` returns `None` for the absolute base path, so the `is_absolute_enclosed` branch is dead code.
- **Established:**
  - Extracting `real.txt` + `link -> real.txt` twice fails the second time, despite the "overwriting" docs.
  - An entry `cur/x` under `cur -> real` also fails.

**`by_index_seek` returns ciphertext**
`src/read.rs:1087-1103`.
- **Established:** with no password it returned 28 bytes for a 16-byte ZipCrypto entry and 44 bytes for an AES entry.

**`data_start()` panics on stream entries**
`src/read.rs:1818`. The docs at `src/read.rs:1935` and `src/read/stream.rs:120` say it is 0.
- **Established:** `Option::unwrap` panic in debug and release.

**Streaming an empty archive fails**
`src/read.rs:1944`.
- **Established:** a valid 22-byte empty zip gives an `i/o error`, because 30 bytes are read before the signature is checked.

**`new_stream` with ZipCrypto produces undecryptable output**
`src/write.rs:168` against `src/read.rs:412-416`.
- **Problem:** the writer stores the CRC check byte, but bit 3 tells the reader to expect the time byte.
- **Established:** the correct password returns `provided password is incorrect`.

**`new_stream` sets bit 3 without writing a descriptor**
`src/write.rs:1009`, `src/write.rs:1088`, `src/write.rs:1451`.
- **Established:** after `raw_copy_file`, flags are `0x8`, and the data is followed directly by `PK\x03\x04`. A forward-only reader can't parse this. `add_directory` takes the same path, but I confirmed that only by reading the code.

**`new_stream` rejects a large file that was declared large**
`src/write.rs:2053`.
- **Established:** `large_file(true)` with 4.3 GiB fails at finish with `Large file option has not been set`. The check is inverted. In the non-large branch at `src/types.rs:967`, `as u32` truncates instead (read only).

**`deep_copy_file` after `new_append` writes a bad local header**
`src/types.rs:868`, `src/read.rs:1433`.
- **Established:** the copy of a ZIP64 entry has local sizes `0xffffffff` and `extra_len=0`. `read_zipfile_from_stream` reports 4294967295 for both sizes.

**`new_append` rewrites text it should preserve**
`src/write.rs:2107`, `src/types.rs:850`.
- **Established:** comment `café` becomes `caf├⌐`. Name `caf├⌐.txt` (raw bytes that are valid UTF-8) becomes `café.txt`.

**General-purpose flag bits are lost on rewrite**
`src/types.rs:849-865`.
- **Established:** `tests/data/legacy/implode.zip` fails with `too many codeword lengths` after `new_append`, `merge_archive` or `raw_copy_file`. Central flags went from `6` to `0`.

**AES entries lose method 99 on rewrite**
`src/read.rs:1514`, `src/types.rs:920`.
- **Established:** central `(flags, method)` goes from `(1, 99)` to `(1, 8)` after `new_append`. This crate still reads it. I expect other tools won't, but I did not test any.

**`set_zip64_comment` can't be read back**
`src/spec.rs:365-367`, `src/write.rs:1569`.
- **Established:** the EOCD64 is written, but `zip64_comment()` is `None` and `new_append` sees `None`. Appending to such an archive therefore loses the comment.

**ZipCrypto bit-3 entries with an invalid DOS date reject the right password**
`src/read.rs:414`.
- **Established:** date `0x5021` decrypts, while date `0` gives `InvalidPassword`.

**Declared size is not enforced**
`src/read.rs:1184-1190`.
- **Established:** `size() == 1` while the read returned 11 bytes with `Ok`, so callers limiting by `size()` are bypassed.

**Backslash-separated names extract as one flat file**
`src/types.rs:598-612`.
- **Established:** `dir\f.txt` becomes a single file on Unix, plus a directory literally named `dir\`. `mangled_name` converts the separators, but `extract` does not.

**Implode code-table length overflows**
`src/legacy/implode.rs:29`.
- **Problem:** `byte + 1` overflows `u8` when the count byte is 255. I constructed a table of 256 single-symbol runs.
- **Established:** that table panics in debug. In release it fails with `not enough codeword lengths`.

**Untrusted headers drive large allocations**
`src/compression.rs:444`, `src/compression.rs:373`, `src/legacy/shrink.rs:227`.
- **Established:**
  - Under `ulimit -v 1500000`, a 196-byte mutated XZ entry aborted on a 35 TiB allocation. Without the limit it finished normally.
  - An 825-byte Shrink entry aborted on a 4 GiB reservation under the same limit.
  - A PPMd payload of 34 bytes (2-byte parameter + 32 bytes) reached 264 MB resident.

**Raw copy drops metadata the docs say is copied**
`src/read.rs:1836`, `src/write.rs:477`, `src/write.rs:1412-1420`.
- **Established:**
  - A symlink becomes `100777` with `is_symlink=false`.
  - A directory becomes `100755`.
  - The comment and extra fields are gone.
  - `raw_copy_file_touch(.., Some(0o120777))` also loses the type.

**`merge_archive` silently overwrites duplicates**
`src/write.rs:1267`.
- **Established:** merging two archives that both contain `a.txt` gives one entry with no error, and the first is lost. `start_file` rejects the same name.

**Unchecked arithmetic panics under overflow checks**
`src/aes.rs:70`, `src/read.rs:996`, `src/read.rs:296`, `src/legacy/huffman.rs:141`.
- **Established:**
  - AES `compressed_size` below the overhead panics.
  - `has_overlapping_files` and `seek(End(0))` panic with a ZIP64 size of `u64::MAX`.
  - Fuzzing hit the first and last of these in debug builds.
  - In release, the AES case gave normal errors and `has_overlapping_files` returned `Ok(false)` by wrapping. I did not run the other two sites in release.

**`debug_assert` fires on input and on I/O failure**
`src/read.rs:1668`, `src/write.rs:1096`.
- **Established:**
  - An entry `root/../evil` panics `extract_unwrapped_root_dir` in debug.
  - A failing writer hit the assert at `src/write.rs:1096` during cleanup, panicked again in `Drop`, and aborted in debug.
  - Release showed 0 panics over 6,000 failure points.

**Tests that do not test the behaviour**
`src/read/stream.rs:383`, `src/read/stream.rs:288-301`, `src/read/stream.rs:332-345`.
- **Established:**
  - `test_cannot_symlink_outside_destination` passes because the cursor is at EOF (position 242 of 242), which gives `i/o error`. After rewinding, `extract` returns `Ok(())` and writes `dest/symlink/dest-file`.
  - The `visit_additional_metadata` assertions never run (read only, since that callback is never called).

## Low

- **Write after merge:** `src/write.rs:1259`. `write` after `merge_archive` returns `Ok(4)` and puts the bytes in the output (ran). Clear `writing_to_file`.
- **Stream abort:** `src/write.rs:1155`. `abort_file` on a stream returns `Err` but has already removed the entry (ran). Pop only after it succeeds.
- **Rejected extra data kept:** `src/write.rs:315-316`. `add_extra_data` returns `Err` but keeps the field, and a later `start_file` succeeds (ran). Validate before adding.
- **Root dir with `./`:** `src/read.rs:1257-1262`. `root_dir` returns `Some(".")` for `./root/...`, and nothing is unwrapped (ran). Skip `CurDir` components.
- **`by_path` on directories:** `src/unstable.rs:98-117`. `by_path("dir/")` is not found, while `by_name` finds it (ran). Keep the trailing slash.
- **`decompressed_size`:** `src/read.rs:566`. Returns `None` for any bit-3 entry, though central sizes exist (ran). Use the central sizes.
- **Empty AES entries:** `src/aes.rs:171`. The auth code is never checked, so a tampered one gives `Ok(0)` (ran). Verify at construction.
- **DOS read-only mode:** `src/types.rs:632`. `mode &= 0o555` clears the type bits, so `unix_mode()` returns `444` (ran). Clear only the write bits.
- **Compression level docs:** `src/write.rs:445-447`. Docs say 0-9, but Deflated and Bzip2 reject 0 (ran). Fix the docs or the range.
- **Dead fast path:** `src/unstable.rs:85-87`. `contains([a, b])` matches any char, not a substring, so the path is dead (read only). Use string patterns.
- **`Drop` behaviour:** `src/write.rs:1674`. The library prints to stderr and finalizes again after a failed `finish` (ran). Skip the retry and the print.
- **Supported methods list:** `src/compression.rs:247`. `SUPPORTED_COMPRESSION_METHODS` omits Lzma and the legacy methods the crate decodes (read only). Add them.
- **Clippy on 1.99:** `src/write.rs:2173`. `collapsible_match` under `-D warnings`. This is toolchain drift (ran).
- **Documentation:** 59 functions lack `# Errors` and 5 lack `# Panics`, for example `src/unstable.rs:119`.
- **SAFETY comments:** 10 are missing (7 `unsafe impl`, 3 blocks), such as `src/spec.rs:157-167`. I read the `Pod` impls and they look sound.
- **`Debug`:** missing on `ZipFile` at `src/read.rs:246`, `ZipFileSeek` at `src/read.rs:252` and `StreamWriter` at `src/write.rs:2136`.
- **Lint configuration:** 35 `#[allow]` without a reason, 39 asserts without messages, and 4 glob re-exports (`src/read.rs:33`, `src/unstable.rs:10`, `src/extra_fields/mod.rs:23`, `src/extra_fields/mod.rs:25`).
- **`unwrap`:** 33 calls in non-test code. The ones I could reach from input or the API are listed above. I did not review all the others individually.
