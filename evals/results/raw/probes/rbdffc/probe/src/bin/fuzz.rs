use probe::*;
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use zip::ZipArchive;
struct A;
static IN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
unsafe impl std::alloc::GlobalAlloc for A {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 { hook(&l); std::alloc::System.alloc(l) }
    unsafe fn alloc_zeroed(&self, l: std::alloc::Layout) -> *mut u8 { hook(&l); std::alloc::System.alloc_zeroed(l) }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) { std::alloc::System.dealloc(p, l) }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 { if n > 1 << 46 { hook(&std::alloc::Layout::from_size_align_unchecked(n, l.align())); } std::alloc::System.realloc(p, l, n) }
}
fn frames() -> String { let b = std::backtrace::Backtrace::force_capture().to_string(); let mut out = vec![]; let mut prev = String::new(); for l in b.lines() { if l.trim_start().starts_with("at ") { if l.contains("/work/crate") || l.contains("cargo-home") { out.push(format!("{} <- {}", l.trim(), prev.trim())); } } else { prev = l.to_string(); } } out.into_iter().take(8).collect::<Vec<_>>().join("\n     ") }
fn hook(l: &std::alloc::Layout) { if l.size() > 1 << 46 && !IN.swap(true, std::sync::atomic::Ordering::SeqCst) { println!("HUGE ALLOC {}\n     {}", l.size(), frames()); IN.store(false, std::sync::atomic::Ordering::SeqCst); } }
#[global_allocator] static GA: A = A;
static SEEN: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
fn drain<R: Read>(mut r: R) { let mut b = vec![]; let _ = (&mut r).take(1 << 20).read_to_end(&mut b); }
fn exercise(v: &[u8]) {
    if let Ok(mut a) = ZipArchive::new(Cursor::new(v)) {
        let n = a.len().min(64);
        let _ = a.has_overlapping_files();
        let _ = a.root_dir(zip::read::root_dir_common_filter);
        let _ = a.decompressed_size();
        for i in 0..n {
            if a.by_index_raw(i).map(|f| f.compression() == zip::CompressionMethod::Xz).unwrap_or(false) { continue; }
            if let Ok(f) = a.by_index_raw(i) {
                let _ = (f.name(), f.mangled_name(), f.enclosed_name(), f.unix_mode(), f.is_symlink(), f.is_dir(), f.last_modified(), f.options(), f.version_made_by());
                let _ = f.extra_data_fields().count();
                let _ = f.data_start();
                drain(f);
            }
            if let Ok(f) = a.by_index(i) { drain(f); }
            let _ = a.get_aes_verification_key_and_salt(i);
            if let Ok(f) = a.by_index_decrypt(i, b"pw") { drain(f); }
            if let Ok(mut f) = a.by_index_seek(i) { use std::io::Seek; let _ = f.seek(std::io::SeekFrom::End(-1)); let _ = f.seek(std::io::SeekFrom::Current(-3)); drain(f); }
        }
    }
    let mut c = Cursor::new(v);
    for _ in 0..64 { match zip::read::read_zipfile_from_stream(&mut c) { Ok(Some(f)) => { if f.compression() == zip::CompressionMethod::Xz { break; } drain(f) } _ => break } }
}
fn tree(r: &mut Rng, n: usize) -> Vec<u8> {
    // random Kraft-full code lengths for n symbols; returns RLE bytes
    let mut leaves = vec![1u8, 1u8];
    while leaves.len() < n { let i = r.below(leaves.len() as u64) as usize; if leaves[i] < 16 { leaves[i] += 1; let d = leaves[i]; leaves.push(d); } }
    leaves.sort();
    let mut bytes = vec![]; let mut i = 0;
    while i < leaves.len() { let l = leaves[i]; let mut run = 0; while i < leaves.len() && leaves[i] == l && run < 16 { run += 1; i += 1; } bytes.push((l - 1) | ((run - 1) << 4)); }
    let mut o = vec![(bytes.len() - 1) as u8]; o.extend(bytes); o
}
fn legacy_case(r: &mut Rng) -> Vec<u8> {
    let m = 1 + r.below(6) as u16;
    let unc = r.below(60000) as u32;
    let mut d: Vec<u8>;
    let mut flags = 0u16;
    if m == 1 {
        let mut b = Bits::new(); let mut sz = 9;
        let n = r.below(9000);
        for _ in 0..n { match r.below(40) {
            0 => { b.put(256, sz); b.put(2, sz); }
            1 if sz < 13 => { b.put(256, sz); b.put(1, sz); sz += 1; }
            2..=19 => { b.put(r.below(256) as u32, sz); }
            _ => { b.put((257 + r.below(1 << 8)) as u32 & ((1 << sz) - 1), sz); }
        } }
        d = b.finish();
    } else if m == 6 {
        flags = (r.below(4) as u16) << 1;
        d = vec![];
        if flags & 4 != 0 { let k = tree(r, 256); d.extend(k); }
        let k = tree(r, 64); d.extend(k); let k = tree(r, 64); d.extend(k);
        for _ in 0..r.below(8000) { d.push(r.next() as u8); }
    } else {
        let mut b = Bits::new();
        for _ in 0..256 { let n = r.below(33) as u32; b.put(n, 6); for _ in 0..n { b.put(r.below(256) as u32, 8); } }
        for _ in 0..r.below(8000) { b.put(r.below(256) as u32, 8); }
        d = b.finish();
    }
    if r.below(3) == 0 { let cut = r.below(d.len() as u64 + 1) as usize; d.truncate(cut); }
    let mut x = e("x", &d); x.method = m; x.flags = flags; x.usize_ = unc; build(&[x])
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a[1].as_str(); let secs: u64 = a[2].parse().unwrap(); let seed: u64 = a[3].parse().unwrap();
    std::panic::set_hook(Box::new(|i| {
        let msg = i.payload().downcast_ref::<String>().map(|s| s.as_str()).or(i.payload().downcast_ref::<&str>().copied()).unwrap_or("?").to_string();
        let k = format!("{} :: {}", i.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default(), msg.chars().take(100).collect::<String>());
        if SEEN.lock().unwrap().insert(k.clone()) { println!("NEW PANIC {k}\n     {}", frames()); }
    }));
    let mut r = Rng(seed | 1);
    let mut seeds = vec![];
    for dir in ["/work/crate/tests/data", "/work/crate/tests/data/legacy"] { if let Ok(rd) = std::fs::read_dir(dir) { for f in rd.flatten() { if let Ok(m) = f.metadata() { if m.is_file() && m.len() < 300_000 { seeds.push(std::fs::read(f.path()).unwrap()); } } } } }
    let t = Instant::now(); let mut iters = 0u64;
    while t.elapsed() < Duration::from_secs(secs) {
        iters += 1;
        let v = if mode == "legacy" { legacy_case(&mut r) } else {
            let mut v = seeds[r.below(seeds.len() as u64) as usize].clone();
            if v.is_empty() { continue; }
            // bias mutations to the tail (metadata)
            for _ in 0..1 + r.below(6) {
                let n = v.len() as u64;
                let pos = if r.below(4) > 0 { n - 1 - r.below(n.min(600)) } else { r.below(n) } as usize;
                match r.below(4) { 0 => v[pos] = r.next() as u8, 1 => v[pos] = 0xff, 2 => v[pos] = 0, _ => { for j in pos..(pos + 8).min(v.len()) { v[j] = 0xff; } } }
            }
            v
        };
        let _ = std::panic::catch_unwind(|| exercise(&v));
    }
    println!("iters={iters} unique_panics={}", SEEN.lock().unwrap().len());
}
