extern crate alloc;

use alloc::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue, Sourcepos};
use core::cell::OnceCell;
use core::fmt;

use comrak::{Arena, Options, parse_document};
use miette::IntoDiagnostic as _;
use miette::Result;
use rustc_hash::FxHashMap;

#[derive(Clone)]
#[non_exhaustive]
pub struct Document<'a> {
    pub path: PathBuf,
    pub ast: &'a AstNode<'a>,

    /// The same document parsed with GFM's autolink extension on, which MD034
    /// walks and no other rule does. Taken on the first ask and not before:
    /// every other rule reads `ast`, so a run without MD034 in it never parses
    /// twice. [`Document::autolink_ast`] is the ask.
    ///
    /// What MD034 reports is a URL a reader is handed a link to without anyone
    /// having written a link around it, and the autolink extension is what
    /// decides which text that is: `http\://example.com` and
    /// `http://localhost/x` are both URLs to a scanner and neither is linked,
    /// while `http://ex\-ample.com/` is linked with the backslash still in it.
    /// Asking comrak is asking the parser the rule reports on.
    ///
    /// It is a tree of its own because the extension does not only add links.
    /// A bare URL takes the text around it with it, splitting the text node it
    /// was written in into as many as three, and a rule that reads a text node
    /// whole reads a different document for it: MD036 counts a paragraph's
    /// emphasis once per text node in it, MD037 matches an emphasis pair inside
    /// one, and MD020 asks whether a heading's last one ends in a `#`. None of
    /// them is asking about links, and each of them is wrong about a document
    /// that has one.
    ///
    /// This is [`Document::ast`] itself where the two would be the same tree,
    /// which is most documents: a URL is usually written as a link's
    /// destination, and no autolink can be made out of one.
    autolink_ast: OnceCell<&'a AstNode<'a>>,

    /// What the second parse takes, kept for as long as it might be asked for.
    /// The arena is the one `ast` was built in, so both trees live as long as
    /// the document does.
    arena: &'a Arena<'a>,
    options: Options<'static>,

    pub text: String,
    pub lines: Vec<String>,

    /// The regions comrak unescaped, by the line they were written on. See
    /// [`Document::written_position`].
    unescaped_regions: FxHashMap<usize, Vec<UnescapedRegion>>,
}

// Written out because the arena cannot be derived through: it is comrak's, and
// `typed_arena::Arena` has no `Debug`. Every field a caller could want is here,
// and the arena is what `finish_non_exhaustive` stands for.
impl fmt::Debug for Document<'_> {
    #[inline]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Document")
            .field("path", &self.path)
            .field("ast", &self.ast)
            .field("autolink_ast", &self.autolink_ast)
            .field("options", &self.options)
            .field("text", &self.text)
            .field("lines", &self.lines)
            .field("unescaped_regions", &self.unescaped_regions)
            .finish_non_exhaustive()
    }
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
            autolink_ast: OnceCell::new(),
            arena,
            options,
            text,
            lines,
            unescaped_regions,
        })
    }

    /// The document parsed with GFM's autolink extension on, for MD034.
    ///
    /// Taken on the first ask, because MD034 is the only rule that asks and a
    /// run it is not in should not pay for it. See the field of the same name.
    #[inline]
    #[must_use]
    pub fn autolink_ast(&self) -> &'a AstNode<'a> {
        self.autolink_ast.get_or_init(|| {
            Self::parse_with_autolink(self.arena, &self.text, &self.options, self.ast)
        })
    }

    /// [`Document::autolink_ast`], parsed only where it would differ from
    /// `ast`.
    ///
    /// A document the extension can find no autolink in parses to the same tree
    /// either way, and `ast` is handed back rather than parsed a second time.
    /// comrak begins an autolink at a `://`, a `www.` or an `@`, so the
    /// question is whether the document has one of those somewhere the inline
    /// parser would read it as text — and `ast` can be asked, because up to the
    /// first autolink the two parses are the same parse.
    ///
    /// They are the same parse because the extension adds nothing but a branch
    /// at those three, and takes `:`, `w` and `@` for special bytes to reach
    /// it. Without it they are ordinary text, so the marker that begins the
    /// first autolink of the extended parse is text of one node in `ast`, whole
    /// — the two cannot part company before it, and a node cannot be split
    /// where no construct begins. That is what makes the text nodes of `ast`
    /// the right place to look, and it is a narrower place than the document:
    /// a URL written as a link's destination, inside a code span or inside a
    /// raw HTML tag is not text by the time the parser is reading it, and a
    /// destination is where a URL is usually written.
    ///
    /// A text node inside a link is not passed over, tempting as it is: comrak
    /// refuses an autolink while it is inside brackets, but it counts its way
    /// out of them on any `]` at all, so the `http://x.example.com/` of
    /// `[a [b] http://x.example.com/](y)` is inside link text and autolinked
    /// both. Skipping those made a URL's report depend on whether the document
    /// had another one somewhere else, which is the shape of bug that is worst
    /// to have: not a wrong report, but a right one that is not made.
    ///
    /// Coarse on the `@`, deliberately. comrak asks an email address for a
    /// period after the `@` as well, so the guard could ask for one too and
    /// stay sound — and it would buy nothing. Of the 1522 documents in the
    /// gitlab benchmark corpus, 1000 take the second parse, 13 of those on the
    /// `@` alone, and every one of the 13 has a period somewhere after it:
    /// prose ends its sentences. The parse it would save is one nobody writes.
    ///
    /// `autolink_ast_is_ast_only_when_the_trees_agree` is the test that this
    /// reasoning is comrak's behaviour and not just an account of it.
    fn parse_with_autolink(
        arena: &'a Arena<'a>,
        text: &str,
        options: &Options,
        ast: &'a AstNode<'a>,
    ) -> &'a AstNode<'a> {
        let possible = ast.descendants().any(|node| {
            let NodeValue::Text(literal) = &node.data.borrow().value else {
                return false;
            };

            literal.contains("://") || literal.contains("www.") || literal.contains('@')
        });

        if !possible {
            return ast;
        }

        // Cloned rather than taken by `&mut`, which would leave the extension
        // on in the caller's own options for whatever it parses next.
        let mut options = options.clone();
        options.extension.autolink = true;
        parse_document(arena, text, &options)
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
    /// escapes are already out of, added to the column comrak reported. That
    /// column is put back on the line once, for the node, rather than once for
    /// each column reported out of it, so a `\|` written between the start of
    /// the node and the offset is a column that stays missing. It is the wrong
    /// answer for that offset, and it is the only one left where this cannot
    /// read the line.
    fn line_text<'t>(&'t self, position: Sourcepos, literal: &str) -> Option<(&'t str, usize)> {
        if position.start.line != position.end.line {
            return None;
        }

        let written = self.written_position(position);

        // A line and a column are counted from one, and an index from zero, so
        // a position that starts at either's zero indexes nothing at all.
        let index = written.start.line.checked_sub(1)?;
        let mut start = written.start.column.checked_sub(1)?;
        let line = self.lines.get(index)?;

        // comrak measures a node from the byte its literal begins with, and a
        // byte written escaped is a column further along than the escape that
        // wrote it. The backslash is the node's own — nothing else can end on
        // one, an inline ending in a backtick, a bracket, a marker or a `>` —
        // and without it the slice is the literal's twin rather than its
        // source: the escapes in the rest of it are read one byte early, and
        // the escape at the start is not there to be read at all.
        if start > 0 && line.as_bytes().get(start - 1) == Some(&b'\\') {
            start -= 1;
        }

        let text = line.get(start..written.end.column)?;

        Self::is_source_of(text, literal).then_some((text, start + 1))
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
    use core::ptr;

    use comrak::format_html;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    // The arena has no `Debug` and the derive could not reach through it, so
    // this one is written out and is code like any other.
    #[test]
    fn debug_names_the_document() -> Result<()> {
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, "x http://www.example.com/ y".to_owned())?;
        let debug = format!("{doc:?}");
        assert!(debug.starts_with("Document {"), "{debug}");
        assert!(debug.contains("test.md"), "{debug}");

        // The arena is what the `..` stands for.
        assert!(debug.ends_with(".. }"), "{debug}");
        Ok(())
    }

    #[test]
    fn open() {
        let arena = Arena::new();
        let path = Path::new("README.md");
        assert!(Document::open(&arena, path).is_ok());
    }

    // `Document::parse_with_autolink` hands `ast` back for a document it reads
    // as one the extension can find no autolink in, and MD034 walks whatever it
    // hands back — so a document it is wrong about is one MD034 reports nothing
    // for and says nothing about. What makes it right is an argument about
    // comrak's inline parser rather than anything checked at the time, and this
    // is where that argument is checked against comrak.
    //
    // The two parses are rendered and compared, a renderer being what says
    // which text the extension made a link of. A document they differ on is one
    // the second tree was owed, and handing `ast` back for it is the failure
    // that has no symptom. The other direction is cost rather than correctness,
    // so it is recorded per input instead of asserted over all of them: the
    // second column is whether the document was its own answer, and the two
    // marked `false` against an unchanged rendering are what the guard is
    // deliberately coarse about.
    //
    // The inputs are the shapes the argument turns on. A marker the parser
    // never reads as text — a destination, a code span, a raw HTML tag, an
    // indented or fenced block — is one no autolink can be made of, and those
    // are the documents that are their own answer. A marker in text is owed the
    // second parse wherever it is written, link text included: comrak refuses
    // an autolink inside brackets but counts its way out of them on any `]`,
    // which the three nested rows are here for. And the markers that only look
    // like markers are owed nothing.
    //
    // `WWW.EXAMPLE.COM` is among those, and is the row that guards the guard.
    // `literal.contains("www.")` is written in lower case, which is right only
    // for as long as comrak matches `www.` in lower case — cmark-gfm compares
    // it with `memcmp` and comrak with `starts_with`, so the two agree today.
    // Were comrak to stop, the extended parse would link this and the plain one
    // would not, while the guard carried on calling the document its own
    // answer: which is the assertion below, and it would fail. The scheme has
    // no such row to spare, `://` having no letters in it for a case to differ
    // in; `HTTP://WWW.EXAMPLE.COM/` is here to be parsed twice and linked by
    // neither, and #420 is what fails when comrak closes that one.
    #[test]
    fn autolink_ast_is_ast_only_when_the_trees_agree() -> Result<()> {
        let texts = [
            ("see http://www.example.com/ now", false),
            ("see <http://www.example.com/> now", false),
            ("see [x](http://www.example.com/) now", true),
            ("see [http://www.example.com/](y) now", false),
            ("see [a [b] http://www.example.com/](y) now", false),
            ("see [[a] http://www.example.com/](y) now", false),
            ("see [a ![b](i.png) http://www.example.com/](y) now", false),
            ("see ![http://www.example.com/](y.png) now", false),
            ("see [http://www.example.com/] now", false),
            ("see [x] now\n\n[x]: http://www.example.com/", true),
            ("see `http://www.example.com/` now", true),
            ("    http://www.example.com/", true),
            ("```\nhttp://www.example.com/\n```", true),
            ("see <a href=\"http://www.example.com/\">x</a> now", true),
            ("see <div>http://www.example.com/</div> now", false),
            ("see www.example.com now", false),
            ("see foo@example.com now", false),
            ("see mailto:foo@example.com now", false),
            ("see xmpp:foo@example.com/bar now", false),
            ("see <foo@example.com> now", false),
            ("see [foo@example.com](y) now", false),
            (r"see http\://www.example.com/ now", false),
            ("see http://localhost/x now", false),
            ("see http://localhost:3000/admin now", false),
            (r"see http://ex\_ample.com/ now", false),
            (r"see http://ex\-ample.com/ now", false),
            ("see http:// now", false),
            ("see wwwexample now", true),
            ("see WWW.EXAMPLE.COM now", true),
            ("see HTTP://WWW.EXAMPLE.COM/ now", false),
            ("see a@ now", false),
            ("see nothing at all now", true),
            (
                "| a | b |\n| --- | --- |\n| http://www.example.com/ | c |",
                false,
            ),
            ("> http://www.example.com/", false),
            ("- http://www.example.com/", false),
            ("# http://www.example.com/", false),
            ("*http://www.example.com/*", false),
            ("see [x](y) and http://www.example.com/ now", false),
            (
                "see <http://a.example.com/> and http://b.example.com/ now",
                false,
            ),
        ];

        for (text, own_answer) in texts {
            let arena = Arena::new();
            let path = Path::new("test.md").to_path_buf();
            let doc = Document::new(&arena, path, text.to_owned())?;

            let mut options = Options::default();
            options.extension.front_matter_delimiter = Some("---".to_owned());
            options.extension.table = true;
            let mut plain = String::new();
            format_html(doc.ast, &options, &mut plain).into_diagnostic()?;

            options.extension.autolink = true;
            let reference_arena = Arena::new();
            let reference = parse_document(&reference_arena, text, &options);
            let mut extended = String::new();
            format_html(reference, &options, &mut extended).into_diagnostic()?;

            assert_eq!(ptr::eq(doc.ast, doc.autolink_ast()), own_answer, "{text:?}");

            // The half that is correctness: what was handed back as its own
            // answer has to be a document the extension changes nothing about.
            if own_answer {
                assert_eq!(plain, extended, "{text:?}");
            }
        }

        Ok(())
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

    // comrak measures a position from inside a table cell against the unescaped
    // cell, so the columns are put back on the line before the slice is taken.
    #[test]
    fn written_text_without_escapes_of_a_table_cell() -> Result<()> {
        let text = indoc! {r"
            | a | b |
            | --- | --- |
            | x\|y w | c |
        "}
        .to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        let actual = doc.written_text_without_escapes(Sourcepos::from((3, 3, 3, 7)), "x|y w");
        assert_eq!(actual, (Cow::Owned("xxxy w".to_owned()), 3));
        Ok(())
    }

    // comrak measures a node from the byte its literal begins with, and a byte
    // written escaped is a column further along than the escape that wrote it.
    // Without the backslash the slice is the literal's twin rather than its
    // source — here it is that byte for byte — and the escape at the start is
    // not there to be masked at all.
    #[test]
    fn written_text_without_escapes_of_a_line_beginning_with_an_escape() -> Result<()> {
        let text = r"\*x y".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;

        // comrak reports 1:2 for a node beginning at the escaped marker, which
        // is written at columns 1 and 2.
        let actual = doc.written_text_without_escapes(Sourcepos::from((1, 2, 1, 5)), "*x y");
        assert_eq!(actual, (Cow::Owned("xxx y".to_owned()), 1));
        Ok(())
    }

    // A position that names two lines describes no slice of either, one
    // reaching past the end of its line describes none of it, and one starting
    // at a line or a column of zero indexes nothing at all. None of them is a
    // line to read, so the literal answers for itself.
    #[test]
    fn written_text_without_escapes_of_no_line() -> Result<()> {
        let text = "x y\nz w".to_owned();
        let arena = Arena::new();
        let path = Path::new("test.md").to_path_buf();
        let doc = Document::new(&arena, path, text)?;
        assert_eq!(
            doc.written_text_without_escapes(Sourcepos::from((1, 1, 2, 3)), "x y z w"),
            (Cow::Borrowed("x y z w"), 1)
        );
        assert_eq!(
            doc.written_text_without_escapes(Sourcepos::from((1, 1, 1, 4)), "x y"),
            (Cow::Borrowed("x y"), 1)
        );
        assert_eq!(
            doc.written_text_without_escapes(Sourcepos::from((3, 1, 3, 1)), "v"),
            (Cow::Borrowed("v"), 1)
        );
        assert_eq!(
            doc.written_text_without_escapes(Sourcepos::from((0, 0, 0, 0)), "x y"),
            (Cow::Borrowed("x y"), 0)
        );
        Ok(())
    }

    // The line a position names is not always the one the literal was built
    // from: comrak measures the inlines after one that spans two lines from a
    // line behind, and the slice is then some other text of the document
    // entirely. Reading it back as the literal's source is what tells the two
    // apart, and the literal answers where it is not.
    #[test]
    fn written_text_without_escapes_of_a_line_that_is_not_the_source() -> Result<()> {
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
            doc.written_text_without_escapes(Sourcepos::from((2, 1, 2, 13)), "will be used."),
            (Cow::Borrowed("will be used."), 1)
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
