extern crate alloc;

use alloc::borrow::Cow;
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
    /// The pipe is the only escape this knows about, and only where it is
    /// resolved ahead of the inline parser rather than by it — inside a cell,
    /// or in that paragraph. Written anywhere else it is resolved like any
    /// other escape, `\<punctuation>` and `\|` alike, and a column that leaves
    /// wrong is one comrak never shifted and nothing here can find. #406
    /// covers those.
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

    /// The column the byte at `offset` of a text node's `literal` was written
    /// at.
    ///
    /// A rule that counts an offset off a literal is counting on a string
    /// `CommonMark` has already resolved the escapes in: `\.` is two bytes on
    /// the line and one there, so an offset past one names a column to the left
    /// of the one it was written at, and one more for each further escape.
    /// Adding an offset to a column is what put the report to the left of the
    /// character it named; the offset is walked along the line here instead,
    /// where the escapes still are, and the column comes out of the walk.
    ///
    /// [`Document::written_position`] is where the walk starts, because a
    /// position from inside a table cell is measured against the unescaped cell
    /// rather than against the line. What that corrects and what this does are
    /// the two halves of one report: the position of the node, and the offsets
    /// counted off inside it.
    ///
    /// An offset past the end of the literal answers with the column after the
    /// node, which is where a caller naming the byte after a span lands.
    ///
    /// Where the line is not the literal's source, `line_text` says so, and the
    /// offset is added to the column comrak reported and that column put back
    /// on the line — which is what the rules did before any of this, and is
    /// wrong by whatever escapes the offset passed.
    #[inline]
    #[must_use]
    pub fn written_column_of(&self, position: Sourcepos, literal: &str, offset: usize) -> usize {
        match self.line_text(position, literal) {
            Some((text, column)) => column + Self::written_offset(text, offset),
            None => self.written_column(position.start.line, position.start.column + offset),
        }
    }

    /// The offset into `written` of the byte its literal has at `offset`.
    ///
    /// The two run together a character at a time, and part company only at an
    /// escape: its backslash is a byte of the line that the literal does not
    /// have, and the byte it guards stands there for the pair.
    fn written_offset(written: &str, offset: usize) -> usize {
        let mut literal_offset = 0;
        let mut chars = written.char_indices().peekable();

        while let Some((index, char)) = chars.next() {
            if literal_offset >= offset {
                return index;
            }

            // An escape is two bytes of the line and one of the literal, and
            // the one is the byte it guards, which is always punctuation and so
            // always a single byte.
            if char == '\\'
                && chars
                    .peek()
                    .is_some_and(|&(_, next)| next.is_ascii_punctuation())
            {
                chars.next();
                literal_offset += 1;
            } else {
                literal_offset += char.len_utf8();
            }
        }

        written.len()
    }

    /// The text `position` describes with the escapes in it masked out, and the
    /// column it starts at.
    ///
    /// An escaped marker is not a marker, and against the literal it cannot be
    /// told from one: `CommonMark` resolves `\*` before the literal is built,
    /// so a rule searching for emphasis reads the marker the author escaped to
    /// keep. Searching the line instead is not enough on its own — a marker is
    /// escaped by the byte *before* it, and only some of the places a search
    /// can find one have something before them to look at — so the escapes are
    /// taken out of the search rather than guarded against inside it.
    ///
    /// Each is replaced by as many bytes as it was written with, so every
    /// column the search reports is still the column the byte is at, and by a
    /// letter, which a search for markers and the whitespace around them can
    /// only read as text. `\\*` is an escaped backslash and then a marker, and
    /// comes back as one: the run is walked rather than the pairs matched, the
    /// same as everywhere else here.
    ///
    /// Where the line is not the literal's source, `line_text` says so and the
    /// literal answers for itself — with its escapes already resolved, and
    /// nothing on it to mask.
    #[inline]
    #[must_use]
    pub fn written_text_without_escapes<'t>(
        &'t self,
        position: Sourcepos,
        literal: &'t str,
    ) -> (Cow<'t, str>, usize) {
        match self.line_text(position, literal) {
            Some((text, column)) => (Self::without_escapes(text), column),
            None => (
                Cow::Borrowed(literal),
                self.written_position(position).start.column,
            ),
        }
    }

    /// The line `position` was written on, sliced to the columns it covers, and
    /// the column that slice starts at.
    ///
    /// `None` means the line is not the source `literal` was built from, which
    /// is checked rather than assumed. Two things are known to fail it. comrak
    /// measures the inlines after one that spans two lines — a link with its
    /// destination on the line below — from a line and a column that are both a
    /// line behind, and the slice then holds text the node was never built
    /// from; against the literal that is a wrong column, and against the line
    /// it would be a violation reported out of text the document does not have
    /// there. A character reference is resolved into the literal the way an
    /// escape is, and naming the character `&amp;` stands for takes the whole
    /// HTML5 table, so a node holding one does not read back either.
    ///
    /// A position naming two lines, or columns its line does not have, has no
    /// slice to answer with at all.
    ///
    /// A caller that answers with the literal instead is measuring what the
    /// rules measured before any of this: offsets counted off a string the
    /// escapes are already out of, added to the column comrak reported. The one
    /// difference is that the column is put back on the line once, for the node,
    /// rather than once for each column reported out of it, so a `\|` written
    /// between the start of the node and the offset is a column that stays
    /// missing. Both are wrong about that offset either way, and neither is a
    /// line this can read.
    fn line_text<'t>(&'t self, position: Sourcepos, literal: &str) -> Option<(&'t str, usize)> {
        if position.start.line != position.end.line {
            return None;
        }

        let written = self.written_position(position);

        // A line and a column are counted from one, and an index from zero, so
        // a position that starts at either's zero indexes nothing at all.
        let index = written.start.line.checked_sub(1)?;
        let start = written.start.column.checked_sub(1)?;
        let text = self
            .lines
            .get(index)
            .and_then(|line| line.get(start..written.end.column))?;

        Self::is_source_of(text, literal).then_some((text, written.start.column))
    }

    /// `written` with the escapes in it masked out, a byte for a byte.
    ///
    /// The byte the escape guards goes with the backslash: a marker is what
    /// there is to hide, and it is the guarded byte that is one.
    fn without_escapes(written: &str) -> Cow<'_, str> {
        // Nothing to take out, and nothing to allocate for. Most text is this.
        if !written.contains('\\') {
            return Cow::Borrowed(written);
        }

        let mut masked = String::with_capacity(written.len());
        let mut chars = written.chars().peekable();

        while let Some(char) = chars.next() {
            match chars.peek() {
                Some(&next) if char == '\\' && next.is_ascii_punctuation() => {
                    chars.next();

                    // Two bytes for the two the escape was written with, so an
                    // offset past this one is still the column it was at.
                    masked.push_str("xx");
                }
                _ => masked.push(char),
            }
        }

        Cow::Owned(masked)
    }

    /// Whether `CommonMark` built `literal` out of `written`.
    ///
    /// The escapes are the difference between the two that this knows about:
    /// the backslash of a `\<punctuation>` is not in the literal, and the byte
    /// it guards stands there for the pair. Everything else has to be equal,
    /// byte for byte and to the same length, so text from some other part of
    /// the document is not taken for this node's.
    ///
    /// A backslash that is itself escaped does not start an escape, and does
    /// not need saying so: `\\|` is walked as `\\` and then `|`, which is the
    /// pair `CommonMark` resolves it to.
    fn is_source_of(written: &str, literal: &str) -> bool {
        // Resolving an escape takes a byte off, and nothing `CommonMark` does
        // to a text node puts one back, so two of the same length had no escape
        // between them and have to be the same bytes. That is nearly every node
        // in a document, and comparing the two whole is cheaper than walking
        // them a byte at a time.
        if written.len() == literal.len() {
            return written == literal;
        }

        let mut literal = literal.bytes();
        let mut written = written.bytes().peekable();

        while let Some(byte) = written.next() {
            // The backslash of an escape is not in the literal, where the byte
            // it guards stands for the pair.
            let byte = match written.peek() {
                Some(&next) if byte == b'\\' && next.is_ascii_punctuation() => {
                    written.next();
                    next
                }
                _ => byte,
            };

            if literal.next() != Some(byte) {
                return false;
            }
        }

        literal.next().is_none()
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

    // The offset is walked along the line, where the escapes still are, so the
    // column comes out one to the right of the literal's own for each one the
    // walk passed. An offset that lands on an escape answers with the backslash
    // it begins at, which is where the character was written.
    #[test]
    fn written_column_of_a_line() -> Result<()> {
        let text = r"x \. y z".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((1, 1, 1, 8));
        assert_eq!(doc.written_column_of(position, "x . y z", 0), 1);
        assert_eq!(doc.written_column_of(position, "x . y z", 2), 3);
        assert_eq!(doc.written_column_of(position, "x . y z", 6), 8);
        Ok(())
    }

    // comrak measures a position from inside a table cell against the unescaped
    // cell, so the columns are put back on the line before the walk starts —
    // and the walk takes it from there, the cell's `\|` being an escape like
    // any other on the line.
    #[test]
    fn written_column_of_a_table_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y w | c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((3, 3, 3, 7));
        assert_eq!(doc.written_column_of(position, "x|y w", 0), 3);
        assert_eq!(doc.written_column_of(position, "x|y w", 1), 4);
        assert_eq!(doc.written_column_of(position, "x|y w", 4), 8);

        // The byte after the node, which is where a caller naming the byte
        // after a span lands.
        assert_eq!(doc.written_column_of(position, "x|y w", 5), 9);
        Ok(())
    }

    // `\\|` is an escaped backslash and then a pipe, and the walk takes the
    // run as `CommonMark` resolves it: two bytes for the one, and the pipe on
    // its own.
    #[test]
    fn written_column_of_an_escaped_backslash() -> Result<()> {
        let text = r"a \\| b".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let position = Sourcepos::from((1, 1, 1, 7));
        assert_eq!(doc.written_column_of(position, r"a \| b", 3), 5);
        assert_eq!(doc.written_column_of(position, r"a \| b", 5), 7);
        Ok(())
    }

    // A position that names two lines describes no slice of either, one
    // reaching past the end of its line describes none of it, and one starting
    // at a line or a column of zero indexes nothing at all. None of them is a
    // line to walk, so the offset is added to the column comrak reported.
    #[test]
    fn written_column_of_no_line() -> Result<()> {
        let text = "x y\nz w".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(
            doc.written_column_of(Sourcepos::from((1, 1, 2, 3)), "x y z w", 2),
            3
        );
        assert_eq!(
            doc.written_column_of(Sourcepos::from((1, 1, 1, 4)), "x y", 2),
            3
        );
        assert_eq!(
            doc.written_column_of(Sourcepos::from((3, 1, 3, 1)), "v", 0),
            1
        );
        assert_eq!(
            doc.written_column_of(Sourcepos::from((0, 0, 0, 0)), "x y", 1),
            1
        );
        Ok(())
    }

    // The line a position names is not always the one the literal was built
    // from: comrak measures the inlines after one that spans two lines from a
    // line behind, and the slice is then some other text of the document
    // entirely. Reading it back as the literal's source is what tells the two
    // apart, and the column comrak reported answers where it is not.
    #[test]
    fn written_column_of_a_line_that_is_not_the_source() -> Result<()> {
        let text = indoc! {"
            a [b](
            https://www.example.com/) c
            will be used.
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(
            doc.written_column_of(Sourcepos::from((2, 1, 2, 13)), "will be used.", 5),
            6
        );
        Ok(())
    }

    // A character reference is resolved into the literal the way an escape is,
    // and is not one this can put back, so the line does not read back as the
    // source and the column comrak reported answers — which is what the rules
    // counted off before any of this, and is short by the reference.
    #[test]
    fn written_column_of_a_character_reference() -> Result<()> {
        let text = "a &amp; b".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(
            doc.written_column_of(Sourcepos::from((1, 1, 1, 9)), "a & b", 4),
            5
        );
        Ok(())
    }

    // The escapes are masked a byte for a byte, so a marker the author escaped
    // is not one the search can find and every column past it is still where it
    // was. `\\*` is an escaped backslash and then a marker, and the marker
    // comes back.
    #[test]
    fn written_text_without_escapes_of_a_line() -> Result<()> {
        let text = r"x \* y \\* z".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let actual =
            doc.written_text_without_escapes(Sourcepos::from((1, 1, 1, 12)), r"x * y \* z");

        // `Cow` compares what it holds and not which of the two it is, so the
        // masking is asked for by name as well as by what it produced.
        assert!(matches!(actual.0, Cow::Owned(_)));
        assert_eq!(actual, (Cow::Owned("x xx y xx* z".to_owned()), 1));
        Ok(())
    }

    // A line with no escape on it is masked into nothing, and is answered with
    // as it stands.
    #[test]
    fn written_text_without_escapes_of_a_line_without_one() -> Result<()> {
        let text = "x ** b ** y".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let actual =
            doc.written_text_without_escapes(Sourcepos::from((1, 1, 1, 11)), "x ** b ** y");

        // Borrowed, so the line was answered with as it stands rather than
        // copied to take nothing out of.
        assert!(matches!(actual.0, Cow::Borrowed(_)));
        assert_eq!(actual, (Cow::Borrowed("x ** b ** y"), 1));
        Ok(())
    }

    // The literal has its escapes resolved already, and a backslash still in it
    // was written as an escaped one — `\*` there is a backslash and a marker,
    // and masking it would take a marker the document has out of the search.
    #[test]
    fn written_text_without_escapes_of_a_literal() -> Result<()> {
        let text = r"a &amp; b \\* c".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let actual =
            doc.written_text_without_escapes(Sourcepos::from((1, 1, 1, 15)), r"a & b \* c");

        // Borrowed, and the literal's own: nothing was masked out of it.
        assert!(matches!(actual.0, Cow::Borrowed(_)));
        assert_eq!(actual, (Cow::Borrowed(r"a & b \* c"), 1));
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
