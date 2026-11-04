use pulldown_cmark::{html, Event, LinkType, Options, Parser, Tag};

fn inspect<'a>(id: &str, input: &'a str, options: Options) -> (Vec<Event<'a>>, String) {
    let events: Vec<_> = Parser::new_ext(input, options).collect();
    let mut rendered = String::new();
    html::push_html(&mut rendered, events.iter().cloned());
    println!("{id}: input={input:?}\nevents={events:?}\nhtml={rendered:?}");
    (events, rendered)
}

#[test]
fn decoy_1_backtick_runs_match_exactly() {
    for (input, expected) in [
        ("`left``middle` tail", "<p><code>left``middle</code> tail</p>\n"),
        ("prefix ``` unmatched `left``middle` tail", "<p>prefix ``` unmatched <code>left``middle</code> tail</p>\n"),
    ] {
        let (events, rendered) = inspect("decoy-1", input, Options::empty());
        assert_eq!(rendered, expected);
        let code: Vec<_> = events.iter().filter_map(|e| match e { Event::Code(s) => Some(s.as_ref()), _ => None }).collect();
        assert_eq!(code, ["left``middle"]);
    }
}

#[test]
fn decoy_2_table_code_removes_pipe_escape() {
    let input = "| code | value |\n| --- | --- |\n| `a\\|b` | z |\n";
    let (events, rendered) = inspect("decoy-2", input, Options::ENABLE_TABLES);
    assert!(events.iter().any(|e| matches!(e, Event::Code(s) if s.as_ref() == "a|b")));
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Start(Tag::TableCell))).count(), 4);
    assert_eq!(rendered, "<table><thead><tr><th>code</th><th>value</th></tr></thead><tbody>\n<tr><td><code>a|b</code></td><td>z</td></tr>\n</tbody></table>\n");
    // Outside tables, backslash remains literal inside code as required.
    let (_, rendered) = inspect("decoy-2/outside-table", "`a\\|b`", Options::empty());
    assert_eq!(rendered, "<p><code>a\\|b</code></p>\n");
}

#[test]
fn false_1_first_reference_definition_wins() {
    let input = "[pick][id]\n\n[id]: /first \"First\"\n[id]: /second \"Second\"\n";
    let parser = Parser::new(input);
    let definition = parser.reference_definitions().get("id").unwrap();
    assert_eq!(definition.dest.as_ref(), "/first");
    assert_eq!(definition.title.as_ref().unwrap().as_ref(), "First");
    let (events, rendered) = inspect("false-1", input, Options::empty());
    assert!(events.iter().any(|e| matches!(e, Event::Start(Tag::Link { dest_url, title, .. }) if dest_url.as_ref() == "/first" && title.as_ref() == "First")));
    assert_eq!(rendered, "<p><a href=\"/first\" title=\"First\">pick</a></p>\n");
}

#[test]
fn false_2_image_alt_consumes_nested_ends() {
    for (input, expected) in [
        ("![before *inner* after](image.png) tail", "<p><img src=\"image.png\" alt=\"before inner after\" /> tail</p>\n"),
        ("![before ![inner](small.png) after](image.png) tail", "<p><img src=\"image.png\" alt=\"before inner after\" /> tail</p>\n"),
    ] {
        let (_, rendered) = inspect("false-2", input, Options::empty());
        assert_eq!(rendered, expected);
    }
}

#[test]
fn false_3_invalid_numeric_entities_use_replacement_character() {
    for input in ["&#xD800;", "&#x110000;", "&#1114112;", "&#0;"] {
        let (events, rendered) = inspect("false-3", input, Options::empty());
        assert!(events.iter().any(|e| matches!(e, Event::Text(s) if s.as_ref() == "\u{FFFD}")));
        assert_eq!(rendered, "<p>\u{FFFD}</p>\n");
    }
    let (_, rendered) = inspect("false-3/valid-boundary", "&#x10FFFF;", Options::empty());
    assert_eq!(rendered, "<p>\u{10FFFF}</p>\n");
}

#[test]
fn false_4_autolink_scheme_inclusive_upper_boundary() {
    for length in [2, 31, 32, 33] {
        let destination = format!("{}:item", "a".repeat(length));
        let input = format!("<{destination}>");
        let (events, rendered) = inspect("false-4", &input, Options::empty());
        let has_autolink = events.iter().any(|e| matches!(e, Event::Start(Tag::Link { link_type: LinkType::Autolink, dest_url, .. }) if dest_url.as_ref() == destination));
        assert_eq!(has_autolink, length <= 32);
        if length <= 32 {
            assert_eq!(rendered, format!("<p><a href=\"{destination}\">{destination}</a></p>\n"));
        }
    }
}

#[test]
fn false_5_balanced_parentheses_stay_in_link_destination() {
    for destination in ["a(b)c", "a(b(c)d)e"] {
        let input = format!("[label]({destination}) tail");
        let (events, rendered) = inspect("false-5", &input, Options::empty());
        assert!(events.iter().any(|e| matches!(e, Event::Start(Tag::Link { dest_url, .. }) if dest_url.as_ref() == destination)));
        assert_eq!(rendered, format!("<p><a href=\"{destination}\">label</a> tail</p>\n"));
    }
}
