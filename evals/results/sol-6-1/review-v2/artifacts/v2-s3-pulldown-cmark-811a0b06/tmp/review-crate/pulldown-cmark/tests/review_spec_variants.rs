use pulldown_cmark::{html, Parser};
fn render(s: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new(s));
    out
}
fn normalize(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n").replace("<br>", "<br />").replace("<br/>", "<br />")
        .replace("<hr>", "<hr />").replace("<hr/>", "<hr />").replace(">\n<", "><")
}
#[test]
fn commonmark_fixtures_with_default_options_and_line_endings() {
    let spec = include_str!("../third_party/CommonMark/spec.txt");
    let mut counts = [0; 3];
    let mut fixtures = 0;
    for section in spec.split("```````````````````````````````` example\n").skip(1) {
        let (input, rest) = section.split_once("\n.\n").unwrap();
        let input = input.replace('→', "\t") + "\n";
        let expected = rest.split_once("````````````````````````````````\n").unwrap().0.replace('→', "\t");
        fixtures += 1;
        let baseline = render(&input);
        let values = [baseline.clone(), render(&input.replace('\n', "\r\n")), render(&input.replace('\n', "\r"))];
        for (mode, actual) in values.iter().enumerate() {
            if normalize(actual) != normalize(&expected) {
                counts[mode] += 1;
                if counts[mode] <= 5 {
                    println!("mode={mode} example={fixtures} input={input:?}\nexpected={expected:?}\nactual={actual:?}");
                }
            }
        }
    }
    println!("fixtures={fixtures}, mismatches [LF, CRLF, CR]={counts:?}");
    assert_eq!(counts, [0; 3]);
}
