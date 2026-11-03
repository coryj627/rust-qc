        "stash_moved_opts" => {
            fn make<'a>() -> StashApplyOptions<'a> {
                let mut o = StashApplyOptions::new();
                o.progress_cb(|_| { println!("progress"); true });
                o
            }
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "m", None).unwrap();
            let mut o = make();
            fn clobber() -> u64 { let a = std::hint::black_box([0x41u8; 4096]); a.iter().map(|x| *x as u64).sum() }
            println!("{}", clobber());
            println!("apply {:?}", repo.stash_apply(0, Some(&mut o)));
        }
        "apply_moved_opts" => {
            fn make<'a>() -> ApplyOptions<'a> {
                let mut o = ApplyOptions::new();
                o.delta_callback(|_| { println!("delta"); true });
                o
            }
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let d = repo.diff_index_to_workdir(None, None).unwrap();
            let mut o = make();
            fn clobber() -> u64 { let a = std::hint::black_box([0x41u8; 4096]); a.iter().map(|x| *x as u64).sum() }
            println!("{}", clobber());
            println!("apply {:?}", repo.apply(&d, ApplyLocation::Index, Some(&mut o)));
        }
        "rebase_op_dangling" | "rebase_exec" => {
            let (_td, repo) = init();
            let base = commit(&repo, &[(b"f", b"a\n")], "base");
            let head_branch = repo.head().unwrap().name().unwrap().to_string();
            let _tip = commit(&repo, &[(b"g", b"b\n")], "tip");
            repo.branch("up", &repo.find_commit(base).unwrap(), true).unwrap();
            repo.set_head("refs/heads/up").unwrap();
            repo.checkout_head(Some(CheckoutBuilder::new().force())).unwrap();
            commit(&repo, &[(b"h", b"c\n")], "up");
            let one = repo.reference_to_annotated_commit(&repo.find_reference(&head_branch).unwrap()).unwrap();
            let up = repo.reference_to_annotated_commit(&repo.head().unwrap()).unwrap();
            let mut opts = RebaseOptions::new();
            opts.inmemory(true);
            if name == "rebase_exec" {
                let mut r = repo.rebase(Some(&one), Some(&up), None, Some(&mut opts)).unwrap();
                let op = r.next().unwrap().unwrap();
                println!("{:?}", op.kind());
                println!("{:?}", op.exec());
            } else {
                let op = {
                    let mut r = repo.rebase(Some(&one), Some(&up), None, Some(&mut opts)).unwrap();
                    r.next().unwrap().unwrap()
                };
                println!("{:?} {:?}", op.kind(), op.id());
            }
        }
        "annotated_refname" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let a = repo.find_annotated_commit(id).unwrap();
            println!("{:?}", a.refname());
        }
        "tag_foreach" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            repo.tag_lightweight("t1", &repo.find_object(id, None).unwrap(), false).unwrap();
            let r = repo.tag_foreach(|_, _| panic!("boom"));
            println!("tag_foreach returned {:?}", r);
            let mut n = 0;
            let r = repo.tag_foreach(|_, _| { n += 1; true });
            println!("again {:?} visited {}", r, n);
        }
        "notify_cancel" => {
            let (td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            let _b = commit(&repo, &[(b"f", b"b\n")], "b");
            let mut cb = CheckoutBuilder::new();
            let mut called = 0;
            cb.notify_on(CheckoutNotificationType::all()).notify(|_, _, _, _, _| { called += 1; false });
            let r = repo.checkout_tree(&repo.find_object(a, None).unwrap(), Some(&mut cb));
            drop(cb);
            println!("called {} result {:?} file now {:?}", called, r, fs::read_to_string(td.path().join("f")));
        }
        "notify_panic" => {
            let (_td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            commit(&repo, &[(b"f", b"b\n")], "b");
            let mut cb = CheckoutBuilder::new();
            cb.notify_on(CheckoutNotificationType::all()).notify(|_, _, _, _, _| panic!("boom"));
            let r = repo.checkout_tree(&repo.find_object(a, None).unwrap(), Some(&mut cb));
            println!("result {:?}", r);
        }
        "recreate_order" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "a");
            fs::remove_file(td.path().join("f")).unwrap();
            let mut cb = CheckoutBuilder::new();
            cb.recreate_missing(true).safe();
            repo.checkout_head(Some(&mut cb)).unwrap();
            println!("recreate_missing(true).safe(): exists={}", td.path().join("f").exists());
            let mut cb = CheckoutBuilder::new();
            cb.safe().recreate_missing(true);
            repo.checkout_head(Some(&mut cb)).unwrap();
            println!("safe().recreate_missing(true): exists={}", td.path().join("f").exists());
        }
        "dry_run" => {
            let (td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            commit(&repo, &[(b"f", b"b\n")], "b");
            fs::write(td.path().join("f"), "dirty\n").unwrap();
            let o = repo.find_object(a, None).unwrap();
            let r = repo.checkout_tree(&o, Some(&mut CheckoutBuilder::new()));
            println!("safe: {:?}", r.map_err(|e| e.code()));
            let r = repo.checkout_tree(&o, Some(CheckoutBuilder::new().dry_run()));
            println!("dry_run: {:?}", r.map_err(|e| e.code()));
            let r = repo.checkout_tree(&o, Some(CheckoutBuilder::new().dry_run().force()));
            println!("dry_run.force: {:?} file {:?}", r.map_err(|e| e.code()), fs::read_to_string(td.path().join("f")));
        }
        "indexer_odb" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            let out = TempDir::new().unwrap();
            let mut idx = {
                let odb = Odb::new().unwrap();
                Indexer::new(Some(&odb), out.path(), 0, true).unwrap()
            };
            for _ in 0..50 { let _ = std::hint::black_box(vec![0u8; 256]); }
            println!("{:?}", idx.write(&buf).map(|_| ()));
            println!("{:?}", idx.commit());
        }
        "from_buffer_static" => {
            let v = b"diff --git a/f b/f\nindex 1111111..2222222 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n".to_vec();
            let d = Diff::from_buffer(&v).unwrap();
            drop(v);
            let mut out = vec![];
            d.print(DiffFormat::Patch, |_, _, l| { out.extend_from_slice(l.content()); true }).unwrap();
            println!("{}", String::from_utf8_lossy(&out));
        }
        "pack_write_leak" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let out = TempDir::new().unwrap();
            let before = LIVE.load(std::sync::atomic::Ordering::SeqCst);
            for _ in 0..2000 {
                let mut b = repo.packbuilder().unwrap();
                b.insert_commit(id).unwrap();
                b.write(out.path(), 0).unwrap();
            }
            let after = LIVE.load(std::sync::atomic::Ordering::SeqCst);
            println!("live rust allocs grew by {} after 2000 PackBuilder::write calls", after - before);
        }
        "refs_names_latin1" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(repo.path().join("refs/heads").join(std::ffi::OsStr::from_bytes(b"caf\xe9")), format!("{}\n", id)).unwrap();
            let mut refs = repo.references().unwrap();
            let v: Vec<_> = refs.names().collect();
            println!("{:?}", v);
        }
        "update_tips_latin1" => {
            let (_td, repo) = init();
            let (td2, repo2) = init();
            let id = commit(&repo2, &[(b"f", b"a\n")], "c");
            fs::write(td2.path().join(".git/refs/heads").join(std::ffi::OsStr::from_bytes(b"caf\xe9")), format!("{}\n", id)).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            let mut cb = RemoteCallbacks::new();
            let mut n = 0;
            cb.update_tips(|_, _, _| { n += 1; true });
            let mut fo = FetchOptions::new();
            fo.remote_callbacks(cb);
            println!("fetch {:?}", r.fetch(&["refs/heads/*:refs/remotes/o/*"], Some(&mut fo), None));
        }
        "pathspec_empty" => {
            let ps = Pathspec::new(["*"].iter()).unwrap();
            println!("{:?}", ps.matches_path(Path::new(""), PathspecFlags::DEFAULT));
        }
        "submodule_fuzz" => {
            for (i, gm) in [
                "[submodule \"a\"]\n\tpath = x\n\turl = ../y\n",
                "[submodule \"a\"]\n\tpath = x\n",
                "[submodule \"a\"]\n\turl = ../y\n",
                "[submodule \"a/b\"]\n\tpath = x/y\n\turl = u\n",
                "[submodule \"a\"]\n\tpath = x\n\tupdate = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n\tignore = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n\tfetchRecurseSubmodules = bogus\n",
                "[submodule \"a\"]\n\tpath = x\n[submodule \"b\"]\n\tpath = x\n",
                "[submodule \"A\"]\n\tpath = x\n\turl=u\n",
                "[submodule \"a\"]\n\tpath = \n",
            ].iter().enumerate() {
                let (td, repo) = init();
                fs::write(td.path().join(".gitmodules"), gm).unwrap();
                println!("case {} -> {:?}", i, repo.submodules().map(|v| v.len()).map_err(|e| e.message().to_string()));
            }
        }
        "time_ord" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let t = repo.find_commit(id).unwrap().tree().unwrap().id();
            let body = format!("tree {}\nauthor a <a@x> 1000 -0000\ncommitter a <a@x> 1000 -0000\n\nm\n", t);
            let odb = repo.odb().unwrap();
            let c = odb.write(ObjectType::Commit, body.as_bytes()).unwrap();
            let c = repo.find_commit(c).unwrap();
            let a = c.time();
            let b = c.committer().when();
            println!("time.sign {} sig.when.sign {} eq {} cmp {:?}", a.sign(), b.sign(), a == b, a.cmp(&b));
        }
        "index_flags_overread" => {
            let p = std::env::args().nth(2).unwrap();
            let i = Index::open(Path::new(&p)).unwrap();
            println!("entries {}", i.len());
            let e = i.get(0).unwrap();
            println!("path len {} head {:?}", e.path.len(), &e.path[..e.path.len().min(32)]);
        }
        "transport_err_msg" => {
            unsafe { git2::transport::register("xx", |_| Err(Error::from_str("factory says no"))).unwrap(); }
            let (_td, repo) = init();
            let mut r = repo.remote("o", "xx://h/p").unwrap();
            println!("{:?}", r.connect(Direction::Fetch).map_err(|e| e.to_string()));
        }
        "remote_create_err_msg" => {
            let td = TempDir::new().unwrap();
            let (td2, repo2) = init();
            commit(&repo2, &[(b"f", b"a\n")], "c");
            let mut b = git2::build::RepoBuilder::new();
            b.remote_create(|_, _, _| Err(Error::from_str("create says no")));
            println!("{:?}", b.clone(td2.path().to_str().unwrap(), &td.path().join("x")).map(|_| ()).map_err(|e| e.to_string()));
        }
