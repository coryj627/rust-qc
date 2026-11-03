// Assessor-owned adapter for candidate A; reviewed against its public enum.
// Maps public discriminants only, without inspecting messages or call context.
pub fn classify(error: &byte_span::SpanError) -> super::ErrorClass {
    match error {
        byte_span::SpanError::Reversed { .. } => super::ErrorClass::Reversed,
        byte_span::SpanError::OutOfBounds { .. } => super::ErrorClass::Bounds,
        byte_span::SpanError::InvalidUtf8Boundary { .. } => super::ErrorClass::Utf8Boundary,
        // The public enum is non_exhaustive; fail explicitly on an unknown tag.
        _ => panic!("unrecognized public SpanError variant"),
    }
}
