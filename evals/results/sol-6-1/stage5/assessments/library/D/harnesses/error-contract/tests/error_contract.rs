use byte_span::ByteSpan;

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorClass {
    Reversed,
    Bounds,
    Utf8Boundary,
}

#[path = "support/error_adapter.rs"]
mod adapter;

#[test]
fn construction_error_is_machine_distinguishable() {
    for (start, end) in [(1, 0), (9, 3), (usize::MAX, 0)] {
        let error = ByteSpan::new(start, end).unwrap_err();
        assert_eq!(adapter::classify(&error), ErrorClass::Reversed);
    }
}

#[test]
fn bounds_and_boundary_errors_are_machine_distinguishable() {
    for (text, start, end) in [
        ("abc", 0, 4),
        ("", 0, 1),
        ("abc", 4, 4),
        ("é", 0, usize::MAX),
    ] {
        let error = ByteSpan::new(start, end).unwrap().slice(text).unwrap_err();
        assert_eq!(adapter::classify(&error), ErrorClass::Bounds);
    }
    for (text, start, end) in [
        ("é", 1, 2),
        ("é", 0, 1),
        ("é", 1, 1),
        ("中", 1, 2),
        ("🦀", 3, 4),
    ] {
        let error = ByteSpan::new(start, end).unwrap().slice(text).unwrap_err();
        assert_eq!(adapter::classify(&error), ErrorClass::Utf8Boundary);
    }
}

#[test]
fn bounds_failure_precedes_invalid_utf8_endpoint() {
    // Each start is inside a code point AND each end is out of bounds.
    for (text, start, end) in [("é", 1, 3), ("中", 2, 5), ("🦀", 1, usize::MAX)] {
        let error = ByteSpan::new(start, end).unwrap().slice(text).unwrap_err();
        assert_eq!(adapter::classify(&error), ErrorClass::Bounds);
    }
}
