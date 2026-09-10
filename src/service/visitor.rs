extern crate alloc;

use alloc::sync::Arc;
use core::result::Result;
use std::collections::HashSet;
use std::path::{Component, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::SyncSender;

use comrak::Arena;
use globset::GlobSet;
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};
use miette::IntoDiagnostic as _;

use super::Linter;
use crate::{Document, Violation, config::Config};

pub struct MarkdownLintVisitor {
    linter: Linter,
    exclusion: GlobSet,
    tx: SyncSender<Vec<Violation>>,
    said: Option<Arc<Mutex<HashSet<String>>>>,
}

impl MarkdownLintVisitor {
    #[inline]
    #[must_use]
    pub const fn new(linter: Linter, exclusion: GlobSet, tx: SyncSender<Vec<Violation>>) -> Self {
        Self {
            linter,
            exclusion,
            tx,
            said: None,
        }
    }

    /// This visitor, sharing `said` as the note of what the walks have said
    /// about themselves. A tree is walked a group at a time, and an ignore
    /// file several groups read has as much to say to each of them.
    #[inline]
    #[must_use]
    pub fn saying_each_thing_once(mut self, said: Arc<Mutex<HashSet<String>>>) -> Self {
        self.said = Some(said);
        self
    }

    /// Whether `message` is yet to be said. A note that cannot be read leaves
    /// it said again rather than unsaid: saying a thing twice is the smaller
    /// fault of the two.
    fn unsaid(&self, message: &str) -> bool {
        self.said.as_ref().is_none_or(|said| {
            said.lock()
                .map_or(true, |mut said| said.insert(message.to_owned()))
        })
    }

    /// Say what the walk has to say about itself, unless it has been said
    /// already. What a file has to say is its own and is said either way.
    fn say_once(&self, message: &str) {
        if self.unsaid(message) {
            eprintln!("{message}");
        }
    }

    fn visit_inner(&self, entry: &DirEntry) -> miette::Result<()> {
        let path = entry.path();
        if path.is_file() && path.extension() == Some("md".as_ref()) {
            // Strip a leading "./" so that exclude patterns match regardless of
            // whether the walked path carries one (depends on how the target
            // argument was spelled on the command line, see issue #168). Keep
            // this in sync with the pattern normalization in
            // Lint::exclude_set (src/config/lint.rs) or the two sides stop
            // agreeing on what a match is.
            let normalized_path: PathBuf = path
                .components()
                .skip_while(|component| matches!(component, Component::CurDir))
                .collect();

            if !self.exclusion.is_match(&normalized_path) {
                let arena = Arena::new();
                let doc = Document::open(&arena, path)?;
                let violations = self.linter.check(&doc)?;
                if !violations.is_empty() {
                    self.tx.send(violations).into_diagnostic()?;
                }
            }
        }

        Ok(())
    }
}

impl ParallelVisitor for MarkdownLintVisitor {
    #[inline]
    fn visit(&mut self, either_entry: Result<DirEntry, Error>) -> WalkState {
        // TODO: Handle errors
        match either_entry {
            // The walk speaks here of the files it read to walk by, which are
            // one file's worth of trouble however many walks read them.
            Err(err) => self.say_once(&err.to_string()),
            Ok(entry) => {
                if let Err(err) = self.visit_inner(&entry) {
                    eprintln!("{err}");
                }
            }
        }

        WalkState::Continue
    }
}

#[derive(Clone)]
pub struct MarkdownLintVisitorFactory {
    config: Config,
    exclusion: GlobSet,
    tx: SyncSender<Vec<Violation>>,
    said: Arc<Mutex<HashSet<String>>>,
}

impl MarkdownLintVisitorFactory {
    /// A factory whose visitors keep their own note of what has been said. A
    /// clone of it keeps that note rather than starting one.
    #[inline]
    pub fn new(config: Config, tx: SyncSender<Vec<Violation>>) -> miette::Result<Self> {
        let exclusion = config.lint.exclude_set()?;
        Ok(Self {
            config,
            exclusion,
            tx,
            said: Arc::new(Mutex::new(HashSet::new())),
        })
    }
}

impl<'s> ParallelVisitorBuilder<'s> for MarkdownLintVisitorFactory {
    #[inline]
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        let linter = Linter::from(&self.config);
        Box::new(
            MarkdownLintVisitor::new(linter, self.exclusion.clone(), self.tx.clone())
                .saying_each_thing_once(Arc::clone(&self.said)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;

    use ignore::Walk;

    use super::*;

    #[test]
    fn markdown_lint_visitor_visit_inner() -> miette::Result<()> {
        let (tx, rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let linter = Linter::new(vec![]);
        let exclusion = GlobSet::empty();
        let visitor = MarkdownLintVisitor::new(linter, exclusion, tx);

        for entry in Walk::new(".").flatten() {
            visitor.visit_inner(&entry)?;
        }

        drop(visitor);
        assert!(rx.recv().is_err()); // Because rx has not received any messages
        Ok(())
    }

    #[test]
    fn markdown_lint_visitor_says_each_thing_once() {
        let said = Arc::new(Mutex::new(HashSet::new()));
        let (tx, _rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let visitor = MarkdownLintVisitor::new(Linter::new(vec![]), GlobSet::empty(), tx)
            .saying_each_thing_once(Arc::clone(&said));

        assert!(visitor.unsaid("one thing"));
        assert!(!visitor.unsaid("one thing"));
        assert!(visitor.unsaid("another thing"));
    }

    #[test]
    fn markdown_lint_visitor_says_what_it_cannot_take_a_note_of() {
        let said = Arc::new(Mutex::new(HashSet::new()));
        let (tx, _rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let visitor = MarkdownLintVisitor::new(Linter::new(vec![]), GlobSet::empty(), tx)
            .saying_each_thing_once(Arc::clone(&said));

        let poisoning = Arc::clone(&said);
        drop(
            thread::spawn(move || {
                let _held = poisoning.lock();
                panic!("poison the note");
            })
            .join(),
        );

        // The note is unreadable now, and a thing said twice beats one lost.
        assert!(visitor.unsaid("one thing"));
        assert!(visitor.unsaid("one thing"));
    }

    #[test]
    fn markdown_lint_visitor_without_a_note_says_everything() {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let visitor = MarkdownLintVisitor::new(Linter::new(vec![]), GlobSet::empty(), tx);

        assert!(visitor.unsaid("one thing"));
        assert!(visitor.unsaid("one thing"));
    }

    #[test]
    fn markdown_lint_visitor_factory_clone_keeps_the_note() -> miette::Result<()> {
        let (tx, _rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let first = MarkdownLintVisitorFactory::new(Config::default(), tx)?;
        let second = first.clone();

        assert!(Arc::ptr_eq(&first.said, &second.said));
        Ok(())
    }

    #[test]
    fn markdown_lint_visitor_factory_build() -> miette::Result<()> {
        let mut config = Config::default();
        config.lint.rules = vec![];

        let (tx, rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let mut factory = MarkdownLintVisitorFactory::new(config, tx)?;
        let mut visitor = factory.build();

        for entry in Walk::new(".") {
            visitor.visit(entry);
        }

        drop(visitor);
        drop(factory);
        assert!(rx.recv().is_err()); // Because rx has not received any messages
        Ok(())
    }
}
