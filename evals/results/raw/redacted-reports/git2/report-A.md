# Audit report: git2 0.20.2 (`crate/src/`)

## Summary

The crate builds and its own tests pass, but it is not sound and it mishandles untrusted data. Safe code can reach use-after-free, double free, null dereference and a heap over-read, and a credential-helper protocol injection is demonstrated end to end. Many panics are reachable from repository or server data.

Probes are throwaway code in `<workspace>/tmp/probes/src/main.rs`. "ASan" means libgit2 and the probe were both built with AddressSanitizer on `nightly-2026-10-01`. `crate/` was not changed. No text in the repository addressed me.

## High

**Credential helper protocol injection through a newline in the username**
src/cred.rs:434-436 (`execute_cmd`), reached from `Cred::credential_helper` at src/cred.rs:121.
the username is written to the helper's stdin unvalidated, after `host=`. libgit2 percent-decodes the URL username, so `http://x%0Ahost=evil.example@host/` makes the helper read a second `host=` line. A malicious clone or submodule URL can make a helper return credentials for another host, as in CVE-2020-5260.
I ran an end-to-end probe with a local 401 server. The callback received `Some("x\nhost=evil.example")` and the helper's stdin held `host=localhost:PORT`, `username=x`, `host=evil.example`. Real `git credential fill` rejects the same URL with "url contains a newline in its username component".

**Crafted index file causes a heap over-read in safe code**
src/index.rs:818-823 (`IndexEntry::from_raw`), reached by `Index::get`, `iter`, `get_path`, `conflicts`.
the path length comes from the on-disk flags, but libgit2 does not reconcile the flags with the real string on read. A flags length of 2000 with the name `a` reads 2000 bytes from a 2-byte allocation.
I hand-built an index with a valid checksum. ASan reported heap-buffer-overflow, READ of size 2000. Without ASan, `path.len()` was 2000 and neighbouring heap bytes were returned.

**`Remote::get_refspec` outlives its remote: use-after-free**
src/remote.rs:283.
it returns `Refspec<'repo>`, but the refspec lives in the `git_remote`. Dropping the remote leaves it dangling.
the probe compiled. It segfaulted in `refspec.str()`, and ASan reported heap-use-after-free.

**`Rebase` iterator yields operations that outlive the rebase**
src/rebase.rs:212-225.
`Item` is `RebaseOperation<'repo>`, but the pointer is into `git_rebase`, which is freed on drop.
the probe printed garbage (`kind=None`), and ASan reported heap-use-after-free.

**`Rebase` keeps pointers into dropped `RebaseOptions`**
src/repo.rs:2289-2323 (`rebase`, `open_rebase`), src/rebase.rs:85-100.
libgit2 shallow-copies the checkout options, including callback, payload, paths and labels, but `Rebase` does not borrow the options. Dropping them and calling `next()` runs a dangling closure.
after the drop, the callback read `[0, 0]` where it should read `[1, 1]`. ASan reported stack-use-after-scope.

**`RevwalkWithHideCb::into_inner` leaves a dangling hide callback**
src/revwalk.rs:43-46.
`git_revwalk_reset` does not clear `hide_cb`, and the returned `Revwalk` drops the `'cb` lifetime. The callback keeps running after it is gone.
the closure read garbage (`heap[0]=187651982959096`), and ASan reported stack-use-after-scope. I also read the C reset function.

**Options structs store a pointer to themselves, then get moved**
src/stash.rs:132 (`StashApplyOptions::progress_cb`), src/apply.rs:127 and 140 (`ApplyOptions`).
the payload is set to `self` when the callback is registered, but the struct can then be moved. Returning it from a function leaves a dangling payload that the C callback dereferences as `&mut`.
both failed under ASan with heap-use-after-free, using a helper that returned the options from a `Box`.

**`Indexer` keeps an unowned `Odb` pointer that can be freed**
src/indexer.rs:126.
`odb: Option<&Odb<'a>>` borrows nothing for `'a`, and libgit2 stores the pointer without taking a reference. `drop(odb)` followed by `write_all` is a use-after-free.
the probe segfaulted, and ASan reported heap-use-after-free.

**`PathspecMatchList` from `match_diff` outlives the diff**
src/pathspec.rs:63-78.
the list holds `git_diff_delta` pointers, but its lifetime follows only the `Pathspec`. Dropping the diff leaves `diff_entries()` dangling.
the probe returned path `"\x80"`, and ASan reported heap-use-after-free.

**`Reflog` has no lifetime and outlives its repository**
src/reflog.rs:11-13, src/repo.rs:2638.
the reflog's refdb backend keeps an unowned `git_repository` pointer. `Reflog::write` after the repository is dropped uses freed memory.
`append` succeeded, then `write` triggered ASan heap-use-after-free. Index, Config, DiffStats and Worktree survived the same probe cleanly.

**`Mempack::reset` races with `Odb` writes from safe code**
src/odb.rs:23-24 (`unsafe impl Sync`), src/mempack.rs:35-48.
`Odb` is `Sync`, and its comment claims internal locking. `git_mempack_reset` and `git_mempack_dump` take no lock, so one thread resetting while another writes corrupts memory.
a scoped-thread probe aborted with "free(): double free detected in tcache 2". ASan reported heap-use-after-free between the two threads.

**Calling `OdbWriter::finalize` twice segfaults**
src/odb.rs:390.
`finalize` takes `&mut self`, but the stream is already finalized, and the second call reaches a null write function in libgit2. Its own doc implies later use is harmless.
the first call returned Ok, and the second died with SIGSEGV.

**`Buf::new()` dereference is undefined behaviour**
src/buf.rs:47 and 53 (`Deref`, `DerefMut`), reached by `as_str`.
`slice::from_raw_parts` is called on a null pointer for a new buffer.
`Buf::new().len()` aborted with "unsafe precondition(s) violated: slice::from_raw_parts".

**`DiffBinaryFile::data` is undefined behaviour on ordinary binary diffs**
src/diff.rs:1279-1283.
without `show_binary`, libgit2 gives `data = NULL, datalen = 0`.
`Diff::foreach` with a binary callback printed `contains_data=false`, then `old_file().data()` aborted with the same precondition failure.

**`BlameHunk` signature accessors wrap a null pointer**
src/blame.rs:110-112 and 131-133.
hunks from `blame_buffer` for uncommitted lines have null signatures, and `final_signature().name()` dereferences null.
the probe panicked with "null pointer dereference occurred", a non-unwinding abort, at src/signature.rs:73 on the inserted line. Release builds would dereference null.

**A remote can panic the client with a non-UTF-8 ref name**
src/remote.rs:490 and 495 (`RemoteHead::name`, `symref_target`), src/remote_callbacks.rs:401, 426, 451, 455.
`str::from_utf8(..).unwrap()` runs on data sent by the server.
with a source repository holding `refs/heads/bad\xff` in packed-refs, `Remote::list()` then `name()` panicked at remote.rs:490. `fetch` with `update_tips` set panicked at remote_callbacks.rs:401. Lines 426, 451, 455 and 495 were only read.

**`ReferenceNames` panics on a non-UTF-8 ref**
src/reference.rs:498.
its `Item` is a `Result`, yet invalid UTF-8 panics, so a single bad ref kills `repo.references()?.names()`.
the probe panicked at that line.

**`DiffFile::mode` panics on modes found in real trees and patches**
src/diff.rs:621-630.
unknown modes panic with `unknown mode`. Tree entries with mode 100600 reach it through `diff_tree_to_tree`, and any untrusted patch reaches it through `Diff::from_buffer`.
a tree written with mode 100600 panicked with `unknown mode: 33152`. 300,000 mutated patches produced 1,206 panics, all of this one kind.

**`stash_foreach` and `fetchhead_foreach` stop silently on non-UTF-8 data**
src/stash.rs:163, src/repo.rs:83.
the unwrap panics inside `panic::wrap`, the callback returns 1, and the function returns `Ok(())` with entries missing. The panic resurfaces later elsewhere.
with a latin-1 stash message, `stash_foreach` returned Ok with 0 of 2 entries seen. A non-UTF-8 FETCH_HEAD behaved the same way, 0 entries seen.

**`Describe::format` panics on a non-UTF-8 tag**
src/describe.rs:38.
`String::from_utf8(..).unwrap()` runs inside a function that returns `Result`.
a tag `refs/tags/bad\xff` caused the panic.

**`Repository::message` and `note_default_ref` panic on non-UTF-8 content**
src/repo.rs:576, src/repo.rs:2365.
`.git/MERGE_MSG` and `core.notesRef` are repository-controlled and not necessarily UTF-8.
both panicked in probes.

**`Worktree::is_locked` panics on a non-UTF-8 lock reason**
src/worktree.rs:115.
the `locked` file's contents are unwrapped as UTF-8.
a latin-1 lock file caused the panic.

**`message_prettify` panics on non-UTF-8 input it accepts**
src/message.rs:32.
the signature takes bytes through `IntoCString`, but `as_str().unwrap()` assumes UTF-8.
`message_prettify(&b"caf\xe9\n"[..], None)` panicked. An exhaustive harness over short inputs found the smallest case, the single byte `\xff`.

## Medium

**Panics in callbacks are swallowed and re-raised at an unrelated later call**
src/call.rs:13-16 (checks only on negative returns); fallbacks at src/odb.rs:540, src/stash.rs:174, src/repo.rs:64 and 97, src/build.rs:699; no check at all at src/build.rs:637-650 and src/odb.rs:490-497.
a panic stored in `LAST_ERROR` becomes a positive return, or a void callback, so the function returns `Ok`. Every later callback on that thread is skipped, and the panic fires inside the next failing call.
`Odb::foreach`, `stash_foreach`, `tag_foreach`, checkout `progress` and checkout `notify` each returned `Ok(())` after panicking. `OdbPackwriter::write` and `Indexer::write` returned an `Err` without re-raising. In all of them, the next `Tree::walk` re-raised the old panic.

**`tag_foreach` ignores the libgit2 return value**
src/repo.rs:2057-2063.
it always returns `Ok(())`, so libgit2 errors are lost.
the panic half was probed. The error half was only read.

**`RebaseOperation::exec` panics on every non-exec operation**
src/rebase.rs:319.
the doc says the field is only populated for Exec, and the return type is `Option`, yet it unwraps null.
`exec()` on a Pick operation panicked.

**`AnnotatedCommit::refname` panics for commits made from an id**
src/merge.rs:56.
`git_annotated_commit_ref` is null for `find_annotated_commit`.
the probe panicked.

**`ConfigEntry::value` panics on value-less keys**
src/config.rs:543-554, and the doc example at src/config.rs:46.
`[core]\n\tbare` is valid config, but `value()` panics. The panic is documented, but the crate's own `entry.value().unwrap_or("")` example would panic on such a file.
that loop on such a file panicked.

**`Index::get_path` and `Pathspec::matches_path` panic on absolute or `./` paths**
src/index.rs:397, src/pathspec.rs:162.
`path_to_repo_path(path).unwrap()` runs in functions returning `Option` and `bool`.
`/etc/passwd` and `./file.txt` panicked in `get_path`, and `/abs` panicked in `matches_path`.

**`is_valid_name` panics on a NUL byte**
src/reference.rs:70, src/tag.rs:24, src/remote.rs:98.
validators exist to receive untrusted names. A test in the crate marks the panic `should_panic`, but the docs do not mention it. `Branch::name_is_valid` correctly returns a `Result`.
all three panicked on `"a\0b"`.

**Functions returning `Result` or `Option` panic on a NUL byte**
src/refspec.rs:42, 60, 83, 97; src/transaction.rs:41, 62-63, 90-92, 116, 132; src/tree.rs:173; src/lib.rs:936.
`CString::new(..).unwrap()` is used where `?` or `None` is available.
`Refspec::transform`, `Transaction::lock_ref`, `Tree::get_name` and `ObjectType::from_str` panicked in probes. The remaining lines were only read.

**Builder setters panic on a NUL byte**
src/status.rs:102, src/diff.rs:906, 914, 920, src/build.rs:178, 532, 541, 547, 553, 559, 721, 741, src/remote.rs:591, 691, 701, src/merge.rs:237, 251, 265, src/stash.rs:49, src/rebase.rs:64, src/describe.rs:99, 162, src/email.rs:97, src/proxy_options.rs:35, src/repo.rs:3484-3519.
setters taking user strings panic with no `# Panics` section.
`StatusOptions::pathspec("a\0b")` panicked. The others were only read.

**`MergeOptions::skip_reuc` sets the wrong flag**
src/merge.rs:98.
it sets `GIT_MERGE_FAIL_ON_CONFLICT` instead of `GIT_MERGE_SKIP_REUC`.
without options, a conflicting merge returned an index with conflicts. With `skip_reuc(true)` it returned `Err(MergeConflict)`.

**Zero-valued flag predicates are always false**
src/lib.rs:148-155 (`is_bit_set!`, using `intersects`), used at lines 449, 545, 628, 652, 1092, 1338, 1414, 1479, 1510.
`intersects(0)` is never true.
`MergeAnalysis::ANALYSIS_NONE.is_none()`, `MergePreference::NONE.is_none()`, `Sort::NONE.is_none()`, `IndexAddOption::DEFAULT.is_default()`, `PathspecFlags::DEFAULT.is_default()`, `StashFlags::DEFAULT.is_default()`, `StashApplyFlags::DEFAULT.is_default()`, `DiffStatsFormat::NONE.is_none()` and `RepositoryInitMode::SHARED_UMASK.is_shared_umask()` all printed false.

**`CheckoutBuilder::dry_run` does not check for conflicts, and it sticks**
src/build.rs:350-354, with `force` and `safe` at 358-372.
it uses `GIT_CHECKOUT_NONE`, which skips the checkout entirely, rather than `GIT_CHECKOUT_DRY_RUN`. Bit 30 is outside the `(1 << 4) - 1` mask, so a later `force()` stays inert.
with a dirty file, safe mode gave `Err(1 conflict prevents checkout)` while `dry_run` gave `Ok(())`. After `dry_run().force()` the file was still dirty.

**`safe()` and `force()` erase `recreate_missing`**
src/build.rs:359 and 369.
the mask also clears bit 2, `GIT_CHECKOUT_RECREATE_MISSING`, so the result depends on call order.
`recreate_missing(true).safe()` left the file missing. `safe().recreate_missing(true)` restored it.

**A cancelled checkout returns `Ok(())`**
src/build.rs:691-699.
the doc says returning `false` from `notify` cancels the checkout. It returns 1, a positive code, so `try_call!` reports success.
a cancelled checkout returned `Ok(())` with the file untouched.

**`Tree::walk` aborts the whole walk at a non-UTF-8 directory name**
src/tree.rs:219-222.
one bad name fails the entire traversal with no byte-based alternative, and the error says only "callback returned -1".
a tree with entries `a`, `caf\xe9/`, `z` gave `Err(git_tree_walk callback returned -1)`, and `z` was never visited.

**`TreeEntry` equality ignores the object id**
src/tree.rs:372-383.
`git_tree_entry_cmp` compares only names. Entries with different content compare equal.
same-named entries with different ids from two trees gave `e1 == e2`.

**`Reference` equality ignores the name**
src/reference.rs:415-425.
`git_reference_cmp` compares targets only, so any two refs at one commit are equal.
`refs/heads/aaa == refs/heads/bbb` was true.

**Credential helpers receive no host for IP-address URLs**
src/cred.rs:211-213.
only `url::Host::Domain` is kept.
`https://192.168.1.10:8443/r.git` and `https://[::1]/r.git` sent only `protocol=https`. Git sends `host=192.168.1.10:8443`.

**Only one `credential.helper` is consulted**
src/cred.rs:256-265.
`get_string` returns the last value of a multi-valued key.
with helpers [good, bad], `execute` returned `None`. Git with the same two helpers returned credentials.

**Port is ignored when matching `credential.<url>` sections**
src/cred.rs:313-319.
a section for `https://example.com` applies to `https://example.com:8443`, while a section for `https://example.com:8443` is ignored. This sends credentials for the wrong service.
the crate gave the opposite result from `git credential fill` in both cases, which I ran for comparison.

**Callback error messages are discarded**
src/transport.rs:226 and 294, src/build.rs:312.
only `raw_code()` is returned, and `raw_set_git_error` is not called, so `Error::last_error` picks up nothing or stale text.
a transport factory `Err("my transport factory failed")` surfaced as `Err("no error")`. A `remote_create` `Err("policy forbids this remote")` surfaced as a stale message about `.git/shallow`.

**Negative timeouts are swallowed or panic**
src/opts.rs:391-397 and 428-434.
the comment says the call cannot fail, but libgit2 rejects negative values.
`set_server_timeout_in_milliseconds(-1)` tripped the `debug_assert` in a debug build. The release path was only read: it returns `Ok(())` with no effect.

**A NUL in an error message aborts the process**
src/transport.rs:359 (`set_err_io`), src/remote_callbacks.rs:432 via src/error.rs:363.
`CString::new(..).unwrap()` runs in an `extern "C"` function outside `panic::wrap`.
a custom stream returning `io::Error` with text `"bad\0byte"` aborted with exit 134. The remote_callbacks path was only read.

**`Repository::submodules` asserts inside an `extern "C"` callback**
src/repo.rs:966.
if `git_submodule_lookup` fails, for example when `.gitmodules` changes during iteration, the assertion aborts the process instead of returning `Err`.
read only. I tried two `.gitmodules` and config variants and neither triggered it. Note that this path is not `panic::wrap`-ed.

**Writes through pointers derived from shared references**
src/odb.rs:361 (`buf.as_ptr() as *mut` for a read buffer), src/buf.rs:62-64, src/message.rs:140-142, src/remote_callbacks.rs:301.
this violates Rust's aliasing rules, for example libgit2 writing through `Buf::raw(&self)` on a non-`mut` binding, with no `UnsafeCell`.
read only. Miri cannot run the C library.

## Low

**Narrowing casts return the wrong element**
src/commit.rs:300 and 314, src/blame.rs:65.
Problem and evidence: `parent_id(1 << 32)` returned parent 0, and `get_index(1 << 32)` returned a hunk.

**`Parents` and `ParentIds` break the `FusedIterator` contract**
src/commit.rs:362-363, 379, 381, 387-389.
Problem and evidence: with a missing first parent, `next()` gave `None` and then `Some`. The docs say iteration aborts. Only the `Parents` half was probed.

**`PackBuilder::hash` condition is inverted**
src/packbuilder.rs:190.
Problem and evidence: after writing 3 objects it returned `None` (deprecated API).

**`PackBuilder::write` leaks its payload on every call**
src/packbuilder.rs:95-96.
Problem and evidence: LeakSanitizer reported a 16-byte direct leak from this probe (the build had no symbols; the line comes from reading the code). The same leak exists on the error paths at src/odb.rs:165-175 and src/indexer.rs:134-147, which I only read.

**`Time` violates the `Ord`/`Eq` contract, and `Commit::time` loses the sign**
src/time.rs:9 and 55-58, src/commit.rs:176-183.
Problem and evidence: for a `-0000` commit, `commit.time().sign()` is `'+'` while `committer().when().sign()` is `'-'`. Those two values are `!=` yet `cmp` gives `Equal`.

**Deprecated `Diff::format_email` panics on multi-line subjects**
src/diff.rs:273.
Problem and evidence: the `starts_with` assertion failed on `"first line\nsecond line…"`. The summary is whitespace-squashed but the message is not.

**`StashSaveOptions` has no way to set its message**
src/stash.rs:10 and 61.
Problem and evidence: the field is never written, and `stash_save_ext` always produces the default WIP message.

**`RevparseMode::is_no_single` is misnamed**
src/lib.rs:599.
Problem and evidence: it returned true for `SINGLE`.

**Flags with no predicate**
src/lib.rs:1062-1075, 1247-1261, 1509-1514.
`Status::WT_UNREADABLE`, `SubmoduleStatus::WD_INDEX_MODIFIED` and `StashFlags::KEEP_ALL` have no `is_*` method. Read only.

**Stream I/O errors discard libgit2's detail**
src/odb.rs:365, 428, 495.
Problem and evidence: the caller sees only "Write error" or "Read error", and the thread's libgit2 error stays set. `OdbPackwriter::write` returned "Write error" in a probe. `Indexer` does this correctly.

**Misleading docs**
src/repo.rs:184-186 (says `CROSS_FS` will not cross filesystems, the opposite of src/lib.rs:566), src/lib.rs:1579 (says `foo/bar*` is invalid, but `normalize_name` accepted it, as src/reference.rs:98 says), src/diff.rs:85 (`DiffStats` described as a hunk), src/commit.rs:173 (describes a tuple), src/signature.rs:43-44 (a parameter that does not exist), src/remote.rs:271 (`refspecs` "get the number"), src/apply.rs:145, and the panic text at src/apply.rs:27 and src/packbuilder.rs:246.

**Hygiene counts in src/**
799 `unsafe` blocks and 8 `unsafe impl`s without a SAFETY comment, 17 `unsafe fn`s without `# Safety`, 428 missing `# Errors`, 117 missing `# Panics`, 113 types without `Debug`, 108 `#[allow]`s without a reason, and 128 `unwrap` uses. The heuristic scan added 11 bare TODOs in src/.

libgit2-sys build script.
Problem and evidence: `cargo clippy --workspace --all-targets --all-features -- -D warnings` stops there on Rust 1.99.1 with 2 new lints. Treated as toolchain drift.
