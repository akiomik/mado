#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tag {
    Atx,
    AtxClosed,
    BlankLines,
    Blockquote,
    Bullet,
    Code,
    Emphasis,
    HardTab,
    Headers,
    Hr,
    Html,
    Indentation,
    Language,
    LineLength,
    Links,
    Ol,
    Spaces,
    Ul,
    Url,
    Whitespace,
}
