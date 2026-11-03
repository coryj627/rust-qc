        "index_cb_empty_pathspec" => {
            let (td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            fs::write(td.path().join("f"), "b\n").unwrap();
            fs::write(td.path().join("g"), "c\n").unwrap();
            let mut idx = repo.index().unwrap();
            let mut seen = 0;
            let mut cb = |_: &Path, _: &[u8]| -> i32 { seen += 1; 0 };
            let which = std::env::args().nth(2).unwrap();
            let r = match which.as_str() {
                "add" => idx.add_all(Vec::<&str>::new(), IndexAddOption::DEFAULT, Some(&mut cb)),
                "update" => idx.update_all(Vec::<&str>::new(), Some(&mut cb)),
                _ => idx.remove_all(Vec::<&str>::new(), Some(&mut cb)),
            };
            println!("{:?} seen {}", r, seen);
        }
        "fetchhead_forms" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            for (i, form) in [
                format!("{}\n", id),
                format!("{}\t\thttps://example.com/x.git\n", id),
                format!("{}\t\tbranch 'main' of https://example.com/x\n", id),
                format!("{}\tnot-for-merge\tbranch 'main'\n", id),
                format!("{}\t\t'refs/x' of https://example.com/x\n", id),
                format!("{}\t\ttag 'v1' of https://example.com/x\n", id),
                format!("{}\t\tremote-tracking branch 'o/m' of https://example.com/x\n", id),
            ].iter().enumerate() {
                fs::write(repo.path().join("FETCH_HEAD"), form).unwrap();
                let mut seen = vec![];
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    repo.fetchhead_foreach(|n, u, _, m| { seen.push((n.to_string(), String::from_utf8_lossy(u).to_string(), m)); true })
                }));
                println!("form {} -> {:?} seen {:?}", i, r.map(|x| x.is_ok()).map_err(|_| "panic"), seen);
            }
        }
        "stash_no_message" => {
            let (td, mut repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "c");
            fs::write(td.path().join("f"), "b\n").unwrap();
            let sig = repo.signature().unwrap();
            repo.stash_save(&sig, "m", None).unwrap();
            let rl = repo.path().join("logs/refs/stash");
            let d = fs::read_to_string(&rl).unwrap();
            let line = d.split('\t').next().unwrap().to_string() + "\n";
            fs::write(&rl, line).unwrap();
            let mut n = 0;
            let r = repo.stash_foreach(|_, m, _| { n += 1; println!("{:?}", m); true });
            println!("{:?} {}", r, n);
        }
        "diff_stress" => {
            for round in 0..40 {
                let (td, repo) = init();
                let mut ga = String::new();
                for i in 0..64 { ga.push_str(&format!("*.e{} diff=drv{}\n", i, i)); }
                let mut files: Vec<(Vec<u8>, Vec<u8>)> = vec![(b".gitattributes".to_vec(), ga.into_bytes())];
                for i in 0..64 { files.push((format!("f.e{}", i).into_bytes(), b"a\n".to_vec())); }
                let refs: Vec<(&[u8], &[u8])> = files.iter().map(|(a,b)| (&a[..], &b[..])).collect();
                commit(&repo, &refs, "base");
                for i in 0..64 { fs::write(td.path().join(format!("f.e{}", i)), "b\n").unwrap(); }
                let mut ds: Vec<Diff> = (0..4).map(|_| repo.diff_index_to_workdir(None, None).unwrap()).collect();
                std::thread::scope(|s| {
                    for d in ds.drain(..) {
                        s.spawn(move || { let mut n = 0; d.print(DiffFormat::Patch, |_, _, _| { n += 1; true }).unwrap(); assert!(n > 0); });
                    }
                });
                if round % 10 == 0 { println!("round {}", round); }
            }
        }
