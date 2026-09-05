use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::{Arena, Options, parse_document};
use miette::IntoDiagnostic as _;
use miette::Result;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Document<'a> {
    pub path: PathBuf,
    pub ast: &'a AstNode<'a>,
    pub text: String,
    pub lines: Vec<String>,

    /// Where each column comrak reports on a line was written, for the lines
    /// where the two part company. See [`Document::written_position`].
    written_columns: HashMap<usize, Vec<usize>>,
}

impl<'a> Document<'a> {
    #[inline]
    pub fn new(arena: &'a Arena<'a>, path: PathBuf, text: String) -> Result<Self> {
        let mut options = Options::default();
        options.extension.front_matter_delimiter = Some("---".to_owned());
        options.extension.table = true;
        let ast = parse_document(arena, &text, &options);
        let lines: Vec<_> = text.lines().map(ToOwned::to_owned).collect();
        let written_columns = Self::written_columns(ast, &lines, &text);

        Ok(Self {
            path,
            ast,
            text,
            lines,
            written_columns,
        })
    }

    #[inline]
    pub fn open(arena: &'a Arena<'a>, path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).into_diagnostic()?;
        Self::new(arena, path.to_path_buf(), text)
    }

    /// The position as written, for one comrak measured against a table cell.
    ///
    /// comrak unescapes a table cell before it parses the cell's inlines, so a
    /// column reported from inside one counts each `\|` the cell was written
    /// with as the single byte it was unescaped to. Every inline after the
    /// escape therefore lands one column to the left of where it was written,
    /// and one more for each further escape. This puts those bytes back, so the
    /// column names the character a reader sees at it.
    ///
    /// A position from anywhere else is returned unchanged, and so is one
    /// outside the line it names — a rule that has done its own column
    /// arithmetic can hand over either.
    #[inline]
    #[must_use]
    pub fn written_position(&self, position: Sourcepos) -> Sourcepos {
        let mut written = position;
        written.start.column = self.written_column(position.start.line, position.start.column);
        written.end.column = self.written_column(position.end.line, position.end.column);
        written
    }

    fn written_column(&self, line: usize, column: usize) -> usize {
        self.written_columns
            .get(&line)
            .and_then(|columns| columns.get(column))
            .copied()
            .unwrap_or(column)
    }

    /// The column table [`Document::written_position`] reads, keyed by line.
    ///
    /// Only a line carrying a table cell that was written with a `\|` gets an
    /// entry; on every other line the two columns are equal, and leaving those
    /// out keeps the table empty for the documents that have no escape at all.
    /// Within an entry, index and value are the reported and the written
    /// column, and an entry reaches as far as the last cell it was built from —
    /// a column past its end is one nothing shifted, and reading past the end
    /// is how [`Document::written_column`] answers for those.
    ///
    /// A cell's own `sourcepos` is built from the raw line rather than from the
    /// unescaped content, so it is unshifted and gives the region to walk.
    fn written_columns(
        ast: &'a AstNode<'a>,
        lines: &[String],
        text: &str,
    ) -> HashMap<usize, Vec<usize>> {
        let mut written_columns = HashMap::new();

        // Walking the tree costs more than the escape is common, and a document
        // written without one has no shifted column to correct.
        if !text.contains(r"\|") {
            return written_columns;
        }

        for node in ast.descendants() {
            if !matches!(node.data.borrow().value, NodeValue::TableCell) {
                continue;
            }

            let position = node.data.borrow().sourcepos;
            let start = position.start.column;

            // A cell's `sourcepos` names bytes of the line it was parsed from,
            // so the slice is there to take. One that somehow is not carries no
            // escape either, and leaves with the cells that were written
            // without one.
            let cell = lines
                .get(position.start.line - 1)
                .and_then(|line| line.get(start - 1..position.end.column.min(line.len())))
                .unwrap_or_default();

            let dropped = Self::dropped_columns(cell, start);
            if dropped.is_empty() {
                continue;
            }

            // Identity as far as this cell reaches, for the columns before its
            // first escape and for the cells on the line that had none.
            let end = start + cell.len() - 1;
            let columns = written_columns.entry(position.start.line).or_default();
            if columns.len() <= end {
                let identity = columns.len()..=end;
                columns.extend(identity);
            }

            let mut dropped = dropped.into_iter().peekable();
            let mut reported = start;
            for written in start..=end {
                // A dropped column is one comrak never reports, so it is passed
                // over and every column after it in the cell moves right by one.
                if dropped.next_if_eq(&written).is_some() {
                    continue;
                }

                columns[reported] = written;
                reported += 1;
            }
        }

        written_columns
    }

    /// The columns of `cell`, which starts at column `start`, that comrak drops
    /// before it parses the cell's inlines.
    ///
    /// Those are the backslashes of the cell's `\|` escapes, and nothing else:
    /// a table cell is unescaped for its pipes alone, and every other backslash
    /// reaches the inline parser, which records the columns it was written at.
    ///
    /// A backslash that is itself escaped does not start an escape, so the run
    /// is walked rather than the pairs matched: in `\\|` the second backslash
    /// is the first one's escape, and the pipe stands on its own. comrak leaves
    /// all three bytes where they were written, and so does this.
    fn dropped_columns(cell: &str, start: usize) -> Vec<usize> {
        let mut dropped = vec![];
        let mut after_backslash = false;

        for (offset, byte) in cell.bytes().enumerate() {
            if after_backslash {
                if byte == b'|' {
                    dropped.push(start + offset - 1);
                }

                after_backslash = false;
            } else if byte == b'\\' {
                after_backslash = true;
            }
        }

        dropped
    }

    #[inline]
    #[must_use]
    pub fn front_matter(&self) -> Option<String> {
        if let Some(node) = self.ast.first_child()
            && let NodeValue::FrontMatter(front_matter) = &node.data.borrow().value
        {
            return Some(front_matter.clone());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn open() {
        let arena = Arena::new();
        let path = Path::new("README.md");
        assert!(Document::open(&arena, path).is_ok());
    }

    #[test]
    fn front_matter_some() -> Result<()> {
        let front_matter = indoc! {"
            ---
            foo: bar
            ---

        "}
        .to_owned();
        let text = format!("{front_matter}text");
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(doc.front_matter(), Some(front_matter));
        Ok(())
    }

    #[test]
    fn front_matter_none() -> Result<()> {
        let text = "text".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(doc.front_matter(), None);
        Ok(())
    }

    #[test]
    fn written_position_in_table_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y\|z w | c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;

        // `w` is at column 11, and comrak has it at 9 — one for each `\|`. The
        // shift starts at the first escape, so the `x` before it and the `c` in
        // the next cell, which was written without one, are where they say.
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 9, 3, 9))),
            Sourcepos::from((3, 11, 3, 11))
        );
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 3, 3, 3))),
            Sourcepos::from((3, 3, 3, 3))
        );
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 15, 3, 15))),
            Sourcepos::from((3, 15, 3, 15))
        );
        Ok(())
    }

    // Each cell is unescaped on its own, so the shift restarts at every one and
    // the table has to reach past the first to carry the second.
    #[test]
    fn written_position_in_two_table_cells() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y w | c\|d v |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;

        // `w` is at column 8 and `v` at column 17, each one to the right of
        // where comrak has it — the second because of its own cell's escape,
        // not because the first cell's is still being counted.
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 7, 3, 7))),
            Sourcepos::from((3, 8, 3, 8))
        );
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 16, 3, 16))),
            Sourcepos::from((3, 17, 3, 17))
        );
        Ok(())
    }

    // `\\` is an escaped backslash, so the pipe after it stands on its own and
    // comrak drops nothing. Matching `\|` as a pair would find one across the
    // two and shift every column after it that was never shifted.
    #[test]
    fn written_position_with_escaped_backslash() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\\|y w | c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((3, 8, 3, 8));
        assert_eq!(doc.written_position(position), position);
        Ok(())
    }

    #[test]
    fn written_position_without_escaped_pipe() -> Result<()> {
        let text = indoc! {"
            | a | b |
            | --- | --- |
            | x | c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((3, 3, 3, 3));
        assert_eq!(doc.written_position(position), position);
        Ok(())
    }

    // An escape outside a table is `CommonMark`'s own, and comrak reports the
    // columns of the line as written for it.
    #[test]
    fn written_position_outside_table() -> Result<()> {
        let text = r"a \| b".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((1, 3, 1, 4));
        assert_eq!(doc.written_position(position), position);
        Ok(())
    }

    #[test]
    fn front_matter_empty() -> Result<()> {
        let text = String::new();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(doc.front_matter(), None);
        Ok(())
    }
}
