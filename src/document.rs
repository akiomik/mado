use std::fs;
use std::path::{Path, PathBuf};

use comrak::nodes::AstNode;
use comrak::{parse_document, Arena, Options};
use miette::IntoDiagnostic;
use miette::Result;

pub struct Document<'a> {
    pub path: PathBuf,
    pub ast: &'a AstNode<'a>,
    pub text: String,
}

impl<'a> Document<'a> {
    pub fn open(arena: &'a Arena<AstNode<'a>>, path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).into_diagnostic()?;
        let ast = parse_document(arena, &text, &Options::default());

        Ok(Self {
            path: path.to_path_buf(),
            // ast: None,
            ast,
            text,
        })
    }
}
