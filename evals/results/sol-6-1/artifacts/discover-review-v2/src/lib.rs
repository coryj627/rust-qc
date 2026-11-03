//! Page calculations.

/// Returns the number of pages needed to store items, or None for a zero page size.
pub fn pages(items: usize, page_size: usize) -> Option<usize> {
    if page_size == 0 {
        None
    } else {
        Some((items + page_size - 1) / page_size)
    }
}
