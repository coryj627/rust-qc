use heapless::*;
pub fn control_u8(s: &HistoryBuf<u8,4>) -> &heapless::history_buf::HistoryBufView<u8> { s.as_view() }
#[cfg(c1)] pub fn as_view_nc(s: &HistoryBuf<std::string::String,4>) { let _ = s.as_view(); }
fn ser<T: serde::Serialize + ?Sized>(_: &T) {}
pub fn s_vec(v: &Vec<u8,4>) { ser(v) }
pub fn s_deq(v: &deque::DequeView<u8>) { ser(v) }
#[cfg(c2)] pub fn s_vv(v: &vec::VecView<u8>) { ser(v) }
#[cfg(c3)] pub fn iset_eq() { fn e<T:Eq>(){} e::<index_set::FnvIndexSet<u8,4>>(); }
#[cfg(c4)] pub fn iset_into(s: index_set::FnvIndexSet<u8,4>) { for _x in s {} }
#[cfg(c5)] pub fn sll_u32() { let _l: sorted_linked_list::SortedLinkedList<u32, sorted_linked_list::Max, 4, u32> = sorted_linked_list::SortedLinkedList::new_u32(); }
pub fn sll_u32_ty(_l: &sorted_linked_list::SortedLinkedList<u32, sorted_linked_list::Max, 4, u32>) {}
