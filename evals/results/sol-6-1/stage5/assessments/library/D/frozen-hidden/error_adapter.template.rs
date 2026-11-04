// Grader-owned adapter; copy to /work/tmp/candidate_error_adapter.rs and replace
// the body using ONLY the candidate's public, machine-readable error API.
// Do not parse Display/Debug, inspect private fields, or infer categories from
// the call arguments. Return categories based on the supplied error alone.
// Matching enum variants, a kind() method, public tags, or public predicates
// are all acceptable. Multiple concrete variants may map to one category.
// If impossible, document that product defect instead of inventing an adapter.
pub fn classify(error: &byte_span::SpanError) -> super::ErrorClass {
    let _ = error;
    todo!("map candidate public error categories to super::ErrorClass")
}
