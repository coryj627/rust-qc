use byte_span::{ByteSpan, SpanError};

fn span(start: usize, end: usize) -> ByteSpan {
    ByteSpan::new(start, end).unwrap()
}

#[test]
fn construction_and_accessors() {
    let value = span(4, 9);
    assert_eq!(value.start(), 4);
    assert_eq!(value.end(), 9);
    assert_eq!(value.len(), 5);
    assert!(!value.is_empty());
    assert_eq!(
        ByteSpan::new(9, 4),
        Err(SpanError::Reversed { start: 9, end: 4 })
    );
}

#[test]
fn full_usize_range() {
    let full = span(0, usize::MAX);
    assert_eq!(full.len(), usize::MAX);
    assert!(full.contains(0));
    assert!(full.contains(usize::MAX - 1));
    assert!(!full.contains(usize::MAX));
    let last = span(usize::MAX - 1, usize::MAX);
    assert_eq!(last.len(), 1);
    assert_eq!(full.intersection(&last), Some(last));
    assert_eq!(
        ByteSpan::new(usize::MAX, 0),
        Err(SpanError::Reversed {
            start: usize::MAX,
            end: 0,
        })
    );
}

#[test]
fn empty_spans_contain_nothing() {
    for offset in [0, 5, usize::MAX] {
        let empty = span(offset, offset);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        for candidate in [0, 4, 5, 6, usize::MAX - 1, usize::MAX] {
            assert!(!empty.contains(candidate));
        }
        assert_eq!(empty.intersection(&span(0, usize::MAX)), None);
        assert_eq!(empty.intersection(&empty), None);
    }
}

#[test]
fn containment_is_half_open() {
    let value = span(3, 7);
    for offset in 0..10 {
        assert_eq!(value.contains(offset), (3..7).contains(&offset));
    }
}

#[test]
fn intersections_cover_overlap_nesting_touching_and_disjoint_spans() {
    let base = span(3, 8);
    for (other, expected) in [
        (span(1, 5), Some(span(3, 5))),
        (span(6, 10), Some(span(6, 8))),
        (span(4, 7), Some(span(4, 7))),
        (span(0, 10), Some(base)),
        (base, Some(base)),
        (span(1, 3), None),
        (span(8, 10), None),
        (span(0, 2), None),
        (span(9, 10), None),
        (span(5, 5), None),
    ] {
        assert_eq!(base.intersection(&other), expected);
        assert_eq!(other.intersection(&base), expected);
    }
}

#[test]
fn intersection_matches_shared_byte_membership_for_all_small_spans() {
    for a in 0..=6 {
        for b in a..=6 {
            for c in 0..=6 {
                for d in c..=6 {
                    let left = span(a, b);
                    let right = span(c, d);
                    let intersection = left.intersection(&right);
                    let shared: Vec<_> = (0..=6)
                        .filter(|&offset| left.contains(offset) && right.contains(offset))
                        .collect();
                    assert_eq!(intersection.is_none(), shared.is_empty());
                    if let Some(overlap) = intersection {
                        assert!(!overlap.is_empty());
                        assert_eq!(overlap.len(), shared.len());
                        for offset in 0..=6 {
                            assert_eq!(overlap.contains(offset), shared.contains(&offset));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn slices_ascii_and_multibyte_text_without_copying() {
    let text = String::from("aé中🦀z");
    for (start, end, expected) in [
        (0, 1, "a"),
        (1, 3, "é"),
        (3, 6, "中"),
        (6, 10, "🦀"),
        (1, 10, "é中🦀"),
        (0, 11, "aé中🦀z"),
        (10, 11, "z"),
    ] {
        let result = span(start, end).slice(&text).unwrap();
        assert_eq!(result, expected);
        assert_eq!(result.as_ptr(), text[start..].as_ptr());
    }
}

#[test]
fn empty_slices_require_valid_boundaries() {
    assert_eq!(span(0, 0).slice(""), Ok(""));
    for offset in [0, 1, 3, 6, 10, 11] {
        assert_eq!(span(offset, offset).slice("aé中🦀z"), Ok(""));
    }
    assert_eq!(
        span(2, 2).slice("aé中🦀z"),
        Err(SpanError::InvalidCharBoundary { offset: 2 })
    );
}

#[test]
fn invalid_start_and_end_boundaries_are_reported() {
    for (start, end, offset) in [(2, 3, 2), (1, 2, 2), (2, 4, 2), (3, 4, 4)] {
        assert_eq!(
            span(start, end).slice("aé中🦀z"),
            Err(SpanError::InvalidCharBoundary { offset })
        );
    }
}

#[test]
fn bounds_errors_precede_boundary_errors() {
    for (start, end) in [(0, 3), (1, 3), (3, 3), (usize::MAX, usize::MAX)] {
        assert_eq!(
            span(start, end).slice("é"),
            Err(SpanError::OutOfBounds {
                start,
                end,
                text_len: 2,
            })
        );
    }
    assert_eq!(
        span(0, usize::MAX).slice(""),
        Err(SpanError::OutOfBounds {
            start: 0,
            end: usize::MAX,
            text_len: 0,
        })
    );
}

#[test]
fn slice_success_matches_str_get_for_every_small_range() {
    let text = "aé中🦀z";
    for start in 0..=text.len() + 1 {
        for end in start..=text.len() + 1 {
            let result = span(start, end).slice(text);
            assert_eq!(result.as_ref().ok().copied(), text.get(start..end));
            if end > text.len() {
                assert!(matches!(result, Err(SpanError::OutOfBounds { .. })));
            } else if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                assert!(matches!(result, Err(SpanError::InvalidCharBoundary { .. })));
            }
        }
    }
}

#[test]
fn slice_lifetime_is_tied_to_text() {
    let text = String::from("hello");
    let borrowed = {
        let temporary_span = span(1, 4);
        temporary_span.slice(&text).unwrap()
    };
    assert_eq!(borrowed, "ell");
}

#[test]
fn errors_support_standard_error_and_formatting() {
    for error in [
        SpanError::Reversed { start: 2, end: 1 },
        SpanError::OutOfBounds {
            start: 0,
            end: 2,
            text_len: 1,
        },
        SpanError::InvalidCharBoundary { offset: 1 },
    ] {
        let standard_error: &dyn std::error::Error = &error;
        assert!(!standard_error.to_string().is_empty());
        assert!(standard_error.source().is_none());
        assert!(!format!("{error:?}").is_empty());
    }
    assert!(!format!("{:?}", span(0, 1)).is_empty());
}
