        "parents_fused" => {
            let (_td, repo) = init();
            let a = commit(&repo, &[(b"f", b"a\n")], "a");
            let t = repo.find_commit(a).unwrap().tree().unwrap().id();
            let missing = Oid::from_str("1111111111111111111111111111111111111111").unwrap();
            let body = format!("tree {t}\nparent {missing}\nparent {a}\nauthor a <a@x> 1000 +0000\ncommitter a <a@x> 1000 +0000\n\nm\n");
            let c = repo.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
            let c = repo.find_commit(c).unwrap();
            let mut it = c.parents();
            println!("len {} first {:?} second {:?}", it.len(), it.next().map(|p| p.id()), it.next().map(|p| p.id()));
        }
        "index_alias" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n"), (b"g", b"a\n")], "a");
            let a = repo.index().unwrap();
            let mut b = repo.index().unwrap();
            let mut it = a.iter();
            b.clear().unwrap();
            println!("{:?}", it.next().map(|e| e.path));
        }
