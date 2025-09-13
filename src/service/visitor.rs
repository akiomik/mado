use core::result::Result;
use std::sync::mpsc::SyncSender;

use comrak::Arena;
use globset::GlobSet;
use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkState};
use miette::IntoDiagnostic as _;

use super::Linter;
use crate::{config::Config, Document, Violation};

pub struct MarkdownLintVisitor {
    linter: Linter,
    exclusion: GlobSet,
    tx: SyncSender<Vec<Violation>>,
}

impl MarkdownLintVisitor {
    #[inline]
    #[must_use]
    pub const fn new(linter: Linter, exclusion: GlobSet, tx: SyncSender<Vec<Violation>>) -> Self {
        Self {
            linter,
            exclusion,
            tx,
        }
    }

    fn visit_inner(&self, either_entry: Result<DirEntry, Error>) -> miette::Result<()> {
        let entry = either_entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_file()
            && path.extension() == Some("md".as_ref())
            && !self.exclusion.is_match(path)
        {
            let arena = Arena::new();
            let doc = Document::open(&arena, path)?;
            let violations = self.linter.check(&doc)?;
            if !violations.is_empty() {
                self.tx.send(violations).into_diagnostic()?;
            }
        }

        Ok(())
    }
}

impl ParallelVisitor for MarkdownLintVisitor {
    #[inline]
    fn visit(&mut self, either_entry: Result<DirEntry, Error>) -> WalkState {
        if let Err(err) = self.visit_inner(either_entry) {
            // TODO: Handle errors
            println!("{err}");
        }
        WalkState::Continue
    }
}

pub struct MarkdownLintVisitorFactory {
    config: Config,
    exclusion: GlobSet,
    tx: SyncSender<Vec<Violation>>,
}

impl MarkdownLintVisitorFactory {
    #[inline]
    pub fn new(config: Config, tx: SyncSender<Vec<Violation>>) -> miette::Result<Self> {
        let exclusion = config.lint.exclude_set()?;
        Ok(Self {
            config,
            exclusion,
            tx,
        })
    }
}

impl<'s> ParallelVisitorBuilder<'s> for MarkdownLintVisitorFactory {
    #[inline]
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        let linter = Linter::from_config(&self.config);
        Box::new(MarkdownLintVisitor::new(
            linter,
            self.exclusion.clone(),
            self.tx.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ignore::Walk;

    use super::*;

    #[test]
    fn markdown_lint_visitor_visit_inner() -> miette::Result<()> {
        let (tx, rx) = mpsc::sync_channel::<Vec<Violation>>(0);
        let linter = Linter::new(vec![]);
        let exclusion = GlobSet::empty();
        let visitor = MarkdownLintVisitor::new(linter, exclusion, tx);

        for entry in Walk::new(".") {
            visitor.visit_inner(entry)?;
        }

        drop(visitor);
        assert!(rx.recv().is_err()); // Because rx has not received any messages
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
