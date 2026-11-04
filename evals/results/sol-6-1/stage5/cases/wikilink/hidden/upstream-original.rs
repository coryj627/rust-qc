    #[cfg(feature = "html")]
    #[test]
    fn wikilink_malformed_no_subtract_overflow() {
        // Regression for #1108: on malformed image/wikilink input the
        // wikilink body could begin at or after the closing delimiter, so
        // `end_ix - start_ix` underflowed (panicking in debug) and later
        // sliced an inverted range. handle_wikilink must bail early instead.
        let test_str = " \u{fffd}  ![[] ]()]]";

        let mut buf = String::new();
        crate::html::push_html(&mut buf, Parser::new_ext(test_str, Options::all()));
        assert!(!buf.is_empty());
    }
