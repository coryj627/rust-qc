pub fn f(h: &heapless::HistoryBuf<std::string::String, 4>) -> &heapless::history_buf::HistoryBufView<std::string::String> { h.as_view() }
