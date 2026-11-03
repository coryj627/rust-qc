use byte_span::{ByteSpan, SpanError};

fn span(start: usize, end: usize) -> ByteSpan {
    ByteSpan::new(start, end).unwrap()
}

#[test]
fn construction_and_accessors_cover_extreme_offsets() {
    for (start, end) in [
        (0, 0),
        (0, 1),
        (2, 7),
        (0, usize::MAX),
        (usize::MAX - 1, usize::MAX),
        (usize::MAX, usize::MAX),
    ] {
        let value = span(start, end);
        assert_eq!(value.start(), start);
        assert_eq!(value.end(), end);
        assert_eq!(value.len(), end - start);
        assert_eq!(value.is_empty(), start == end);
    }
    for (start, end) in [
        (1, 0),
        (7, 2),
        (usize::MAX, 0),
        (usize::MAX, usize::MAX - 1),
    ] {
        assert_eq!(
            ByteSpan::new(start, end),
            Err(SpanError::Reversed { start, end })
        );
    }
}

#[test]
fn containment_is_half_open_even_at_usize_limits() {
    let value = span(2, 5);
    for offset in 0..8 {
        assert_eq!(value.contains(offset), (2..5).contains(&offset));
    }
    let full = span(0, usize::MAX);
    assert!(full.contains(0));
    assert!(full.contains(usize::MAX - 1));
    assert!(!full.contains(usize::MAX));
    let last = span(usize::MAX - 1, usize::MAX);
    assert!(!last.contains(usize::MAX - 2));
    assert!(last.contains(usize::MAX - 1));
    assert!(!last.contains(usize::MAX));
    for offset in [0, 3, usize::MAX] {
        let empty = span(offset, offset);
        assert!(!empty.contains(offset));
        assert!(!empty.contains(0));
        assert!(!empty.contains(usize::MAX));
    }
}

#[test]
fn intersections_cover_overlap_containment_touching_and_empty_spans() {
    // Enumerate all short spans and compare their intersection to the actual
    // set of shared byte offsets, including empty spans inside another span.
    for a_start in 0..=6 {
        for a_end in a_start..=6 {
            for b_start in 0..=6 {
                for b_end in b_start..=6 {
                    let a = span(a_start, a_end);
                    let b = span(b_start, b_end);
                    let shared: Vec<_> = (a_start..a_end)
                        .filter(|offset| (b_start..b_end).contains(offset))
                        .collect();
                    let expected = shared
                        .first()
                        .map(|first| span(*first, shared.last().unwrap() + 1));
                    assert_eq!(a.intersection(&b), expected, "{a:?}, {b:?}");
                    assert_eq!(b.intersection(&a), expected, "{b:?}, {a:?}");
                }
            }
        }
    }
}

#[test]
fn intersections_handle_usize_limits_without_arithmetic_overflow() {
    let full = span(0, usize::MAX);
    let last = span(usize::MAX - 1, usize::MAX);
    assert_eq!(full.intersection(&last), Some(last));
    assert_eq!(last.intersection(&last), Some(last));
    assert_eq!(full.intersection(&full), Some(full));
    assert_eq!(span(0, usize::MAX - 1).intersection(&last), None);
    assert_eq!(full.intersection(&span(usize::MAX, usize::MAX)), None);
    assert_eq!(span(0, 0).intersection(&full), None);
}

#[test]
fn slices_borrow_the_input_and_use_byte_offsets() {
    let text = String::from("aé中🦀z");
    // The boundaries are 0, 1, 3, 6, 10, 11 bytes.
    for (start, end, expected) in [
        (0, 11, "aé中🦀z"),
        (0, 1, "a"),
        (1, 3, "é"),
        (3, 6, "中"),
        (6, 10, "🦀"),
        (1, 10, "é中🦀"),
        (10, 11, "z"),
    ] {
        let slice = span(start, end).slice(&text).unwrap();
        assert_eq!(slice, expected);
        assert_eq!(slice.as_ptr(), text[start..].as_ptr());
    }
    // The result's lifetime is tied to the text, not to the span.
    let borrowed = {
        let temporary_span = span(1, 3);
        temporary_span.slice(&text).unwrap()
    };
    assert_eq!(borrowed, "é");
}

#[test]
fn empty_slices_require_a_boundary_and_allow_the_end_of_input() {
    let text = "é🦀";
    for offset in [0, 2, 6] {
        assert_eq!(span(offset, offset).slice(text), Ok(""));
    }
    for offset in [1, 3, 4, 5] {
        assert_eq!(
            span(offset, offset).slice(text),
            Err(SpanError::InvalidCharBoundary { offset })
        );
    }
    assert_eq!(span(0, 0).slice(""), Ok(""));
    assert_eq!(
        span(1, 1).slice(""),
        Err(SpanError::OutOfBounds { end: 1, len: 0 })
    );
}

#[test]
fn slicing_reports_invalid_start_then_invalid_end() {
    let text = "é🦀";
    for offset in [1, 3, 4, 5] {
        assert_eq!(
            span(offset, 6).slice(text),
            Err(SpanError::InvalidCharBoundary { offset })
        );
        assert_eq!(
            span(0, offset).slice(text),
            Err(SpanError::InvalidCharBoundary { offset })
        );
    }
    assert_eq!(
        span(1, 5).slice(text),
        Err(SpanError::InvalidCharBoundary { offset: 1 })
    );
}

#[test]
fn slicing_checks_bounds_before_utf8_boundaries() {
    for (start, end) in [
        (0, 7),
        (1, 7), // Invalid UTF-8 start, but bounds take precedence.
        (7, 8),
        (0, usize::MAX),
        (1, usize::MAX),
        (usize::MAX, usize::MAX),
    ] {
        assert_eq!(
            span(start, end).slice("é🦀"),
            Err(SpanError::OutOfBounds { end, len: 6 })
        );
    }
}

#[test]
fn all_short_utf8_ranges_agree_with_str_get() {
    for text in ["", "ascii", "aé中🦀z", "e\u{301}\0\r\n"] {
        for start in 0..=text.len() + 1 {
            for end in start..=text.len() + 1 {
                let result = span(start, end).slice(text);
                match text.get(start..end) {
                    Some(expected) => assert_eq!(result, Ok(expected)),
                    None => assert!(result.is_err(), "{text:?}, {start}..{end}"),
                }
            }
        }
    }
}

#[test]
fn errors_support_standard_error_reporting() {
    let errors = [
        SpanError::Reversed { start: 5, end: 2 },
        SpanError::OutOfBounds { end: 9, len: 4 },
        SpanError::InvalidCharBoundary { offset: 1 },
    ];
    for error in errors {
        let standard_error: &dyn std::error::Error = &error;
        assert!(!standard_error.to_string().is_empty());
        assert!(!format!("{error:?}").is_empty());
        assert!(standard_error.source().is_none());
    }
    assert!(!format!("{:?}", span(0, 1)).is_empty());
}
