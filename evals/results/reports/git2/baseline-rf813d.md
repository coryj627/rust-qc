# Audit report: git2 0.20.2 (`crate/src/`)

**How I tested.**
- All probes are in a throwaway crate, `tmp/probe`, run through `./run`. `crate/` was not modified.
- "ASan" means a nightly build with `-Zsanitizer=address`, with libgit2's C code also instrumented by clang. "LSan" means the leak checker in that build.
- ASan traces were not symbolized, because the container has no symbolizer. They show the error class, not function names.
- "Debug" and "release" are ordinary stable builds of the probe.
- `cargo test --locked -p git2 --no-default-features` passes (187 unit tests plus the integration tests and doctests), so none of these defects are caught by the existing tests.
- Nothing in the repository or its output addressed me.

## Critical and high: memory unsafety reachable from safe code

1. **`src/remote.rs:283`: `Remote::get_refspec` returns a dangling refspec.** It returns `Refspec<'repo>`, but the refspec is owned by the `git_remote`. A refspec kept after the `Remote` drops reads freed memory. Ran it: ASan heap-use-after-free; segfault with `MALLOC_PERTURB_`.

2. **`src/rebase.rs:213,220`: `RebaseOperation` can outlive its `Rebase`.** `Iterator for Rebase` yields `RebaseOperation<'rebase>`, where `'rebase` is the repository lifetime, but the operation points into the `git_rebase`. Using `op` after `drop(rebase)` is a use-after-free. Ran it: ASan heap-use-after-free; `op.id()` returned a corrupted OID.

3. **`src/repo.rs:2289-2305,2313` with `src/rebase.rs:84-100`: `Rebase` keeps pointers into `RebaseOptions` without borrowing it.** libgit2 copies the options struct by `memcpy` (`rebase.c:292`), including the checkout callback payload and path pointers. Dropping the options and then calling `rebase.next()` calls the progress callback on freed memory. Ran it: ASan heap-use-after-free; segfault.

4. **`src/apply.rs:127,140` and `src/stash.rs:132`: callback setters store a pointer to `self`.** `hunk_callback`, `delta_callback` and `progress_cb` save `self as *mut _` as the payload. Moving the options afterwards, which safe Rust allows, leaves that pointer dangling. I returned the options from a helper function and boxed them. Ran it: ASan stack-use-after-return for both; segfault for apply, exit 135 (SIGBUS) for stash.

5. **`src/pathspec.rs:63-67`: `match_diff` does not borrow the diff.** Lifetime elision ties the list only to `&self`, but the list stores `git_diff_delta*` pointers owned by the diff (`pathspec.c:647`). `diff_entry()` after the diff drops is a use-after-free. Ran it: ASan heap-use-after-free; garbage path, then a bus error.

6. **`src/indexer.rs:126`: `Indexer::new` does not keep its `Odb` alive.** It takes `Option<&Odb<'a>>` with an unconstrained borrow, and libgit2 stores the raw pointer (`indexer.c:180`). Dropping the odb and then writing to the indexer is a use-after-free. Ran it: ASan heap-use-after-free; segfault.

7. **`src/repo.rs:2638` and `src/reflog.rs:11`: `Reflog` has no repository lifetime.** The refdb backend inside it keeps a raw repository pointer. `Reflog::write` after the `Repository` drops reads freed memory. Ran it: ASan heap-use-after-free. Without ASan, a bogus "log file ... doesn't exist" error under `MALLOC_PERTURB_`.

8. **`src/revwalk.rs:43-46`: `into_inner` leaves the hide callback installed.** `git_revwalk_reset` does not clear `hide_cb`, and the returned `Revwalk` has no `'cb` lifetime. Dropping the callback and then using `hide()` plus iteration calls freed memory. Ran it: ASan heap-use-after-free; segfault.

9. **`src/repo.rs:504,520` and `src/repo.rs:544,549,554,563`: `&self` setters free memory that getters lend out.** `workdir()` and `namespace_bytes()` return borrows, while `set_workdir` and `set_namespace`/`remove_namespace` take `&self` and free the old string. Ran both: ASan heap-use-after-free; the stale borrow read `0xAA` bytes under `MALLOC_PERTURB_`.

10. **`src/repo.rs:1109-1127`: `get_attr_bytes` values do not survive an attribute reload.** The returned slice lives in libgit2's attribute cache but is tied to `&self`. A later `get_attr` after `.gitattributes` changes frees it. Ran it: ASan heap-use-after-free; garbage value.

11. **`src/index.rs:736`: `CStr::from_ptr` on a null pointer.** With empty pathspecs, libgit2 passes `matched_pathspec = NULL` (`pathspec.c`). `add_all`, `update_all` and `remove_all` with an empty list and a callback crash before the callback runs. Ran all three: exit 139 (segfault).

12. **`src/blame.rs:111,132`: `final_signature`/`orig_signature` can wrap NULL.** `final_signature` wraps a NULL signature on hunks produced by `blame_buffer` for modified lines (zero commit id). `orig_signature` uses the same pattern, but I only read that one. Calling `.name()` on the `final_signature` result: release exit 139 (segfault), debug abort ("null pointer dereference").

13. **`src/build.rs:288-289` via `src/call.rs:39-44`: `RepoBuilder::clone` can return `Ok` with a null repository.** A notify callback returning `false` during clone makes `git_clone` return a positive code with `*out = NULL` (`clone.c:660-672`). `try_call!` treats positive as success, so `from_raw(NULL)` runs. Ran it: `is_ok()` was true, then `repo.path()` panicked at `repo.rs:457`.

14. **`src/buf.rs:47,53`, `src/diff.rs:1281`, `src/remote.rs:387`: `slice::from_raw_parts` on NULL.** Each of these reaches it:
    - `Buf::new()` followed by a deref.
    - `DiffBinaryFile::data()` without `show_binary`, or on a parsed "Binary files differ" patch.
    - `Remote::list()` on a connection to an empty repository.

    Ran them: debug aborts with exit 134 ("unsafe precondition violated"). In release the undefined behavior is visible: `Buf::new().as_str()` returned `None`.

## High: security

15. **`src/cred.rs:434-436`: credential-helper protocol injection through the username.** The username is written without rejecting newlines, and libgit2 percent-decodes it from the URL. Verified each step:
    - `http://alice%0Ahost=github.com@127.0.0.1/...` reached the credentials callback as `"alice\nhost=github.com"`.
    - The helper's stdin then contained a second `host=` line.
    - For `https://evil.example/repo`, real `git credential-store` returned `s3cret`, the password stored for github.com. Without the injection it returned `None`.

16. **`src/cred.rs:210-216`: helpers are queried with no host.** `url::Host::Domain` is the only host kind kept, so IPv4 and IPv6 URLs send no `host=`. Unparseable URLs such as `git@evil.example:x/y.git` send no protocol or host either. The helper then matches any stored credential. Ran it: `git credential-store` returned `("alice", "s3cret")` (stored for github.com) for both `https://203.0.113.9/repo` and the scp-style URL.

## Medium: wrong results and lost panics

17. **`src/call.rs:15,40-43`: panics from callbacks are lost and resurface later.** `try_call!` only runs `panic::check()` on negative returns. A panicking callback therefore disappears when the C call returns positive or ignores the callback result. Ran each panic path; each of these returned `Ok`:
    - `checkout` progress (`build.rs:637`) and notify (`build.rs:699`)
    - `Odb::foreach` (`odb.rs:540`)
    - `mergehead_foreach` and `fetchhead_foreach` (`repo.rs:64,97`)
    - `stash_foreach` (`stash.rs:174`)
    - `tag_foreach` (`repo.rs:2057`)

    `OdbPackwriter`'s `write` and `commit` (`odb.rs:460,494`) returned `Err` but lost the panic. `Indexer::write` (`indexer.rs:192-196`) has the same missing check; I only read that one. The stored panic also blocks later callbacks on that thread (`panic.rs:10`). Ran it: it re-fired at the next unrelated failure, a `find_branch("no-such")` call.

18. **`src/build.rs:575,693-697`: a cancelled checkout reports success.** The docs say returning `false` from notify cancels the checkout. It does stop it, but `checkout_tree` returned `Ok(())`. Ran it: `Ok(())` with the file unchanged. This has the same cause as finding 13.

19. **`src/repo.rs:2057`: `tag_foreach` ignores the return of `git_tag_foreach`.** It always returns `Ok`. Ran it with tags `a`, a broken symref `m`, and `zzz`: only `a` was visited and the result was `Ok(())`.

20. **`src/build.rs:351-352`: `dry_run` no longer checks anything.** It uses `GIT_CHECKOUT_NONE`, which in this libgit2 skips the checkout entirely, rather than `GIT_CHECKOUT_DRY_RUN`. Ran it with a conflicting edit: `Ok(())` and no notifications, while a real safe checkout gave "1 conflict prevents checkout".

21. **`src/build.rs:351,359,369`: the strategy mask `(1 << 4) - 1` is wrong.** Two results, both run:
    - It does not clear bit 30, so `dry_run().force()` changes nothing.
    - It clears `RECREATE_MISSING` (bit 2), so `recreate_missing(true).safe()` does not recreate the file, while `safe().recreate_missing(true)` does.

22. **`src/merge.rs:98`: `MergeOptions::skip_reuc` sets the wrong flag.** It sets `GIT_MERGE_FAIL_ON_CONFLICT`. Ran it: a conflicting `merge_commits` returned `Err(MergeConflict)` instead of an index with conflicts.

23. **`src/cred.rs:256-265`: only the last helper at each config level is used.** The code says "Discover all helper directives", but `get_string` returns one value. Ran it with two helpers configured: only the second ran, it had no credentials, and the result was `None`. Git would have tried the first.

24. **`src/build.rs:312`, `src/transport.rs:226,294`: custom error messages are dropped.** These return `raw_code()` without setting the libgit2 error, so the user sees an unrelated stale message. Ran `remote_create` and the transport factory: "could not find .../.git/shallow" and "could not find .../.gitconfig". `transport.rs:294` I only read.

25. **`src/tree.rs:219-222`: `Tree::walk` fails on valid non-UTF-8 directory names.** Ran it: `Err("git_tree_walk callback returned -1")`, and the rest of the tree was not walked.

26. **`src/lib.rs:148-155`: `is_bit_set!` with zero-valued flags is always false.** Ran it: `Sort::NONE.is_none()`, `MergeAnalysis::ANALYSIS_NONE.is_none()`, `MergePreference::NONE.is_none()`, `DiffStatsFormat::NONE.is_none()`, the `is_default` methods on `IndexAddOption`, `PathspecFlags`, `StashFlags` and `StashApplyFlags`, and `RepositoryInitMode::SHARED_UMASK.is_shared_umask()` all return `false`.

27. **Panics on non-UTF-8 repository or remote data.** Each location below panics where the return type (or a sibling API) could report it. Ran:
    - `src/reference.rs:498` (`names()`)
    - `src/remote.rs:490` (`RemoteHead::name`, remote-controlled)
    - `src/repo.rs:576` (`message`)
    - `src/repo.rs:2365` (`note_default_ref`)
    - `src/describe.rs:38`
    - `src/worktree.rs:115`
    - `src/message.rs:32` (`message_prettify` on bytes)
    - `src/stash.rs:163`

    Only read: `src/remote.rs:495`, `src/remote_callbacks.rs:401,451,455`, `src/repo.rs:83`, and `src/util.rs:93` (Windows `bytes2path`, which contradicts the doc at `blame.rs:144`).

28. **Panics in `Option`-returning accessors when the optional field is absent.** Ran all three:
    - `src/merge.rs:56`: `AnnotatedCommit::refname` when created by id.
    - `src/rebase.rs:319`: `RebaseOperation::exec` on any non-exec operation.
    - `src/refspec.rs:37`: `Refspec::dst` for a fetch spec like `refs/heads/main`.

29. **`src/diff.rs:629`: `DiffFile::mode` panics on unusual modes from a patch.** A patch from `Diff::from_buffer` with `old mode 100600` panics with "unknown mode: 33152", so patch input can trigger it. Ran it.

## Low

30. **`src/index.rs:397`, `src/pathspec.rs:162`: invalid paths panic.** `Index::get_path("./seed")` and `matches_path("/abs/a")` panic from `.unwrap()`. Ran both.

31. **Interior NUL panics in functions that could return an error or `None`.** Ran: `src/lib.rs:936`, `src/tree.rs:173`, `src/refspec.rs:83`, `src/transaction.rs:41`. Same pattern, read only: `src/refspec.rs:97`, the rest of `transaction.rs`.

32. **`src/transport.rs:359`: NUL in an I/O error message aborts the process.** The unwrap runs inside `extern "C"` outside `panic::wrap`. Ran it: exit 134. `src/remote_callbacks.rs:432` via `src/error.rs:363` has the same exposure; read only.

33. **`src/index.rs:761`: `IndexEntries` can panic on aliased handles.** `repo.index()` returns aliased handles, so clearing through one while iterating another panics. Ran it.

34. **`src/opts.rs:391-399,428-436`: timeout setters hide failures.** They claim they cannot fail, but libgit2 rejects negative values (`settings.c:434`). Ran both setters with `-5`: release returned `Ok(())`, debug panicked on the `debug_assert` at `opts.rs:397`.

35. **`src/diff.rs:273`: deprecated `format_email` can panic.** Its assert fails on a multi-line subject, because the summary is whitespace-squashed. Ran it.

36. **Memory leaks of the progress payload.**
    - `src/packbuilder.rs:95-96`: every `write` call.
    - `src/odb.rs:165-175`: `packwriter` when it fails.
    - `src/indexer.rs:134-147`: `Indexer::new` when it fails.

    Ran all three: LSan reported a 16-byte direct leak each.

37. **Integer truncation of indexes.** `src/commit.rs:300,314` and `src/blame.rs:65` cast indexes to 32 bits. Ran it: `parent(1<<32)`, `parent_id(1<<32)` and `get_index(1<<32)` all returned entry 0.

38. **`src/commit.rs:360-381`: `Parents` breaks its iterator contracts.** It claims `FusedIterator` and `ExactSizeIterator`. Ran it with one missing parent and one real parent: `len()` was 2, and `next()` returned `None` and then `Some`.

39. **Equality contract problems.**
    - `src/time.rs:9,55-59`: `Eq` includes `sign` but `Ord` does not. Ran it: for `-0000` versus `+0000`, `==` is false while `cmp` is `Equal`.
    - `src/tree.rs:378-381`: entries with different ids compare equal. Ran it.
    - `src/reference.rs:421-425`: `refs/heads/a1 == refs/heads/a2` is true when they share a target. Ran it.

40. **`src/packbuilder.rs:190`: deprecated `hash()` looks inverted.** It returns `Some` only when `object_count() == 0`. Ran it: `None` after writing a non-empty pack.

41. **`src/version.rs:43-45`: `crate_version` returns the wrong crate's version.** The docs say libgit2-sys (0.18.1), but it returns git2's `0.20.2`. Ran it.

42. **`src/error.rs:53-60`: the "unknown error" fallback is dead code.** Vendored `git_error_last` never returns NULL (`util/errors.c:289`). Ran it: `Error::last_error(-1).message()` is `"no error"`.

43. **`src/stash.rs:10,61`: `StashSaveOptions.message` has no setter.** `stash_save_ext` can never record a message. Read only.

44. **`src/odb.rs:361`: `OdbReader::read` writes through a shared-derived pointer.** It uses `buf.as_ptr() as *mut`, which is aliasing undefined behavior; `as_mut_ptr` is the fix. Read only.

45. **`src/diff.rs:25`: `unsafe impl Send for Diff` lets a repository be shared across threads.** The `Diff` holds the repository, but `Repository` is not `Sync`. A scoped-thread probe moved the `Diff` to another thread while still using `&repo`. It compiled and ran; I did not observe an actual race.

**Number of findings: 45. Last `./clock` reading: 22 minutes used of 45.**
