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

    /// The regions comrak unescaped, by the line they were written on. See
    /// [`Document::written_position`].
    unescaped_regions: FxHashMap<usize, Vec<UnescapedRegion>>,
}

/// A run of one line's columns that comrak unescaped the pipes of before it
/// parsed the inlines in it.
#[derive(Debug, Clone)]
struct UnescapedRegion {
    /// The column the region begins at, which comrak reports and the line was
    /// written with alike: nothing has been dropped yet where a region starts.
    start: usize,

    /// The columns comrak dropped, as written, in the order they appear.
    dropped: Vec<usize>,
}

impl<'a> Document<'a> {
    #[inline]
    pub fn new(arena: &'a Arena<'a>, path: PathBuf, text: String) -> Result<Self> {
        let mut options = Options::default();
        options.extension.front_matter_delimiter = Some("---".to_owned());
        options.extension.table = true;
        let ast = parse_document(arena, &text, &options);
        let lines: Vec<_> = text.lines().map(ToOwned::to_owned).collect();
        let unescaped_regions = Self::unescaped_regions(ast, &lines, &text);

        Ok(Self {
            path,
            ast,
            text,
            lines,
            unescaped_regions,
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
    /// same way, and is corrected the same way. A position on a line that
    /// carries neither, or before the first region on a line that does, is
    /// returned unchanged.
    ///
    /// The pipe is the only escape this knows about, and it is the only one
    /// comrak resolves before it measures: a column that a `\<punctuation>`
    /// escape put wrong was never shifted by comrak and cannot be found from
    /// here. #406 covers those.
    ///
    /// A column is put back on the line by the character comrak reports at it,
    /// so a caller whose own arithmetic named the byte *after* a span — an
    /// exclusive end — has to hand over the span's last byte and step past the
    /// column that comes back, rather than hand over the byte after it.
    #[inline]
    #[must_use]
    pub fn written_position(&self, position: Sourcepos) -> Sourcepos {
        let mut written = position;
        written.start.column = self.written_column(position.start.line, position.start.column);
        written.end.column = self.written_column(position.end.line, position.end.column);
        written
    }

    /// The column as written, for one comrak reports on `line`.
    ///
    /// A column belongs to the last region that begins at or before it. A
    /// region reaches to where the next one begins rather than to where its
    /// content stops, so the columns between the two carry its shift as well —
    /// comrak reports none of them, and answering with the shift keeps a column
    /// at the end of a cell next to the one before it instead of jumping back.
    fn written_column(&self, line: usize, column: usize) -> usize {
        let Some(regions) = self.unescaped_regions.get(&line) else {
            return column;
        };

        let Some(region) = regions.iter().rev().find(|region| region.start <= column) else {
            return column;
        };

        // Each column comrak dropped at or before where the column has reached
        // is a byte it never reported, so the column moves one further right.
        let mut written = column;
        for &dropped in &region.dropped {
            if dropped > written {
                break;
            }

            written += 1;
        }

        written
    }

    /// The regions [`Document::written_position`] reads, keyed by line.
    ///
    /// Only a line that carries a region comrak dropped a byte from gets an
    /// entry; on every other line the two columns are equal, and leaving those
    /// out keeps this empty for the documents that have no escape at all. An
    /// entry holds every region of its line in the order they were written, so
    /// that a column can be answered for by the one it falls in rather than by
    /// the one before it.
    fn unescaped_regions(
        ast: &'a AstNode<'a>,
        lines: &[String],
        text: &str,
    ) -> FxHashMap<usize, Vec<UnescapedRegion>> {
        let mut unescaped_regions = FxHashMap::default();

        // Walking the tree costs more than the escape is common, and a document
        // written without one has no shifted column to correct.
        if !text.contains(r"\|") {
            return unescaped_regions;
        }

        for node in ast.descendants() {
            let position = node.data.borrow().sourcepos;

            match node.data.borrow().value {
                // A row is taken whole because its cells are unescaped one at a
                // time: a cell after an escaped one is where the shift stops
                // rather than carries on, and it can only say so by being here.
                // The cells' own `sourcepos` is built from the raw line rather
                // than from the unescaped content, so it is unshifted and gives
                // each region's bounds.
                NodeValue::TableRow(_) => {
                    let cells: Vec<_> = node
                        .children()
                        .map(|cell| {
                            let cell_position = cell.data.borrow().sourcepos;
                            Self::unescaped_region(
                                lines,
                                cell_position.start.line,
                                cell_position.start.column,
                                cell_position.end.column,
                            )
                        })
                        .collect();

                    if cells.iter().any(|cell| !cell.dropped.is_empty()) {
                        unescaped_regions.insert(position.start.line, cells);
                    }
                }
                // The preface is unescaped as one string, but the inline parser
                // measures each of its lines from that line's own offset, so
                // each is shifted by the escapes written on it alone. A line of
                // it is content and indentation and nothing else, and an escape
                // cannot be in the indentation, so the whole of it is one
                // region.
                NodeValue::Paragraph if Self::is_table_preface(node) => {
                    for line in position.start.line..=position.end.line {
                        let region = Self::unescaped_region(lines, line, 1, usize::MAX);
                        if !region.dropped.is_empty() {
                            unescaped_regions.insert(line, vec![region]);
                        }
                    }
                }
                _ => {}
            }
        }

        unescaped_regions
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

    /// The region `line` holds from column `start` to column `end`, with `end`
    /// clamped to the line.
    fn unescaped_region(
        lines: &[String],
        line_number: usize,
        start: usize,
        end: usize,
    ) -> UnescapedRegion {
        // A region's `sourcepos` names bytes of the line it was parsed from, so
        // the slice is there to take. One that somehow is not carries no escape
        // either, and leaves as a region nothing was dropped from.
        let region = lines
            .get(line_number - 1)
            .and_then(|line| line.get(start - 1..end.min(line.len())))
            .unwrap_or_default();

        UnescapedRegion {
            start,
            dropped: Self::dropped_columns(region, start),
        }
    }

    /// The columns of `region`, which starts at column `start`, that comrak
    /// drops before it parses the inlines in it.
    ///
    /// Those are the backslashes of the region's `\|` escapes, and nothing
    /// else: it is unescaped for its pipes alone, and every other backslash
    /// reaches the inline parser, which records the columns it was written at.
    ///
    /// A backslash that is itself escaped does not start an escape, so the run
    /// is walked rather than the pairs matched: in `\\|` the second backslash
    /// is the first one's escape, and the pipe stands on its own. comrak leaves
    /// all three bytes where they were written, and so does this.
    fn dropped_columns(region: &str, start: usize) -> Vec<usize> {
        let mut dropped = vec![];
        let mut after_backslash = false;

        for (offset, byte) in region.bytes().enumerate() {
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
        // shift starts at the first escape, so the row's opening delimiter, the
        // `x` before that escape, and the `c` in the next cell, which was
        // written without one, are all where they say.
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 9, 3, 9))),
            Sourcepos::from((3, 11, 3, 11))
        );
        assert_eq!(
            doc.written_position(Sourcepos::from((3, 1, 3, 3))),
            Sourcepos::from((3, 1, 3, 3))
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
    // in it, so the columns at its end are ones comrak never reports from
    // inside it. A region reaches to where the next one begins rather than to
    // where its content stops, so those still answer with its shift instead of
    // falling back on themselves.
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
