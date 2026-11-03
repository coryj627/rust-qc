        "pathspec_diff_dangling" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let ps = Pathspec::new(["f"].iter()).unwrap();
            let list = {
                let d = repo.diff_index_to_workdir(None, None).unwrap();
                ps.match_diff(&d, PathspecFlags::DEFAULT).unwrap()
            };
            let e = list.diff_entry(0).unwrap();
            println!("{:?}", e.old_file().path());
        }
        "reflog_after_repo" => {
            let mut log = {
                let (_td, repo) = init();
                commit(&repo, &[(b"f", b"a\n")], "base");
                repo.reflog("HEAD").unwrap()
            };
            println!("entries {}", log.len());
            println!("{:?}", log.write());
        }
        "packwriter_double" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            let (_td2, repo2) = init();
            let odb = repo2.odb().unwrap();
            let mut pw = odb.packwriter().unwrap();
            pw.write_all(&buf).unwrap();
            println!("commit1 {:?}", pw.commit());
            println!("commit2 {:?}", pw.commit());
            println!("write2 {:?}", pw.write(&buf).map(|_| ()));
        }
        "packbuilder_reuse" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "c");
            let id2 = commit(&repo, &[(b"f", b"b\n")], "d");
            let mut b = repo.packbuilder().unwrap();
            let mut buf = Buf::new();
            b.insert_commit(id).unwrap();
            b.write_buf(&mut buf).unwrap();
            println!("first {}", buf.len());
            println!("second write_buf {:?}", b.write_buf(&mut buf).map(|_| buf.len()));
            println!("insert after {:?}", b.insert_commit(id2));
            println!("third write_buf {:?}", b.write_buf(&mut buf).map(|_| buf.len()));
            println!("foreach {:?}", b.foreach(|_| true));
        }
        "rebase_after_finish" => {
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
            let mut r = repo.rebase(Some(&one), Some(&up), None, None).unwrap();
            let sig = repo.signature().unwrap();
            while let Some(op) = r.next() { op.unwrap(); r.commit(None, &sig, None).unwrap(); }
            println!("finish {:?}", r.finish(None));
            println!("finish2 {:?}", r.finish(None).map_err(|e| e.message().to_string()));
            println!("abort {:?}", r.abort().map_err(|e| e.message().to_string()));
            println!("next {:?}", r.next().map(|x| x.map(|o| o.id()).map_err(|e| e.message().to_string())));
            println!("commit {:?}", r.commit(None, &sig, None).map_err(|e| e.message().to_string()));
        }
        "diff_send_race" => {
            let (td, repo) = init();
            let mut files: Vec<(Vec<u8>, Vec<u8>)> = vec![(b".gitattributes".to_vec(), b"*.x diff=foo\n*.y diff=bar\n*.z diff=baz\n".to_vec())];
            for i in 0..200 { files.push((format!("f{}.{}", i, ["x","y","z"][i%3]).into_bytes(), b"a\n".to_vec())); }
            let refs: Vec<(&[u8], &[u8])> = files.iter().map(|(a,b)| (&a[..], &b[..])).collect();
            commit(&repo, &refs, "base");
            for i in 0..200 { fs::write(td.path().join(format!("f{}.{}", i, ["x","y","z"][i%3])), "b\n").unwrap(); }
            let d1 = repo.diff_index_to_workdir(None, None).unwrap();
            let d2 = repo.diff_index_to_workdir(None, None).unwrap();
            std::thread::scope(|s| {
                s.spawn(move || { let mut n = 0; d1.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); println!("t1 {}", n); });
                s.spawn(move || { let mut n = 0; d2.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); println!("t2 {}", n); });
            });
        }
        "opts_race" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            std::thread::scope(|s| {
                s.spawn(|| { for i in 0..2000 { git2::opts::enable_caching(i % 2 == 0); git2::opts::strict_hash_verification(i % 2 == 0); } });
                let p = repo.path().to_path_buf();
                s.spawn(move || { let r = Repository::open(p).unwrap(); for _ in 0..500 { let _ = r.find_commit(id); } });
            });
        }
