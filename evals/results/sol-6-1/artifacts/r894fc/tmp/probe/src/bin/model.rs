use std::{hash::{Hasher, BuildHasherDefault},collections::HashMap};
#[derive(Default)] struct H;
impl Hasher for H { fn write(&mut self,_:&[u8]){} fn finish(&self)->u64{0} }
fn main() {
    let c=moka::sync::Cache::builder().build_with_hasher(BuildHasherDefault::<H>::default());
    let mut m=HashMap::new(); let mut seed=123456789u64;
    for i in 0..100000u32 {
        seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;
        let k=(seed as u32)%300;
        match (seed>>32)%7 {
            0|1=>{c.insert(k,i);m.insert(k,i);},
            2=>{assert_eq!(c.remove(&k),m.remove(&k));},
            3=>{assert_eq!(c.get(&k),m.get(&k).copied());},
            4=>{assert_eq!(c.get_with(k,||i),*m.entry(k).or_insert(i));},
            5=>{let v=c.entry(k).or_insert(i).into_value();assert_eq!(v,*m.entry(k).or_insert(i));},
            _=>{c.invalidate(&k);m.remove(&k);}
        }
        if i%50==0 { c.run_pending_tasks(); assert_eq!(c.entry_count(),m.len() as u64); }
    }
    c.run_pending_tasks();assert_eq!(c.entry_count(),m.len() as u64);
    for (k,v) in c.iter() {assert_eq!(m.get(&k),Some(&v));}
    println!("100000 collision-heavy model operations passed");
}
