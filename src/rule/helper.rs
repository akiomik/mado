extern crate alloc;

use alloc::borrow::Cow;
use comrak::nodes::{AstNode, NodeValue};

pub fn inline_text_of<'a>(root: &'a AstNode<'a>) -> String {
    let texts: Vec<String> = root
        .descendants()
        .filter_map(|node| match node.data.borrow().value.clone() {
            NodeValue::Text(text) => Some(text),
            NodeValue::Code(code) => Some(format!("`{}`", code.literal).into()),
            _ => None,
        })
        .map(Cow::into_owned)
        .collect();

    texts.join("")
}

#[cfg(test)]
mod tests {
    use comrak::{Arena, Options, parse_document};
    use miette::{Context as _, Result};
    use pretty_assertions::assert_eq;

    use super::*;

    // TODO: Test more inline nodes
    #[test]
    fn test_inline_text_of() -> Result<()> {
        let text = "# Heading with `code`, [link](http://example.com) and **bold**";
        let arena = Arena::new();
        let ast = parse_document(&arena, text, &Options::default());
        let heading = ast
            .first_child()
            .wrap_err("failed to get the first child")?;
        let actual = inline_text_of(heading);
        let expected = "Heading with `code`, link and bold";
        assert_eq!(actual, expected);
        Ok(())
    }
}
