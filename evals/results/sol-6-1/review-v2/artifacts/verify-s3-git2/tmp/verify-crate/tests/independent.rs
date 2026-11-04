use git2::*;
use std::{fs, path::Path, cell::Cell, cmp::Ordering};
use git2::build::CheckoutBuilder;
fn repo() -> (tempfile::TempDir, Repository) {
 let td=tempfile::tempdir().unwrap(); let r=Repository::init(td.path()).unwrap();
 r.config().unwrap().set_str("user.name","Audit").unwrap(); r.config().unwrap().set_str("user.email","a@example.test").unwrap(); (td,r)
}
fn commit(r:&Repository, content:&[u8], parents:&[Oid]) -> Oid {
 let blob=r.blob(content).unwrap(); let mut b=r.treebuilder(None).unwrap(); b.insert("file",blob,0o100644).unwrap(); let tree=r.find_tree(b.write().unwrap()).unwrap(); let sig=Signature::new("A","a@b",&Time::new(123456,0)).unwrap();
 let ps:Vec<_>=parents.iter().map(|id|r.find_commit(*id).unwrap()).collect(); let refs:Vec<_>=ps.iter().collect(); r.commit(None,&sig,&sig,"message",&tree,&refs).unwrap()
}
fn raw_commit(r:&Repository, tree:Oid, parents:&[Oid], sign:&str, msg:&str)->Oid {
 let mut s=format!("tree {}\n",tree); for p in parents {s.push_str(&format!("parent {}\n",p));}
 s.push_str(&format!("author A <a@b> 123456 {}\ncommitter A <a@b> 123456 {}\n\n{}",sign,sign,msg));
 r.odb().unwrap().write(ObjectType::Commit,s.as_bytes()).unwrap()
}
#[test] fn conditional_reference_force() {
 let (_td,r)=repo(); let a=commit(&r,b"a",&[]); let b=commit(&r,b"b",&[a]); let c=commit(&r,b"c",&[b]); r.reference("refs/heads/topic",b,true,"create").unwrap();
 let err=r.reference_matching("refs/heads/topic",c,true,a,"update").err().unwrap(); println!("err={:?} target={:?}",err.code(),r.refname_to_id("refs/heads/topic")); assert_eq!(err.code(),ErrorCode::Modified); assert_eq!(r.refname_to_id("refs/heads/topic").unwrap(),b);
}
#[test] fn raw_message_preserves_newlines() {
 let (_td,r)=repo(); let tree=r.treebuilder(None).unwrap().write().unwrap(); let msg="\n\nsubject\n\nbody\n"; let id=raw_commit(&r,tree,&[],"+0000",msg); let c=r.find_commit(id).unwrap(); println!("raw={:?} cooked={:?}",c.message_raw(),c.message()); assert_eq!(c.message_raw_bytes(),msg.as_bytes()); assert_eq!(c.message_raw(),Some(msg));
}
#[test] fn range_excludes_left() {
 let (_td,r)=repo(); let a=commit(&r,b"a",&[]); let b=commit(&r,b"b",&[a]); let c=commit(&r,b"c",&[b]); let mut w=r.revwalk().unwrap(); w.push_range(&format!("{}..{}",b,c)).unwrap(); let got=w.collect::<Result<Vec<_>,_>>().unwrap(); println!("range={:?}",got); assert_eq!(got,vec![c]);
}
#[test] fn status_can_disable_untracked() {
 let (td,r)=repo(); fs::write(td.path().join("untracked"),b"x").unwrap(); let mut o=StatusOptions::new(); o.include_untracked(true); assert_eq!(r.statuses(Some(&mut o)).unwrap().len(),1); o.include_untracked(false); assert_eq!(r.statuses(Some(&mut o)).unwrap().len(),0);
}
#[test] fn buffer_index_replaces_id_size() {
 let (td,r)=repo(); fs::write(td.path().join("file"),b"x").unwrap(); let mut i=r.index().unwrap(); i.add_path(Path::new("file")).unwrap(); let e=i.get_path(Path::new("file"),0).unwrap(); let data=b"twenty bytes content"; assert_eq!(data.len(),20); i.add_frombuffer(&e,data).unwrap(); let got=i.get_path(Path::new("file"),0).unwrap(); println!("old={} got={} size={}",e.id,got.id,got.file_size); assert_eq!(got.id,r.blob(data).unwrap()); assert_eq!(got.file_size,20); let t=r.find_tree(i.write_tree().unwrap()).unwrap(); assert_eq!(r.find_blob(t.get_name("file").unwrap().id()).unwrap().content(),data);
}
#[test] fn nullable_annotated_refname() { let (_td,r)=repo(); let id=commit(&r,b"x",&[]); assert_eq!(r.find_annotated_commit(id).unwrap().refname(),None); }
#[test] fn nullable_refspec_destination() { let (_td,r)=repo(); r.remote("origin","/unused").unwrap(); r.remote_add_fetch("origin","refs/heads/topic").unwrap(); let remote=r.find_remote("origin").unwrap(); let spec=remote.refspecs().find(|x| x.str()==Some("refs/heads/topic")).unwrap(); assert_eq!(spec.dst(),None); }
#[test] fn nullable_rebase_exec() {
 let (_td,r)=repo(); let a=commit(&r,b"a",&[]); let b=commit(&r,b"b",&[a]); let base=r.find_annotated_commit(a).unwrap(); let branch=r.find_annotated_commit(b).unwrap(); let mut opts=RebaseOptions::new(); opts.inmemory(true); let mut rebase=r.rebase(Some(&branch),Some(&base),None,Some(&mut opts)).unwrap(); let op=rebase.next().unwrap().unwrap(); assert_eq!(op.kind(),Some(RebaseOperationType::Pick)); assert_eq!(op.exec(),None);
}
#[test] fn non_utf8_prettify_returns_error() { assert!(message_prettify(&b"caf\xe9\n"[..],None).is_err()); }
#[test] fn time_eq_ord_agree() {
 let (_td,r)=repo(); let tree=r.treebuilder(None).unwrap().write().unwrap(); let m=raw_commit(&r,tree,&[],"-0000","m"); let p=raw_commit(&r,tree,&[],"+0000","m"); let t1=r.find_commit(m).unwrap().author().when(); let t2=r.find_commit(p).unwrap().author().when(); println!("times={:?} {:?} eq={} cmp={:?}",t1,t2,t1==t2,t1.cmp(&t2)); assert_eq!(t1==t2,t1.cmp(&t2)==Ordering::Equal);
}
#[test] fn parent_iteration_stops() {
 let (_td,r)=repo(); let a=commit(&r,b"a",&[]); let tree=r.find_commit(a).unwrap().tree_id(); let missing=Oid::from_str("1111111111111111111111111111111111111111").unwrap(); let id=raw_commit(&r,tree,&[missing,a],"+0000","merge"); let c=r.find_commit(id).unwrap(); let mut ps=c.parents(); println!("initial size_hint={:?}",ps.size_hint()); assert!(ps.next().is_none()); let next=ps.next(); println!("second={:?}",next); assert!(next.is_none());
}
#[test] fn refspec_lifetime_compile() {
 fn escape<'a>(r:&'a Repository)->Refspec<'a> { let rem=r.remote_anonymous("/unused").unwrap(); rem.get_refspec(0).unwrap() }
 let _f=escape;
}
#[test] fn rebase_operation_lifetime_compile() {
 fn escape<'a>(r:&'a Repository)->RebaseOperation<'a> { let mut opts=RebaseOptions::new(); opts.inmemory(true); let mut rebase=r.rebase(None,None,None,Some(&mut opts)).unwrap(); rebase.next().unwrap().unwrap() }
 let _f=escape;
}
#[test] fn diff_match_lifetime_compile() {
 fn escape<'a>(r:&Repository,p:&'a Pathspec)->PathspecMatchList<'a> { let diff=r.diff_tree_to_workdir(None,None).unwrap(); p.match_diff(&diff,PathspecFlags::DEFAULT).unwrap() }
 let _f=escape;
}
#[test] fn rebase_options_lifetime_compile() {
 fn escape(r:&Repository)->Rebase<'_> { let mut count=0; let mut checkout=CheckoutBuilder::new(); checkout.progress(|_,_,_| count+=1); let mut opts=RebaseOptions::new(); opts.checkout_options(checkout); r.rebase(None,None,None,Some(&mut opts)).unwrap() }
 let _f=escape;
}
#[test] fn predicates_defaults_and_modes() {
 println!("none: sort={} analysis={} preference={} umask={} group_as_all={}",Sort::NONE.is_none(),MergeAnalysis::ANALYSIS_NONE.is_none(),MergePreference::NONE.is_none(),RepositoryInitMode::SHARED_UMASK.is_shared_umask(),RepositoryInitMode::SHARED_GROUP.is_shared_all());
 assert!(Sort::NONE.is_none()); assert!(MergeAnalysis::ANALYSIS_NONE.is_none()); assert!(MergePreference::NONE.is_none()); assert!(RepositoryInitMode::SHARED_UMASK.is_shared_umask()); assert!(!RepositoryInitMode::SHARED_GROUP.is_shared_all());
}
#[test] fn pathspec_relative_dot() { let p=Pathspec::new(["*.rs"]).unwrap(); assert!(p.matches_path(Path::new("lib.rs"),PathspecFlags::DEFAULT)); assert!(p.matches_path(Path::new("./lib.rs"),PathspecFlags::DEFAULT)); }
#[test] fn index_relative_dot() { let (td,r)=repo(); fs::write(td.path().join("lib.rs"),b"x").unwrap(); let mut i=r.index().unwrap(); i.add_path(Path::new("lib.rs")).unwrap(); assert!(i.get_path(Path::new("lib.rs"),0).is_some()); assert!(i.get_path(Path::new("./lib.rs"),0).is_some()); }
#[test] fn revwalk_callback_detaches() {
 let (_td,r)=repo(); let a=commit(&r,b"a",&[]); let b=commit(&r,b"b",&[a]); let calls=Cell::new(0); let mut cb=|_|{calls.set(calls.get()+1); true}; let w=r.revwalk().unwrap(); let mut w=w.with_hide_callback(&mut cb).unwrap().into_inner().unwrap(); w.push(b).unwrap(); let ids=w.collect::<Result<Vec<_>,_>>().unwrap(); println!("calls={} ids={:?}",calls.get(),ids); assert_eq!(calls.get(),0); assert_eq!(ids.len(),2);
}
#[test] fn skip_reuc_not_conflict_policy() {
 let (_td,r)=repo(); let a=commit(&r,b"base\n",&[]); let b=commit(&r,b"ours\n",&[a]); let c=commit(&r,b"theirs\n",&[a]); let at=r.find_commit(a).unwrap().tree().unwrap(); let bt=r.find_commit(b).unwrap().tree().unwrap(); let ct=r.find_commit(c).unwrap().tree().unwrap(); assert!(r.merge_trees(&at,&bt,&ct,None).unwrap().has_conflicts()); let mut o=MergeOptions::new(); o.skip_reuc(true); let got=r.merge_trees(&at,&bt,&ct,Some(&o)); println!("skip_reuc result={:?}",got.as_ref().err()); assert!(got.is_ok());
}
#[test] fn nonempty_pack_hash() {
 let (_td,r)=repo(); let id=r.blob(b"blob").unwrap(); let out=tempfile::tempdir().unwrap(); let mut p=r.packbuilder().unwrap(); p.insert_object(id,None).unwrap(); p.write(out.path(),0).unwrap(); println!("name={:?} count={}",p.name(),p.object_count()); assert!(p.name().is_some()); #[allow(deprecated)] let hash=p.hash(); assert!(hash.is_some());
}
#[test] fn dry_run_conflict() {
 let (td,r)=repo(); let a=commit(&r,b"base\n",&[]); let b=commit(&r,b"next\n",&[a]); r.reference("refs/heads/main",a,true,"init").unwrap(); r.set_head("refs/heads/main").unwrap(); r.checkout_head(Some(CheckoutBuilder::new().force())).unwrap(); fs::write(td.path().join("file"),b"local change\n").unwrap(); let target=r.find_object(b,None).unwrap(); assert!(r.checkout_tree(&target,Some(CheckoutBuilder::new().safe())).is_err()); let mut o=CheckoutBuilder::new(); o.dry_run(); let result=r.checkout_tree(&target,Some(&mut o)); println!("dry run={:?}",result); assert!(result.is_err());
}
#[test] fn force_after_dry_run() {
 let (td,r)=repo(); let a=commit(&r,b"base\n",&[]); let target=r.find_object(a,None).unwrap(); let mut o=CheckoutBuilder::new(); o.dry_run().force(); r.checkout_tree(&target,Some(&mut o)).unwrap(); assert!(td.path().join("file").exists());
}
#[test] fn moved_apply_callbacks() {
 let (td,r)=repo(); fs::write(td.path().join("file"),b"old\n").unwrap(); let mut i=r.index().unwrap(); i.add_path(Path::new("file")).unwrap(); i.write().unwrap(); fs::write(td.path().join("file"),b"new\n").unwrap(); let diff=r.diff_index_to_workdir(None,None).unwrap(); let n1=Cell::new(0); let n2=Cell::new(0); let mut one=ApplyOptions::new(); let mut two=ApplyOptions::new(); one.delta_callback(|_| {n1.set(n1.get()+1); true}); two.delta_callback(|_| {n2.set(n2.get()+1); true}); std::mem::swap(&mut one,&mut two); r.apply(&diff,ApplyLocation::Index,Some(&mut one)).unwrap(); println!("counts=({}, {})",n1.get(),n2.get()); assert_eq!((n1.get(),n2.get()),(0,1));
}
#[test] fn moved_stash_callbacks() {
 let (td,mut r)=repo(); let a=commit(&r,b"base\n",&[]); r.reference("refs/heads/main",a,true,"init").unwrap(); r.set_head("refs/heads/main").unwrap(); r.checkout_head(Some(CheckoutBuilder::new().force())).unwrap(); fs::write(td.path().join("file"),b"changed\n").unwrap(); let sig=r.signature().unwrap(); r.stash_save(&sig,"stash",None).unwrap(); let n1=Cell::new(0); let n2=Cell::new(0); let mut one=StashApplyOptions::new(); let mut two=StashApplyOptions::new(); one.progress_cb(|_| {n1.set(n1.get()+1); true}); two.progress_cb(|_| {n2.set(n2.get()+1); true}); std::mem::swap(&mut one,&mut two); r.stash_apply(0,Some(&mut one)).unwrap(); println!("stash counts=({}, {})",n1.get(),n2.get()); assert_eq!(n1.get(),0); assert!(n2.get()>0);
}

fn helper_cfg() -> (tempfile::TempDir, Config) {let td=tempfile::tempdir().unwrap(); let path=td.path().join("config"); fs::write(&path,"").unwrap(); let cfg=Config::open(&path).unwrap(); (td,cfg)}
#[test] fn credentials_multiple_helpers() {
 let td=tempfile::tempdir().unwrap(); let path=td.path().join("config");
 fs::write(&path, "[credential]\n helper = \"!f() { echo username=alice; }; f\"\n helper = \"!f() { echo password=dummy; }; f\"\n").unwrap();
 let cfg=Config::open(&path).unwrap(); let mut entries=cfg.entries(Some("credential.helper")).unwrap(); let mut count=0; while let Some(e)=entries.next() {println!("helper={:?}",e.unwrap().value());count+=1;} assert_eq!(count,2);
 let got=CredentialHelper::new("https://example.test/repo").config(&cfg).execute(); println!("multiple helpers={:?}",got); assert_eq!(got,Some(("alice".into(),"dummy".into())));
}
#[test] fn credentials_host_variants() {
 let (_td,mut cfg)=helper_cfg(); cfg.set_str("credential.helper","!f() { seen=; while IFS= read -r line; do case \"$line\" in host=*) seen=yes;; esac; done; if test \"$seen\" = yes; then printf 'username=alice\npassword=dummy\n'; fi; }; f").unwrap();
 let expected=Some(("alice".into(),"dummy".into())); let domain=CredentialHelper::new("https://example.test:8443/repo").config(&cfg).execute(); let ipv4=CredentialHelper::new("https://127.0.0.1:8443/repo").config(&cfg).execute(); let ipv6=CredentialHelper::new("https://[::1]:8443/repo").config(&cfg).execute(); println!("hosts domain={:?} ipv4={:?} ipv6={:?}",domain,ipv4,ipv6); assert_eq!(domain,expected); assert_eq!(ipv4,expected); assert_eq!(ipv6,expected);
}
#[test] fn credentials_single_helper_control() {
 let (_td,mut cfg)=helper_cfg(); cfg.set_str("credential.helper","!f() { printf 'username=alice\npassword=dummy\n'; }; f").unwrap(); assert_eq!(CredentialHelper::new("https://example.test/repo").config(&cfg).execute(),Some(("alice".into(),"dummy".into())));
}
