use byte_span::{ByteSpan, SpanError};

#[test]
fn construction_and_membership_cover_usize_extremes() {
    let positions = [0, 1, 2, usize::MAX / 2, usize::MAX - 1, usize::MAX];
    for start in positions {
        for end in positions {
            if start > end {
                assert_eq!(
                    ByteSpan::new(start, end),
                    Err(SpanError::Reversed { start, end })
                );
                continue;
            }
            let span = ByteSpan::new(start, end).unwrap();
            assert_eq!((span.start(), span.end()), (start, end));
            assert_eq!(span.start().checked_add(span.len()), Some(end));
            assert_eq!(span.is_empty(), start == end);
            assert!(!span.contains(end), "exclusive endpoint {end} in {span:?}");
            assert_eq!(span.contains(start), !span.is_empty());
            if start > 0 {
                assert!(!span.contains(start - 1), "offset before {span:?}");
            }
            if start < end {
                assert!(span.contains(end - 1), "last byte of {span:?}");
            }
        }
    }
    assert_eq!(ByteSpan::new(0, usize::MAX).unwrap().len(), usize::MAX);
}

#[test]
fn intersections_match_shared_bytes_for_all_small_spans() {
    let spans: Vec<_> = (0..=8)
        .flat_map(|start| (start..=8).map(move |end| ByteSpan::new(start, end).unwrap()))
        .collect();
    for left in &spans {
        for right in &spans {
            let shared: Vec<_> = (left.start()..left.end())
                .filter(|offset| (right.start()..right.end()).contains(offset))
                .collect();
            let intersection = left.intersection(right);
            assert_eq!(intersection, right.intersection(left));
            assert_eq!(intersection.is_none(), shared.is_empty());
            if let Some(overlap) = intersection {
                assert_eq!((overlap.start()..overlap.end()).collect::<Vec<_>>(), shared);
                assert!(!overlap.is_empty(), "overlap of {left:?} and {right:?}");
            }
        }
    }
}

#[test]
fn intersections_handle_maximum_endpoints() {
    let full = ByteSpan::new(0, usize::MAX).unwrap();
    let last = ByteSpan::new(usize::MAX - 1, usize::MAX).unwrap();
    let before = ByteSpan::new(0, usize::MAX - 1).unwrap();
    let empty = ByteSpan::new(usize::MAX, usize::MAX).unwrap();
    assert_eq!(full.intersection(&full), Some(full));
    assert_eq!(full.intersection(&last), Some(last));
    assert_eq!(last.intersection(&before), None);
    assert_eq!(full.intersection(&empty), None);
    assert_eq!(empty.intersection(&empty), None);
}

#[test]
fn slices_match_character_sequences_at_every_valid_boundary() {
    for text in ["", "abc", "é", "aé中🦀z", "e\u{301}", "\0\r\n"] {
        let boundaries: Vec<_> = text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(text.len()))
            .collect();
        for (start_index, &start) in boundaries.iter().enumerate() {
            for (end_index, &end) in boundaries.iter().enumerate().skip(start_index) {
                let expected: String = text
                    .chars()
                    .skip(start_index)
                    .take(end_index - start_index)
                    .collect();
                let slice = ByteSpan::new(start, end).unwrap().slice(text).unwrap();
                assert_eq!(slice, expected);
                assert_eq!(slice.as_ptr(), text.as_ptr().wrapping_add(start));
            }
        }
    }
}

#[test]
fn slicing_rejects_every_interior_utf8_byte_in_either_endpoint() {
    // The characters occupy one, two, three and four bytes respectively.
    let text = "aé中🦀";
    for offset in [2, 4, 5, 7, 8, 9] {
        let expected = Err(SpanError::InvalidUtf8Boundary { offset });
        assert_eq!(
            ByteSpan::new(offset, text.len()).unwrap().slice(text),
            expected
        );
        assert_eq!(ByteSpan::new(0, offset).unwrap().slice(text), expected);
        assert_eq!(ByteSpan::new(offset, offset).unwrap().slice(text), expected);
    }
    assert_eq!(
        ByteSpan::new(2, 4).unwrap().slice(text),
        Err(SpanError::InvalidUtf8Boundary { offset: 2 })
    );
}

#[test]
fn slicing_reports_bounds_before_utf8_and_handles_huge_offsets() {
    let text = "é";
    for (start, end) in [(1, 3), (0, usize::MAX), (usize::MAX, usize::MAX)] {
        assert_eq!(
            ByteSpan::new(start, end).unwrap().slice(text),
            Err(SpanError::OutOfBounds {
                end,
                text_len: text.len()
            })
        );
    }
    assert_eq!(ByteSpan::new(0, 0).unwrap().slice(""), Ok(""));
    assert_eq!(ByteSpan::new(2, 2).unwrap().slice(text), Ok(""));
    assert_eq!(
        ByteSpan::new(1, 1).unwrap().slice(""),
        Err(SpanError::OutOfBounds {
            end: 1,
            text_len: 0
        })
    );
}

#[test]
fn returned_slice_outlives_span() {
    let text = String::from("aéz");
    let slice = {
        let span = ByteSpan::new(1, 3).unwrap();
        span.slice(&text).unwrap()
    };
    assert_eq!(slice, "é");
}

#[test]
fn errors_implement_standard_error_and_display_context() {
    let cases = [
        (
            SpanError::Reversed { start: 5, end: 2 },
            "start byte offset 5 exceeds end byte offset 2",
        ),
        (
            SpanError::OutOfBounds {
                end: 8,
                text_len: 3,
            },
            "end byte offset 8 exceeds text length 3 bytes",
        ),
        (
            SpanError::InvalidUtf8Boundary { offset: 2 },
            "byte offset 2 is not a UTF-8 boundary",
        ),
    ];
    for (error, expected) in cases {
        let standard_error: &dyn std::error::Error = &error;
        assert_eq!(standard_error.to_string(), expected);
        assert!(
            standard_error.source().is_none(),
            "no underlying error: {error}"
        );
    }
}
