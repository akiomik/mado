use std::fs;
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use comrak::{Arena, Options, parse_document};
use miette::IntoDiagnostic as _;
use miette::Result;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Document<'a> {
    pub path: PathBuf,
    pub ast: &'a AstNode<'a>,
    pub text: String,
    pub lines: Vec<String>,

    /// Where each column comrak reports on a line was written, for the lines
    /// where the two part company. See [`Document::written_position`].
    written_columns: FxHashMap<usize, Vec<usize>>,
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

    /// The position as written, for one comrak measured against a region it
    /// unescaped the pipes of.
    ///
    /// comrak unescapes a table cell before it parses the cell's inlines, so a
    /// column reported from inside one counts each `\|` the cell was written
    /// with as the single byte it was unescaped to. Every inline after the
    /// escape therefore lands one column to the left of where it was written,
    /// and one more for each further escape. This puts those bytes back, so the
    /// column names the character a reader sees at it.
    ///
    /// The paragraph comrak splits off a table's header row is unescaped the
    /// same way, and is corrected the same way. A position from anywhere else
    /// is returned unchanged, and so is one outside the region it names — a
    /// rule that has done its own column arithmetic can hand over either.
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
    /// Only a line carrying a region that was written with a `\|` and unescaped
    /// for it gets an entry; on every other line the two columns are equal, and
    /// leaving those out keeps the table empty for the documents that have no
    /// escape at all. Within an entry, index and value are the reported and the
    /// written column, and an entry reaches as far as the last region it was
    /// built from — a column past that is one nothing shifted, and reading past
    /// the end is how [`Document::written_column`] answers for those.
    fn written_columns(
        ast: &'a AstNode<'a>,
        lines: &[String],
        text: &str,
    ) -> FxHashMap<usize, Vec<usize>> {
        let mut written_columns = FxHashMap::default();

        // Walking the tree costs more than the escape is common, and a document
        // written without one has no shifted column to correct.
        if !text.contains(r"\|") {
            return written_columns;
        }

        for node in ast.descendants() {
            let position = node.data.borrow().sourcepos;

            match node.data.borrow().value {
                // A cell's own `sourcepos` is built from the raw line rather
                // than from the unescaped content, so it is unshifted and gives
                // the region to walk.
                NodeValue::TableCell => Self::map_unescaped(
                    &mut written_columns,
                    lines,
                    position.start.line,
                    position.start.column,
                    position.end.column,
                ),
                // The preface is unescaped as one string, but the inline parser
                // measures each of its lines from that line's own offset, so
                // each is shifted by the escapes written on it alone. A line of
                // it is content and indentation and nothing else, and an escape
                // cannot be in the indentation, so the whole of it is walked.
                NodeValue::Paragraph if Self::is_table_preface(node) => {
                    for line in position.start.line..=position.end.line {
                        Self::map_unescaped(&mut written_columns, lines, line, 1, usize::MAX);
                    }
                }
                _ => {}
            }
        }

        written_columns
    }

    /// Whether this paragraph is the one comrak split off a table's header row.
    ///
    /// A table interrupts the paragraph its header row was written in, and what
    /// came before that row is moved into a paragraph of its own — with its
    /// pipes unescaped on the way, the same as a cell's. Nothing else leaves a
    /// paragraph on the line directly above a table: a table can only begin by
    /// converting an open paragraph, so without a blank line between them the
    /// two came from one block and this is that split, and with one there is a
    /// line between them that this does not match.
    fn is_table_preface(node: &'a AstNode<'a>) -> bool {
        let Some(next) = node.next_sibling() else {
            return false;
        };

        matches!(next.data.borrow().value, NodeValue::Table(_))
            && next.data.borrow().sourcepos.start.line == node.data.borrow().sourcepos.end.line + 1
    }

    /// Records where the columns of one unescaped region were written.
    ///
    /// The region is `line`'s bytes from `start` to `end`, both columns, with
    /// `end` clamped to the line. A region comrak dropped nothing from is not
    /// recorded: its columns are already the ones it was written with.
    fn map_unescaped(
        written_columns: &mut FxHashMap<usize, Vec<usize>>,
        lines: &[String],
        line_number: usize,
        start: usize,
        end: usize,
    ) {
        // A region's `sourcepos` names bytes of the line it was parsed from, so
        // the slice is there to take. One that somehow is not carries no escape
        // either, and leaves with the regions written without one.
        let region = lines
            .get(line_number - 1)
            .and_then(|line| line.get(start - 1..end.min(line.len())))
            .unwrap_or_default();

        let dropped = Self::dropped_columns(region, start);
        if dropped.is_empty() {
            return;
        }

        // Identity as far as this region reaches, for the columns before its
        // first escape and for the regions on the line that had none.
        let last = start + region.len() - 1;
        let columns = written_columns.entry(line_number).or_default();
        if columns.len() <= last {
            let identity = columns.len()..=last;
            columns.extend(identity);
        }

        let shift = dropped.len();
        let mut dropped = dropped.into_iter().peekable();
        let mut reported = start;
        for written in start..=last {
            // A dropped column is one comrak never reports, so it is passed
            // over and every column after it in the region moves right by one.
            if dropped.next_if_eq(&written).is_some() {
                continue;
            }

            columns[reported] = written;
            reported += 1;
        }

        // comrak reports one column fewer than the region was written with for
        // each it dropped, so the columns at the region's end are left over.
        // Those are where a rule's own arithmetic lands when it names the byte
        // after the last one — an exclusive end — and past the region's content
        // the whole of its shift applies.
        for (offset, written) in columns[reported..=last].iter_mut().enumerate() {
            *written = reported + offset + shift;
        }
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

    // A cell reports one column fewer than it was written with for each escape
    // in it, so its last written columns are ones no inline starts at. A rule
    // still names them when it counts an exclusive end off a span that runs to
    // the cell's end, and the cell's whole shift applies to those.
    #[test]
    fn written_position_past_a_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            |x\|y| c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;

        // `y` is the cell's last character, at column 5, and column 6 is the
        // delimiter an exclusive end off it names.
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 4, 3, 5))),
            Sourcepos::from((3, 5, 3, 6))
        );
        Ok(())
    }

    // comrak also unescapes the paragraph it splits off a table's header row,
    // measuring each of its lines from that line's own offset, so a line of it
    // is shifted by the escapes written on it alone.
    #[test]
    fn written_position_in_a_table_header_preface() -> Result<()> {
        let text = indoc! {r"
            foo x\|y bar
            baz a\|b\|c qux
            | a | b |
            | --- | --- |
            | c | d |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;

        // `bar` is at column 10 on its line and `qux` at column 13 on the next,
        // one and two to the right of where comrak has them.
        assert_eq!(
            doc.written_position(Sourcepos::from((1, 9, 1, 11))),
            Sourcepos::from((1, 10, 1, 12))
        );
        assert_eq!(
            doc.written_position(Sourcepos::from((2, 11, 2, 13))),
            Sourcepos::from((2, 13, 2, 15))
        );
        Ok(())
    }

    // A blank line between the two leaves an ordinary paragraph, whose escape
    // is `CommonMark`'s own and whose columns comrak already reports as written.
    #[test]
    fn written_position_in_a_paragraph_above_a_table() -> Result<()> {
        let text = indoc! {r"
            foo x\|y bar

            | a | b |
            | --- | --- |
            | c | d |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((1, 10, 1, 12));
        assert_eq!(doc.written_position(position), position);
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
