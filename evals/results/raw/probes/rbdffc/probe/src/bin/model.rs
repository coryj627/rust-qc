use probe::*;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use zip::unstable::write::FileOptionsExt;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod as M, ZipArchive, ZipWriter};
#[derive(Clone, Debug)]
struct Ent { name: String, content: Vec<u8>, kind: u8, enc: u8, large: bool }
fn check<R: Read + std::io::Seek>(a: &mut ZipArchive<R>, model: &[Ent]) -> Result<(), String> {
    if a.len() != model.len() { return Err(format!("len {} vs {}", a.len(), model.len())); }
    for m in model {
        let r = match m.enc { 0 => a.by_name(&m.name), _ => a.by_name_decrypt(&m.name, b"pw") };
        let mut f = r.map_err(|e| format!("open {}: {e}", m.name))?;
        if m.kind == 1 && !f.is_dir() { return Err(format!("{} not dir", m.name)); }
        if m.kind == 2 && !f.is_symlink() { return Err(format!("{} not symlink", m.name)); }
        let mut o = vec![]; f.read_to_end(&mut o).map_err(|e| format!("read {} enc={} large={}: {e}", m.name, m.enc, m.large))?;
        if o != m.content { return Err(format!("content {} {}!={} enc={} large={}", m.name, o.len(), m.content.len(), m.enc, m.large)); }
    }
    Ok(())
}
fn one(seed: u64) -> Result<(), String> {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    let mut model: Vec<Ent> = vec![]; let mut c = 0; let mut trace = vec![];
    let methods = [M::Stored, M::Deflated, M::Bzip2, M::Zstd, M::Xz, M::Ppmd];
    for _ in 0..r.below(14) {
        c += 1; let name = format!("f{c}");
        match r.below(10) {
            0..=4 => {
                let m = methods[r.below(6) as usize];
                let large = r.below(4) == 0;
                let enc = r.below(4) as u8; // 0,1 plain; 2 aes; 3 zipcrypto
                let al = [1u16, 2, 8, 512, 4096][r.below(5) as usize];
                let o = SimpleFileOptions::default().compression_method(m).large_file(large).with_alignment(al);
                let n = if m == M::Xz { [1usize, 19, 20, 21, 5000][r.below(5) as usize] } else { [0usize, 1, 19, 20, 21, 5000][r.below(6) as usize] };
                let content: Vec<u8> = (0..n).map(|i| (i % 37) as u8 ^ (r.next() as u8 & 3)).collect();
                trace.push(format!("start {name} {m:?} large={large} enc={enc} al={al} n={n}"));
                let res = match enc { 2 => w.start_file(name.clone(), o.with_aes_encryption(AesMode::Aes192, "pw")), 3 => w.start_file(name.clone(), o.with_deprecated_encryption(b"pw")), _ => w.start_file(name.clone(), o) };
                res.map_err(|e| format!("start: {e} {trace:?}"))?;
                w.write_all(&content).map_err(|e| format!("write: {e} {trace:?}"))?;
                model.push(Ent { name, content, kind: 0, enc: if enc >= 2 { enc } else { 0 }, large });
            }
            5 => { if !model.is_empty() { trace.push("abort".into()); w.abort_file().map_err(|e| format!("abort: {e} {trace:?}"))?; model.pop(); } }
            6 => { let dn = format!("{name}/"); trace.push(format!("dir {dn}")); w.add_directory(dn.clone(), SimpleFileOptions::default()).map_err(|e| format!("dir: {e} {trace:?}"))?; model.push(Ent { name: dn, content: vec![], kind: 1, enc: 0, large: false }); }
            7 => { trace.push(format!("sym {name}")); w.add_symlink(name.clone(), "tgt", SimpleFileOptions::default()).map_err(|e| format!("sym: {e} {trace:?}"))?; model.push(Ent { name, content: b"tgt".to_vec(), kind: 2, enc: 0, large: false }); }
            8 => { if let Some(s) = model.iter().filter(|e| e.kind == 0).last().cloned() { trace.push(format!("shallow {} {name}", s.name)); w.shallow_copy_file(&s.name, &name).map_err(|e| format!("shallow: {e} {trace:?}"))?; model.push(Ent { name, ..s }); } }
            _ => { if let Some(s) = model.iter().filter(|e| e.kind == 0 && !e.large).last().cloned() { trace.push(format!("deep {} {name}", s.name)); w.deep_copy_file(&s.name, &name).map_err(|e| format!("deep: {e} {trace:?}"))?; model.push(Ent { name, ..s }); } }
        }
    }
    if model.iter().all(|m| !m.large) && seed % 2 == 0 {
        let mut a = w.finish_into_readable().map_err(|e| format!("readable: {e} {trace:?}"))?;
        check(&mut a, &model).map_err(|e| format!("R {e} {trace:?}"))?;
        let v = a.into_inner().into_inner();
        let mut a = ZipArchive::new(Cursor::new(v)).map_err(|e| format!("reopen: {e} {trace:?}"))?;
        check(&mut a, &model).map_err(|e| format!("R2 {e} {trace:?}"))?;
        return Ok(());
    }
    let v = w.finish().map_err(|e| format!("finish: {e} {trace:?}"))?.into_inner();
    let mut a = ZipArchive::new(Cursor::new(v.clone())).map_err(|e| format!("reopen: {e} {trace:?}"))?;
    check(&mut a, &model).map_err(|e| format!("A {e} {trace:?}"))?;
    // stream reader on unencrypted-only archives is checked separately. Append round trip:
    let mut w = ZipWriter::new_append(Cursor::new(v.clone())).map_err(|e| format!("append-open: {e}"))?;
    w.start_file("appended", SimpleFileOptions::default()).map_err(|e| format!("append-start: {e}"))?; w.write_all(b"zz").unwrap();
    let v2 = w.finish().map_err(|e| format!("append-finish: {e}"))?.into_inner();
    let mut a = ZipArchive::new(Cursor::new(v2)).map_err(|e| format!("append-reopen: {e} {trace:?}"))?;
    let mut m2 = model.clone(); m2.push(Ent { name: "appended".into(), content: b"zz".to_vec(), kind: 0, enc: 0, large: false });
    check(&mut a, &m2).map_err(|e| format!("B(append) {e} {trace:?}"))?;
    // merge into a writer with prior content
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    w.start_file("prior", SimpleFileOptions::default()).unwrap(); w.write_all(b"pp").unwrap();
    w.merge_archive(ZipArchive::new(Cursor::new(v.clone())).unwrap()).map_err(|e| format!("merge: {e}"))?;
    w.start_file("post", SimpleFileOptions::default()).map_err(|e| format!("post: {e}"))?; w.write_all(b"qq").unwrap();
    let v3 = w.finish().map_err(|e| format!("mfinish: {e}"))?.into_inner();
    let mut a = ZipArchive::new(Cursor::new(v3)).map_err(|e| format!("merge-reopen: {e}"))?;
    let mut m3 = model.clone(); m3.push(Ent { name: "prior".into(), content: b"pp".to_vec(), kind: 0, enc: 0, large: false }); m3.push(Ent { name: "post".into(), content: b"qq".to_vec(), kind: 0, enc: 0, large: false });
    check(&mut a, &m3).map_err(|e| format!("C(merge) {e} {trace:?}"))?;
    // raw copy of every entry
    let mut a = ZipArchive::new(Cursor::new(v)).unwrap();
    let mut w = ZipWriter::new(Cursor::new(vec![]));
    for m in &model { if m.enc == 3 || m.kind == 2 { continue; } let f = a.by_name(&m.name).err(); let _ = f; let i = a.index_for_name(&m.name).unwrap(); w.raw_copy_file(a.by_index_raw(i).unwrap()).map_err(|e| format!("rawcopy: {e} {trace:?}"))?; }
    let mut a = ZipArchive::new(w.finish().map_err(|e| format!("rfinish {e}"))?).map_err(|e| format!("raw-reopen: {e}"))?;
    let m4: Vec<Ent> = model.iter().filter(|m| m.enc != 3 && m.kind != 2).cloned().collect();
    check(&mut a, &m4).map_err(|e| format!("D(rawcopy) {e} {trace:?}"))?;
    Ok(())
}
fn main() {
    let n: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut errs: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for s in 1..=n {
        let r = std::panic::catch_unwind(|| one(s));
        let e = match r { Ok(Ok(())) => continue, Ok(Err(e)) => e, Err(p) => format!("PANIC {:?}", p.downcast_ref::<String>().map(|s| s.as_str()).or(p.downcast_ref::<&str>().copied())) };
        let key: String = e.split(|c: char| c.is_ascii_digit()).collect::<Vec<_>>().join("#").chars().take(30).collect();
        let en = errs.entry(key).or_insert((0, e)); en.0 += 1;
    }
    for (k, (c, e)) in errs { println!("[{c}] {k}\n   ex: {}", e.chars().take(900).collect::<String>()); }
    println!("done {n}");
}
