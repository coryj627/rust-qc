use byte_span::{ByteSpan, SpanError};

fn coords(span: &ByteSpan) -> (usize, usize) {
    (span.start(), span.end())
}

#[test]
fn construction_accessors_and_rejection() {
    for start in 0..=9 {
        for end in 0..=9 {
            let result = ByteSpan::new(start, end);
            if start > end {
                assert!(result.is_err(), "accepted reversed [{start}, {end})");
            } else {
                let span = result.unwrap();
                assert_eq!(coords(&span), (start, end));
                assert_eq!(span.len(), end - start);
                assert_eq!(span.is_empty(), start == end);
            }
        }
    }
}

#[test]
fn contains_half_open_and_empty() {
    for start in 0..=8 {
        for end in start..=8 {
            let span = ByteSpan::new(start, end).unwrap();
            for offset in 0..=9 {
                assert_eq!(
                    span.contains(offset),
                    (start..end).contains(&offset),
                    "[{start}, {end}) at {offset}"
                );
            }
        }
    }
}

#[test]
fn intersection_positive_overlap_only() {
    // Model overlap using set membership over a finite universe. No assumption
    // about storage, Eq/Clone/Copy implementations, or intersection algorithm.
    for a in 0..=7 {
        for b in a..=7 {
            for c in 0..=7 {
                for d in c..=7 {
                    let left = ByteSpan::new(a, b).unwrap();
                    let right = ByteSpan::new(c, d).unwrap();
                    let common: Vec<usize> = (0..7)
                        .filter(|p| (a..b).contains(p) && (c..d).contains(p))
                        .collect();
                    let expected = common
                        .first()
                        .map(|first| (*first, common.last().unwrap() + 1));
                    let actual = left.intersection(&right);
                    assert_eq!(
                        actual.as_ref().map(coords),
                        expected,
                        "intersection of [{a}, {b}) and [{c}, {d})"
                    );
                    assert_eq!(right.intersection(&left).as_ref().map(coords), expected);
                    assert_eq!(coords(&left), (a, b));
                    assert_eq!(coords(&right), (c, d));
                }
            }
        }
    }
}

#[test]
fn full_usize_range_without_overflow() {
    let max = usize::MAX;
    for (start, end) in [(0, max), (max, max), (max - 1, max), (1, max), (0, 0)] {
        let span = ByteSpan::new(start, end).unwrap();
        assert_eq!(coords(&span), (start, end));
        assert_eq!(span.len(), end - start);
        assert_eq!(span.is_empty(), start == end);
        for offset in [0, 1, max - 1, max] {
            assert_eq!(span.contains(offset), start <= offset && offset < end);
        }
        assert_eq!(span.slice("").is_ok(), start == 0 && end == 0);
    }
    assert!(ByteSpan::new(max, 0).is_err());
    assert!(ByteSpan::new(max, max - 1).is_err());
    let all = ByteSpan::new(0, max).unwrap();
    let last = ByteSpan::new(max - 1, max).unwrap();
    assert_eq!(
        all.intersection(&last).as_ref().map(coords),
        Some((max - 1, max))
    );
    assert_eq!(
        last.intersection(&all).as_ref().map(coords),
        Some((max - 1, max))
    );
    assert_eq!(all.intersection(&all).as_ref().map(coords), Some((0, max)));
    let empty = ByteSpan::new(max, max).unwrap();
    assert!(empty.intersection(&all).is_none());
    assert!(all.intersection(&empty).is_none());
    assert!(empty.intersection(&empty).is_none());
}

fn check_slices(text: &str) {
    let mut boundaries: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();
    boundaries.push(text.len());
    for start in 0..=text.len() + 2 {
        for end in start..=text.len() + 2 {
            let span = ByteSpan::new(start, end).unwrap();
            let result = span.slice(text);
            let valid =
                end <= text.len() && boundaries.contains(&start) && boundaries.contains(&end);
            assert_eq!(result.is_ok(), valid, "slice [{start}, {end}) of {text:?}");
            if let Ok(actual) = result {
                // The standard library is an independent substring oracle.
                assert_eq!(actual, &text[start..end]);
                if start < end {
                    assert_eq!(
                        actual.as_ptr(),
                        text.as_ptr().wrapping_add(start),
                        "nonempty slice must borrow the indicated source bytes"
                    );
                }
            }
        }
    }
}

#[test]
fn slicing_ascii_empty_and_past_end() {
    for text in ["", "a", "abcdef", "\0a\nb"] {
        check_slices(text);
    }
    let text = "abc";
    assert_eq!(
        ByteSpan::new(text.len(), text.len())
            .unwrap()
            .slice(text)
            .unwrap(),
        ""
    );
}

#[test]
fn slicing_utf8_all_endpoints() {
    // ASCII, 2-, 3-, and 4-byte code points, combining mark, and NUL.
    for text in ["aé中🦀z", "é", "中", "🦀", "e\u{301}\0é"] {
        check_slices(text);
    }
    assert_eq!(
        ByteSpan::new(1, 10).unwrap().slice("aé中🦀z").unwrap(),
        "é中🦀"
    );
}

// The returned borrow must be tied to text, independently of the span's lifetime.
fn borrow_from_short_lived_span(text: &str) -> &str {
    ByteSpan::new(1, text.len()).unwrap().slice(text).unwrap()
}

#[test]
fn slice_borrows_input_independently_of_span() {
    let owned = String::from("xé中🦀");
    let borrowed = borrow_from_short_lived_span(&owned);
    assert_eq!(borrowed, "é中🦀");
    assert_eq!(borrowed.as_ptr(), owned.as_ptr().wrapping_add(1));
}

#[test]
fn required_traits_and_error_operations() {
    fn debug<T: std::fmt::Debug>() {}
    fn error<T: std::error::Error + std::fmt::Display + std::fmt::Debug>() {}
    debug::<ByteSpan>();
    error::<SpanError>();
    let _: fn(usize, usize) -> Result<ByteSpan, SpanError> = ByteSpan::new;
    let _: fn(&ByteSpan) -> usize = ByteSpan::start;
    let _: fn(&ByteSpan) -> usize = ByteSpan::end;
    let _: fn(&ByteSpan) -> usize = ByteSpan::len;
    let _: fn(&ByteSpan) -> bool = ByteSpan::is_empty;
    let _: fn(&ByteSpan, usize) -> bool = ByteSpan::contains;
    let _: fn(&ByteSpan, &ByteSpan) -> Option<ByteSpan> = ByteSpan::intersection;
    let _: for<'a, 'b> fn(&'b ByteSpan, &'a str) -> Result<&'a str, SpanError> = ByteSpan::slice;
    let reversed = ByteSpan::new(2, 1).unwrap_err();
    let bounds = ByteSpan::new(0, 4).unwrap().slice("abc").unwrap_err();
    let boundary = ByteSpan::new(1, 2).unwrap().slice("é").unwrap_err();
    // Exercise the required traits without imposing any display wording,
    // source-chain convention, enum variants, equality, or layout.
    for err in [&reversed, &bounds, &boundary] {
        let _ = format!("{err}");
        let _ = format!("{err:?}");
        let _ = std::error::Error::source(err);
    }
}
