use pulldown_cmark::{html, Options, Parser};
fn main() {
    for text in [
        "[[Page|]] after",
        "![[a.png|]] after",
        "~a^ b~",
        "foo <![CDATA[a]b]]> bar",
    ] {
        let mut out = String::new();
        html::push_html(
            &mut out,
            Parser::new_ext(
                text,
                Options::ENABLE_WIKILINKS | Options::ENABLE_SUBSCRIPT | Options::ENABLE_SUPERSCRIPT,
            ),
        );
        println!("{text:?} => {out:?}");
    }
    for text in [
        "```\nx\n```",
        "```\r\nx\r\n```",
        "```\rx\r```",
        "one \t\ntwo",
        "one\t \ntwo",
    ] {
        let mut out = String::new();
        html::push_html(&mut out, Parser::new(text));
        println!("{text:?} => {out:?}");
    }
}
