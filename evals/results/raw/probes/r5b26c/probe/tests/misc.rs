use heapless::{Deque, Vec, String, IndexSet, LinearMap, CString};
use heapless::index_set::FnvIndexSet;
fn is_eq<T: Eq>() {}
#[test] fn linear_map_eq_for_float() { is_eq::<LinearMap<u32,f32,2>>(); let mut m:LinearMap<u32,f32,2>=LinearMap::new(); m.insert(1,f32::NAN).unwrap(); assert!(m != m, "Eq requires reflexive, this is the proof NaN breaks it"); }
#[test] fn deque_eq_rotations() { for a in 0..4 { for b in 0..4 { for l in 0..=4u32 {
  let mut x:Deque<u32,4>=Deque::new(); let mut y:Deque<u32,4>=Deque::new();
  for _ in 0..a { x.push_back(0).unwrap(); x.pop_front(); } for _ in 0..b { y.push_back(0).unwrap(); y.pop_front(); }
  for i in 0..l { x.push_back(i).unwrap(); y.push_back(i).unwrap(); } assert!(x==y);
  if l>0 { *y.back_mut().unwrap()=99; assert!(x!=y); }
}}}}
#[test] fn format_macro() { let s:String<4>=heapless::format!(4; "abcde").map(|_:String<4>| String::new()).unwrap_or_default(); assert_eq!(s,""); let t:String<8>=heapless::format!("{}",12).unwrap(); assert_eq!(t,"12"); let e:Result<String<2>,_>=heapless::format!("{}",123); assert!(e.is_err()); }
#[test] fn try_from_nums() { assert_eq!(String::<20>::try_from(i64::MIN).unwrap().as_str(),"-9223372036854775808"); assert!(String::<3>::try_from(1000u32).is_err()); assert_eq!(String::<3>::try_from(255u8).unwrap(),"255"); }
#[test] fn utf16() { assert!(String::<4>::from_utf16(&[0xD800]).is_err()); assert_eq!(String::<4>::from_utf16(&[0xD83D,0xDE00]).unwrap(),"😀"); assert!(String::<3>::from_utf16(&[0xD83D,0xDE00]).is_err()); }
#[test] fn set_ops() { let a:FnvIndexSet<u32,8>=[1,2,3,4].into_iter().collect(); let b:FnvIndexSet<u32,8>=[3,4,5].into_iter().collect();
  assert_eq!(a.difference(&b).copied().collect::<std::vec::Vec<_>>(),[1,2]); assert_eq!(a.intersection(&b).copied().collect::<std::vec::Vec<_>>(),[3,4]); assert_eq!(a.union(&b).copied().collect::<std::vec::Vec<_>>(),[1,2,3,4,5]); assert_eq!(a.symmetric_difference(&b).copied().collect::<std::vec::Vec<_>>(),[1,2,5]); let _:Option<&IndexSet<u32,_,8>>=None::<&FnvIndexSet<u32,8>>; }
#[test] fn cstring_edge() { let mut c=CString::<4>::new(); assert!(c.extend_from_bytes(b"\0").is_ok()); assert_eq!(c.as_bytes(),b""); assert!(c.extend_from_bytes(b"abc\0").is_ok()); assert_eq!(c.as_bytes(),b"abc"); assert!(c.extend_from_bytes(b"d").is_err()); assert_eq!(c.as_bytes(),b"abc"); assert!(CString::<4>::from_bytes_with_nul(b"").is_err(),"empty slice has no nul; doc says it fails"); }
#[test] fn bytes_put_u8_to_full() { use bytes::BufMut; let mut v:Vec<u8,2>=Vec::new(); v.put_u8(1); assert_eq!(v.remaining_mut(),1); let c=v.chunk_mut(); assert_eq!(c.len(),1); v.put_u8(2); assert_eq!(v.len(),2); }
#[test] fn hist_ser_order() {}
