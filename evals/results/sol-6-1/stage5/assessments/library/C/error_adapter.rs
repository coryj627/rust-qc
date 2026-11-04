// Assessor-owned adapter for candidate C; reviewed against its public enum.
// Maps public discriminants only, without inspecting messages or call context.
pub fn classify(error: &byte_span::SpanError) -> super::ErrorClass {
    match error {
        byte_span::SpanError::Reversed { .. } => super::ErrorClass::Reversed,
        byte_span::SpanError::OutOfBounds { .. } => super::ErrorClass::Bounds,
        byte_span::SpanError::InvalidCharBoundary { .. } => super::ErrorClass::Utf8Boundary,
    }
}
