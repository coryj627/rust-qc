use critical_section as _;
use std::cell::Cell;
use std::hash::{BuildHasherDefault, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe as A};
use heapless::{Deque, HistoryBuf, IndexMap, LinearMap, Vec as HV, String as HS, CString};
use heapless::index_map::FnvIndexMap;
use heapless::sorted_linked_list::{SortedLinkedList, Max};

#[derive(Default, Clone)]
struct Id(u64);
impl Hasher for Id {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, b: &[u8]) { for x in b { self.0 = (self.0 << 8) | *x as u64; } }
    fn write_u32(&mut self, v: u32) { self.0 = v as u64; }
    fn write_u64(&mut self, v: u64) { self.0 = v; }
}
type BH = BuildHasherDefault<Id>;

thread_local!(static ARM: Cell<bool> = Cell::new(false));
thread_local!(static DROPS: Cell<u32> = Cell::new(0));
struct P(#[allow(dead_code)] Box<u32>, bool);
impl Drop for P { fn drop(&mut self) {
    DROPS.with(|d| d.set(d.get()+1));
    if self.1 && ARM.with(|a| a.get()) { ARM.with(|a| a.set(false)); panic!("boom"); }
}}
fn p(v: u32, b: bool) -> P { P(Box::new(v), b) }
fn arm() { ARM.with(|a| a.set(true)); }
fn drops() -> u32 { DROPS.with(|d| d.get()) }

// Key with panicking drop for IndexMap
#[derive(Clone)]
struct K(String, bool);
impl PartialEq for K { fn eq(&self, o:&K)->bool{ self.0==o.0 } }
impl Eq for K {}
impl std::hash::Hash for K { fn hash<H:Hasher>(&self,h:&mut H){ self.0.hash(h) } }
impl std::borrow::Borrow<str> for K { fn borrow(&self)->&str{ &self.0 } }
impl Drop for K { fn drop(&mut self) { if self.1 && ARM.with(|a| a.get()) { ARM.with(|a| a.set(false)); panic!("boom"); } } }

fn main() {
    let a = std::env::args().nth(1).expect("probe name");
    match a.as_str() {
        "deque_full" => {
            let mut d: Deque<u32,4> = Deque::new();
            for i in 0..4 { d.push_back(i).unwrap(); }
            d.pop_front(); d.pop_front();
            d.push_back(4).unwrap(); d.push_back(5).unwrap();
            let s = d.make_contiguous().to_vec();
            println!("after make_contiguous {:?} len {}", s, d.len());
            println!("pop_front {:?} len {}", d.pop_front(), d.len());
            println!("push_back -> {:?}", d.push_back(99));
            println!("len {}", d.len());
        }
        "deque_full_pop" => {
            let mut d: Deque<u32,4> = Deque::new();
            for i in 0..4 { d.push_back(i).unwrap(); }
            d.pop_front(); d.pop_front();
            d.push_back(4).unwrap(); d.push_back(5).unwrap();
            d.make_contiguous();
            for _ in 0..9 { println!("pop {:?} len {}", d.pop_front(), d.len()); }
        }
        "deque_canary" => {
            #[repr(C)] struct S { d: Deque<u64,4>, canary: u64 }
            let mut s = S { d: Deque::new(), canary: 1 };
            for i in 0..4 { s.d.push_back(i).unwrap(); }
            s.d.pop_front(); s.d.pop_front();
            s.d.push_back(4).unwrap(); s.d.push_back(5).unwrap();
            s.d.make_contiguous();
            s.d.pop_front();
            s.d.push_back(0xDEADBEEF).unwrap();
            let c = unsafe { std::ptr::read_volatile(&s.canary) };
            println!("struct size {} canary {:#x}", std::mem::size_of::<S>(), c);
        }
        "deque_mid" => {
            let mut d: Deque<u32,12> = Deque::new();
            for i in 0..7 { d.push_back(i).unwrap(); }
            for _ in 0..7 { d.pop_front(); }
            for i in 1..=8 { d.push_back(i).unwrap(); }
            println!("before len {}", d.len());
            let s = d.make_contiguous().to_vec();
            println!("slice {:?} len {} iter {:?} pop_back {:?}", s, d.len(), d.iter().collect::<Vec<_>>(), d.clone().pop_back());
        }
        "deque_mid_box" => {
            let mut d: Deque<Box<u32>,12> = Deque::new();
            for i in 0..7 { d.push_back(Box::new(i)).unwrap(); }
            for _ in 0..7 { d.pop_front(); }
            for i in 1..=8 { d.push_back(Box::new(i)).unwrap(); }
            d.make_contiguous();
            println!("len {}", d.len());
            drop(d);
            println!("dropped");
        }
        "sll_u32" => {
            let mut l: SortedLinkedList<u32,Max,4,u8> = SortedLinkedList::new_u8();
            for v in [1,2,3] { l.push(v).unwrap(); }
            let mut f = l.find_mut(|x| *x==2).unwrap();
            *f = 20;
            let v = f.pop();
            println!("popped {} list {:?}", v, l.iter().collect::<Vec<_>>());
        }
        "sll_box" => {
            let mut l: SortedLinkedList<Box<u32>,Max,4,u8> = SortedLinkedList::new_u8();
            for v in [1,2,3] { l.push(Box::new(v)).unwrap(); }
            let mut f = l.find_mut(|x| **x==2).unwrap();
            **f = 20;
            let v = f.pop();
            println!("popped {} list {:?}", v, l.iter().collect::<Vec<_>>());
        }
        "sll_single" => {
            let mut l: SortedLinkedList<u32,Max,4,u8> = SortedLinkedList::new_u8();
            l.push(1).unwrap();
            let mut f = l.find_mut(|x| *x==1).unwrap();
            *f = 20;
            let v = f.pop();
            println!("popped {} list {:?}", v, l.iter().collect::<Vec<_>>());
        }
        "sll_nomut" => {
            let mut l: SortedLinkedList<u32,Max,4,u8> = SortedLinkedList::new_u8();
            for v in [1,2,3] { l.push(v).unwrap(); }
            let f = l.find_mut(|x| *x==2).unwrap();
            let v = f.pop();
            println!("control popped {} list {:?}", v, l.iter().collect::<Vec<_>>());
        }
        "peek" => {
            let mut q: heapless::spsc::Queue<u64,4> = heapless::spsc::Queue::new();
            let (mut pr, co) = q.split();
            std::thread::scope(|s| {
                s.spawn(move || { pr.enqueue(7).unwrap(); });
                s.spawn(move || { loop { if let Some(v) = co.peek() { println!("peek {}", *v); break; } std::thread::yield_now(); } });
            });
        }
        "ready_c" => {
            let mut q: heapless::spsc::Queue<u64,4> = heapless::spsc::Queue::new();
            let (mut pr, mut co) = q.split();
            std::thread::scope(|s| {
                s.spawn(move || { pr.enqueue(7).unwrap(); });
                s.spawn(move || { loop { if co.ready() { println!("got {}", unsafe{co.dequeue_unchecked()}); break; } std::thread::yield_now(); } });
            });
        }
        "ready_p" => {
            let mut q: heapless::spsc::Queue<u64,2> = heapless::spsc::Queue::new();
            let (mut pr, mut co) = q.split();
            std::thread::scope(|s| {
                s.spawn(move || { let mut n=0; while n<4 { if co.dequeue().is_some() { n+=1; } else { std::thread::yield_now(); } } });
                s.spawn(move || { let mut n=0; while n<4 { if pr.ready() { unsafe{pr.enqueue_unchecked(n)}; n+=1; } else { std::thread::yield_now(); } } });
            });
            println!("ready_p done");
        }
        "ctl_dequeue" => {
            let mut q: heapless::spsc::Queue<u64,4> = heapless::spsc::Queue::new();
            let (mut pr, mut co) = q.split();
            std::thread::scope(|s| {
                s.spawn(move || { pr.enqueue(7).unwrap(); });
                s.spawn(move || { loop { if let Some(v) = co.dequeue() { println!("control dequeue {}", v); break; } std::thread::yield_now(); } });
            });
        }
        "im_clear" => {
            let mut m: FnvIndexMap<K,u32,4> = FnvIndexMap::new();
            m.insert(K("a-long-key-to-force-heap-x".into(), true),1).ok();
            m.insert(K("b-long-key-to-force-heap-x".into(), false),2).ok();
            arm();
            let r = catch_unwind(A(|| m.clear()));
            println!("panicked {} len {}", r.is_err(), m.len());
            let r = m.insert(K("a-long-key-to-force-heap-x".into(), false),3);
            println!("insert ok {} len {}", r.is_ok(), m.len());
        }
        "im_retain" => {
            let mut m: FnvIndexMap<u32,u32,8> = FnvIndexMap::new();
            for i in 0..6 { m.insert(i,i).unwrap(); }
            let mut n=0;
            let r = catch_unwind(A(|| m.retain(|_,_| { n+=1; if n==3 { panic!("boom") } n==2 })));
            println!("panicked {} len {}", r.is_err(), m.len());
            for i in 0..6 { println!("get {} {:?}", i, m.get(&i)); }
        }
        "im_trunc" => {
            let mut m: FnvIndexMap<K,u32,8> = FnvIndexMap::new();
            for (i,b) in [(0,false),(1,true),(2,false),(3,false)] { m.insert(K(format!("key-number-{i}-long-enough"), b),i).ok(); }
            arm();
            let r = catch_unwind(A(|| m.truncate(1)));
            println!("panicked {} len {}", r.is_err(), m.len());
            for i in 0..4 { println!("get {} {:?}", i, m.get(format!("key-number-{i}-long-enough").as_str())); }
        }
        "into_iter_panic" => {
            let mut v: HV<P,4> = HV::new();
            for i in 0..3 { v.push(p(i, i==1)).ok(); }
            let mut it = v.into_iter();
            let first = it.next(); drop(first);
            println!("before");
            arm();
            let r = catch_unwind(A(|| drop(it)));
            println!("panicked {} drops {}", r.is_err(), drops());
        }
        "deque_clear" => {
            let mut d: Deque<P,4> = Deque::new();
            d.push_back(p(0,true)).ok(); d.push_back(p(1,false)).ok();
            arm();
            let r = catch_unwind(A(|| d.clear()));
            println!("panicked {} len {} drops {}", r.is_err(), d.len(), drops());
            drop(d);
            println!("final drops {} for 2 values", drops());
        }
        "hist_clear" => {
            let mut d: HistoryBuf<P,4> = HistoryBuf::new();
            d.write(p(0,true)); d.write(p(1,false));
            arm();
            let r = catch_unwind(A(|| d.clear()));
            println!("panicked {} len {} drops {}", r.is_err(), d.len(), drops());
            drop(d);
            println!("final drops {} for 2 values", drops());
        }
        "hist_write" => {
            let mut d: HistoryBuf<P,1> = HistoryBuf::new();
            d.write(p(0,true));
            arm();
            let r = catch_unwind(A(|| d.write(p(1,false))));
            println!("panicked {} drops {}", r.is_err(), drops());
            drop(d);
            println!("final drops {} for 2 values", drops());
        }
        "trunc" => {
            let mut m: FnvIndexMap<u32,u32,8> = FnvIndexMap::new();
            for k in [0,1,9] { m.insert(k,k+1).unwrap(); }
            m.truncate(1);
            println!("len {} iter {:?} get(0) {:?} contains {}", m.len(), m.iter().collect::<Vec<_>>(), m.get(&0), m.contains_key(&0));
            println!("insert {:?} iter {:?}", m.insert(0,9), m.iter().collect::<Vec<_>>());
        }
        "trunc_search" => {
            let mut bad=0; let mut total=0;
            for a in 0..40u32 { for b in 0..40u32 { for c in 0..40u32 {
                if a==b||b==c||a==c {continue}
                let mut m: FnvIndexMap<u32,u32,8> = FnvIndexMap::new();
                m.insert(a,0).unwrap(); m.insert(b,0).unwrap(); m.insert(c,0).unwrap();
                m.truncate(1); total+=1;
                if m.get(&a).is_none() { bad+=1; }
            }}}
            println!("{bad} of {total} triples lose the retained key");
        }
        "cap64k" => {
            std::thread::Builder::new().stack_size(64<<20).spawn(|| {
                let mut m: IndexMap<u32,u32,BH,65536> = IndexMap::default();
                for i in 0..65536u32 { m.insert(i,i).unwrap(); }
                println!("len {} get(65535) {:?} get(1) {:?}", m.len(), m.get(&65535), m.get(&1));
            }).unwrap().join().unwrap();
        }
        "cap128k" => {
            std::thread::Builder::new().stack_size(64<<20).spawn(|| {
                let mut m: IndexMap<u32,u32,BH,131072> = IndexMap::default();
                let mut lost=0;
                for i in 0..70000u32 { let r=m.insert(i,i); assert!(r.is_ok()); if m.get(&i)!=Some(&i) {lost+=1;} }
                println!("len {} lost-right-after-insert {}", m.len(), lost);
            }).unwrap().join().unwrap();
        }
        "lm_from_iter" => {
            let m: LinearMap<u32,char,4> = [(1,'a'),(1,'b')].into_iter().collect();
            print!("len {} get {:?} ", m.len(), m.get(&1));
            let mut m=m; m.remove(&1); println!("after remove get {:?} len {}", m.get(&1), m.len());
            let mut a: LinearMap<u32,char,4> = LinearMap::new(); a.insert(1,'a').unwrap();
            let b: LinearMap<u32,char,4> = [(1,'a'),(1,'b')].into_iter().collect();
            println!("a==b {} b==a {}", a==b, b==a);
        }
        "bytes" => {
            use bytes::BufMut;
            let mut v: HV<u8,8> = HV::new();
            println!("remaining_mut {}", v.remaining_mut());
            v.put_slice(&[1;8]);
            println!("filled {}", v.len());
        }
        "cstring" => {
            println!("{:?}", CString::<8>::from_bytes_with_nul(b"abc"));
            println!("{:?}", CString::<8>::from_bytes_with_nul(b""));
            println!("{:?}", CString::<8>::from_bytes_with_nul(b"abc\0"));
        }
        "im_into_iter" => {
            let mut m: FnvIndexMap<u32,u32,8> = FnvIndexMap::new();
            for i in 1..=3 { m.insert(i,i).unwrap(); }
            println!("iter {:?} into_iter {:?}", m.iter().map(|x|*x.0).collect::<Vec<_>>(), m.into_iter().map(|x|x.0).collect::<Vec<_>>());
        }
        "lm_eq" => {
            fn is_eq<T:Eq>(_:&T){}
            let mut m: LinearMap<u8,f32,4> = LinearMap::new(); m.insert(1,f32::NAN).unwrap();
            is_eq(&m);
            println!("m==m {}", m==m);
        }
        "new_with" => {
            let h: HistoryBuf<u8,0> = HistoryBuf::new_with(1);
            println!("len {}", h.len());
            println!("recent {:?}", h.recent());
        }
        "retain_sb" => {
            let mut v: HV<u32,6> = HV::new();
            for i in 0..6 { v.push(i).unwrap(); }
            let mut n=0;
            let r = catch_unwind(A(|| v.retain(|_| { n+=1; if n==3 {panic!("boom")} n!=1 })));
            println!("panicked {} {:?}", r.is_err(), v);
        }
        "str_insert" => {
            let mut s: HS<8> = HS::new();
            s.insert(0,'a').unwrap();
            s.insert(1,'b').unwrap();
            println!("{}", s);
        }
        "sizes" => {
            let mut v: HV<u8,4> = HV::new(); v.push(1).unwrap();
            println!("vec into_iter {:?}", v.into_iter().size_hint());
            let mut d: Deque<u8,4> = Deque::new(); d.push_back(1).unwrap();
            println!("deque into_iter {:?}", d.into_iter().size_hint());
            let mut m: FnvIndexMap<u8,u8,4> = FnvIndexMap::new(); m.insert(1,1).unwrap();
            println!("im iter {:?}", m.iter().size_hint());
        }
        "pool" => {
            use heapless::{box_pool, pool::boxed::{BoxBlock, BoxPool}};
            box_pool!(PP: u64);
            for _ in 0..4 {
                let b: &'static mut BoxBlock<u64> = Box::leak(Box::new(BoxBlock::new()));
                PP.manage(b);
            }
            let hs: Vec<_> = (0..3).map(|t| std::thread::spawn(move || {
                for i in 0..50u64 { if let Ok(b) = PP.alloc(i+t) { assert_eq!(*b, i+t); drop(b); } std::thread::yield_now(); }
            })).collect();
            for h in hs { h.join().unwrap(); }
            println!("pool done");
        }
        "view" => {
            let s: HistoryBuf<u8,4> = HistoryBuf::new();
            let _ = s.as_view();
        }
        _ => panic!("unknown probe"),
    }
}
