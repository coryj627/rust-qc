use heapless::index_map::{IndexMap, FnvIndexMap};
use std::hash::{BuildHasherDefault, Hasher};
#[derive(Default)] struct Id(u64);
impl Hasher for Id { fn finish(&self)->u64{self.0} fn write(&mut self,_:&[u8]){} fn write_u32(&mut self,v:u32){self.0=v as u64;} }
fn big<F:FnOnce()+Send+'static>(f:F){ std::thread::Builder::new().stack_size(1<<28).spawn(f).unwrap().join().unwrap(); }
#[test] fn n_131072() { big(|| {
  let mut m:std::boxed::Box<FnvIndexMap<u32,u32,131072>> = std::boxed::Box::new(FnvIndexMap::new());
  for k in 0..100_000u32 { assert_eq!(m.insert(k,k),Ok(None), "dup claim at {k}"); }
  let wrong=(0..100_000u32).filter(|k| m.get(k)!=Some(k)).count();
  eprintln!("len={} wrong lookups={}", m.len(), wrong);
  assert_eq!(wrong,0);
});}
#[test] fn n_65536_last() { big(|| {
  let mut m:std::boxed::Box<IndexMap<u32,u32,BuildHasherDefault<Id>,65536>> = std::boxed::Box::new(Default::default());
  for k in 0..0xffffu32 { m.insert(k,k).unwrap(); }
  m.insert(0xffff,7).unwrap();
  eprintln!("len={} get={:?}", m.len(), m.get(&0xffff));
  assert_eq!(m.get(&0xffff),Some(&7));
});}
