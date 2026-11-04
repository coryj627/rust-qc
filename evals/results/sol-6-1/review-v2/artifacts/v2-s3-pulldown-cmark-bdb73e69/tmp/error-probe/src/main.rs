use pulldown_cmark::{HeadingLevel, InlineStr};
fn heading() -> Result<HeadingLevel, Box<dyn std::error::Error>> {
    Ok(HeadingLevel::try_from(7)?)
}
fn inline() -> Result<InlineStr, Box<dyn std::error::Error>> {
    Ok(InlineStr::try_from("a string too long to fit in the inline buffer")?)
}
fn main() {
    let _ = (heading(), inline());
}
