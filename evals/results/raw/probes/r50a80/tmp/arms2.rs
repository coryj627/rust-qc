        "skip_reuc" => {
            let (_td, repo) = init();
            let base = commit(&repo, &[(b"f", b"base\n")], "base");
            let ours = commit(&repo, &[(b"f", b"ours\n")], "ours");
            repo.set_head_detached(base).unwrap();
            let theirs = commit(&repo, &[(b"f", b"theirs\n")], "theirs");
            let (o, t) = (repo.find_commit(ours).unwrap(), repo.find_commit(theirs).unwrap());
            let r = repo.merge_commits(&o, &t, None);
            println!("default: {:?}", r.map(|i| i.has_conflicts()).map_err(|e| e.code()));
            let mut m = MergeOptions::new();
            m.skip_reuc(true);
            let r = repo.merge_commits(&o, &t, Some(&m));
            println!("skip_reuc(true): {:?}", r.map(|i| i.has_conflicts()).map_err(|e| e.code()));
        }
        "cred_inject" => {
            let td = TempDir::new().unwrap();
            let log = td.path().join("stdin.log");
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "").unwrap();
            let mut cfg = Config::open(&cfgp).unwrap();
            cfg.set_str("credential.helper", &format!("!f() {{ cat > {}; echo username=u; echo password=p; }}; f", log.display())).unwrap();
            let r = CredentialHelper::new("https://real.example/repo").config(&cfg).username(Some("a\nhost=evil.example")).execute();
            println!("{:?}", r);
            println!("helper stdin: {:?}", fs::read_to_string(&log));
            let r = CredentialHelper::new("https://10.0.0.1:8443/repo").config(&cfg).execute();
            println!("{:?}", r);
            println!("ip helper stdin: {:?}", fs::read_to_string(&log));
        }
        "cred_multi" => {
            let td = TempDir::new().unwrap();
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "[credential]\n\thelper = \"!f() { echo username=u; echo password=p; }; f\"\n\thelper = \"!f() { exit 1; }; f\"\n").unwrap();
            let cfg = Config::open(&cfgp).unwrap();
            println!("{:?}", CredentialHelper::new("https://real.example/repo").config(&cfg).execute());
        }
        "cred_empty_nosh" => {
            let td = TempDir::new().unwrap();
            let cfgp = td.path().join("cfg");
            fs::write(&cfgp, "[credential]\n\thelper = !\n").unwrap();
            let cfg = Config::open(&cfgp).unwrap();
            unsafe { std::env::set_var("PATH", "/nonexistent"); }
            println!("{:?}", CredentialHelper::new("https://real.example/repo").config(&cfg).execute());
        }
        "http_username_newline" => {
            use std::io::{Read, Write};
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = l.local_addr().unwrap().port();
            std::thread::spawn(move || {
                for s in l.incoming() {
                    let mut s = s.unwrap();
                    let mut b = [0u8; 4096];
                    let _ = s.read(&mut b);
                    let _ = s.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                }
            });
            let (_td, repo) = init();
            let url = format!("http://a%0Ahost=evil.example@127.0.0.1:{}/r.git", port);
            let mut r = repo.remote_anonymous(&url).unwrap();
            let mut cb = RemoteCallbacks::new();
            cb.credentials(|u, name, _| {
                println!("callback url={:?} username_from_url={:?}", u, name);
                Err(Error::from_str("stop"))
            });
            let c = r.connect_auth(Direction::Fetch, Some(cb), None);
            println!("{:?}", c.map(|_| ()).map_err(|e| e.message().to_string()));
        }
        "empty_bufs" => {
            let (_td, repo) = init();
            let id = commit(&repo, &[(b"f", b"a\n")], "base");
            let c = repo.find_commit(id).unwrap();
            let t = c.tree().unwrap();
            let d = repo.diff_tree_to_tree(Some(&t), Some(&t), None).unwrap();
            let which = std::env::args().nth(2).unwrap();
            match which.as_str() {
                "stats" => { let b = d.stats().unwrap().to_buf(DiffStatsFormat::FULL, 80).unwrap(); println!("{:?}", b.as_str()); }
                "patch" => { let mut p = Patch::from_buffers(b"a\n", None, b"a\n", None, None).unwrap(); let b = p.to_buf().unwrap(); println!("{:?}", b.as_str()); }
                "print" => { let b = d.stats().unwrap().to_buf(DiffStatsFormat::SHORT, 80).unwrap(); println!("{:?}", b.len()); }
                "blob" => { let i = repo.blob(b"").unwrap(); let b = repo.find_blob(i).unwrap(); println!("{:?}", b.content()); let o = repo.odb().unwrap().read(i).unwrap(); println!("{:?}", o.data()); }
                "merge_empty" => {
                    let e = repo.blob(b"").unwrap();
                    let mk = |id| IndexEntry { ctime: IndexTime::new(0,0), mtime: IndexTime::new(0,0), dev:0, ino:0, mode:0o100644, uid:0, gid:0, file_size:0, id, flags:0, flags_extended:0, path: b"f".to_vec() };
                    let (a, b, c2) = (mk(e), mk(e), mk(e));
                    let r = repo.merge_file_from_index(&a, &b, &c2, None).unwrap();
                    println!("{:?}", r.content());
                }
                "header" => {
                    let body = format!("tree {}\nauthor a <a@x> 1000 +0000\ncommitter a <a@x> 1000 +0000\nencoding \n\nm\n", t.id());
                    let oid = repo.odb().unwrap().write(ObjectType::Commit, body.as_bytes()).unwrap();
                    let c = repo.find_commit(oid).unwrap();
                    println!("{:?}", c.header_field_bytes("encoding").map(|b| b.len()));
                }
                _ => panic!(),
            }
        }
        "push_negotiation_empty" => {
            let (_td, repo) = init();
            commit(&repo, &[(b"f", b"a\n")], "base");
            let td2 = TempDir::new().unwrap();
            Repository::init_bare(td2.path()).unwrap();
            let mut r = repo.remote("o", td2.path().to_str().unwrap()).unwrap();
            let mut cb = RemoteCallbacks::new();
            cb.push_negotiation(|u| { println!("neg {}", u.len()); Ok(()) });
            let mut po = PushOptions::new();
            po.remote_callbacks(cb);
            println!("{:?}", r.push::<&str>(&[], Some(&mut po)));
        }
        "transport_io_nul" => {
            use git2::transport::*;
            struct S;
            impl std::io::Read for S { fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { Err(std::io::Error::new(std::io::ErrorKind::Other, "bad\0byte")) } }
            impl std::io::Write for S { fn write(&mut self, b: &[u8]) -> std::io::Result<usize> { Ok(b.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }
            struct T;
            impl SmartSubtransport for T {
                fn action(&self, _: &str, _: Service) -> Result<Box<dyn SmartSubtransportStream>, Error> { Ok(Box::new(S)) }
                fn close(&self) -> Result<(), Error> { Ok(()) }
            }
            unsafe { register("yy", |r| Transport::smart(r, false, T)).unwrap(); }
            let (_td, repo) = init();
            let mut r = repo.remote("o", "yy://h/p").unwrap();
            println!("{:?}", r.connect(Direction::Fetch).map_err(|e| e.to_string()));
        }
        "diff_mode_panic" => {
            let d = Diff::from_buffer(b"diff --git a/f b/f\nold mode 100600\nnew mode 100777\n").unwrap();
            for x in d.deltas() { println!("{:?} {:?}", x.status(), x.old_file().path()); println!("{:?}", x.old_file().mode()); }
        }
