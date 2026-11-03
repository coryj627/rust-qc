use std::alloc::{GlobalAlloc,Layout,System};
// A legal allocator: returns blocks aligned exactly as requested, here 4 bytes off a 16-byte boundary for align<=4.
struct A; unsafe impl GlobalAlloc for A{ unsafe fn alloc(&self,l:Layout)->*mut u8{ if l.align()<=4 { let p=System.alloc(Layout::from_size_align_unchecked(l.size()+16,16)); p.add(4) } else { System.alloc(l) } } unsafe fn dealloc(&self,p:*mut u8,l:Layout){ if l.align()<=4 { System.dealloc(p.sub(4),Layout::from_size_align_unchecked(l.size()+16,16)) } else { System.dealloc(p,l) } } }
#[global_allocator] static G:A=A;
fn main(){ let mut b=b"[1,2,3,4]".to_vec(); println!("{:?}",simd_json::to_owned_value(&mut b).map(|_|"parsed").map_err(|e|e.to_string())); }
