use pulldown_cmark::{HeadingLevel, InlineStr};
use std::error::Error;
pub fn heading(n: usize) -> Result<HeadingLevel, Box<dyn Error>> {
    Ok(HeadingLevel::try_from(n)?)
}
pub fn inline(s: &str) -> Result<InlineStr, Box<dyn Error>> {
    Ok(InlineStr::try_from(s)?)
}
