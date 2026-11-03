use heapless::HistoryBuf;
fn main(){ let h:HistoryBuf<std::string::String,4>=HistoryBuf::new(); let _=h.as_view(); }
