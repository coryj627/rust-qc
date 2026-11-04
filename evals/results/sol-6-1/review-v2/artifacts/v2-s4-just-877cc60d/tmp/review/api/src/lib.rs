//! Ordinary public API checks for the local review.

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn invalid_datetime_returns_error() {
        assert!(
            just::run(
                [
                    "just",
                    "--justfile",
                    "/work/tmp/review/datetime/justfile",
                    "--evaluate",
                    "x",
                ]
                .into_iter()
            )
            .is_err()
        );
    }

    #[test]
    fn valid_datetime_succeeds() {
        assert!(
            just::run(
                [
                    "just",
                    "--justfile",
                    "/work/tmp/review/api/valid.just",
                    "--evaluate",
                    "x",
                ]
                .into_iter()
            )
            .is_ok()
        );
    }

    #[test]
    fn invalid_regex_returns_error() {
        assert!(
            just::run(
                [
                    "just",
                    "--justfile",
                    "/work/tmp/review/api/regex.just",
                    "--evaluate",
                    "x",
                ]
                .into_iter()
            )
            .is_err()
        );
    }

    #[test]
    fn summary_returns_parse_error() {
        let result = just::summary::summary(Path::new("/work/tmp/review/api/invalid.just"))
            .expect("reading an existing local fixture should succeed");
        assert!(result.is_err());
    }

    #[test]
    fn unindent_preserves_unicode() {
        assert_eq!(just::unindent("\n  café\n  🤖\n"), "café\n🤖\n");
    }
}
