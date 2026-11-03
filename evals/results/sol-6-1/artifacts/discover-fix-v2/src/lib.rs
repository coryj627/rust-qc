//! Integer statistics.

/// Returns the arithmetic mean, rounded down, or None for empty input.
pub fn mean(values: &[u32]) -> Option<u32> {
    if values.is_empty() {
        None
    } else {
        // u128 can hold the sum of u32 values for any addressable slice.
        let sum: u128 = values.iter().map(|&value| u128::from(value)).sum();
        Some((sum / values.len() as u128) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_handles_sums_larger_than_u32() {
        assert_eq!(mean(&[u32::MAX, u32::MAX]), Some(u32::MAX));
        assert_eq!(mean(&[u32::MAX; 3]), Some(u32::MAX));
        assert_eq!(mean(&[u32::MAX, u32::MAX - 1]), Some(u32::MAX - 1));
        assert_eq!(mean(&[u32::MAX, u32::MAX, 0]), Some(2_863_311_530));
        assert_eq!(mean(&[0, u32::MAX, u32::MAX]), Some(2_863_311_530));
        assert_eq!(mean(&[u32::MAX, 0, u32::MAX]), Some(2_863_311_530));
    }

    #[test]
    fn mean_returns_none_for_empty_input() {
        assert_eq!(mean(&[]), None);
    }

    #[test]
    fn mean_preserves_single_values() {
        assert_eq!(mean(&[0]), Some(0));
        assert_eq!(mean(&[u32::MAX]), Some(u32::MAX));
    }

    #[test]
    fn mean_rounds_down() {
        assert_eq!(mean(&[1, 2]), Some(1));
        assert_eq!(mean(&[0, 0, 2]), Some(0));
        assert_eq!(mean(&[u32::MAX, 0]), Some(2_147_483_647));
    }
}
