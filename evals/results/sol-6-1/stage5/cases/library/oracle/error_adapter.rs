pub fn classify(error: &byte_span::SpanError) -> super::ErrorClass {
    match error {
        byte_span::SpanError::InvalidOrder => super::ErrorClass::Reversed,
        byte_span::SpanError::OutsideText => super::ErrorClass::Bounds,
        byte_span::SpanError::NotCharBoundary => super::ErrorClass::Utf8Boundary,
    }
}
