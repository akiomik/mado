extern crate alloc;

use alloc::sync::Arc;
use comrak::Arena;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::thread;

use ignore::WalkParallel;
use miette::miette;
use miette::{IntoDiagnostic as _, Result};

use super::Linter;
use super::visitor::MarkdownLintVisitorFactory;
use super::walker::{WalkParallelBuilder, thread_budget, walks_at_once};
use crate::config::Config;
use crate::{Document, Violation};

#[non_exhaustive]
pub enum LintRunner {
    Parallel(Box<ParallelLintRunner>),
    String(Box<StringLintRunner>),
}

impl LintRunner {
    #[inline]
    pub fn run(self) -> Result<Vec<Violation>> {
        match self {
            Self::Parallel(runner) => runner.run(),
            Self::String(runner) => runner.run(),
        }
    }
}

pub struct ParallelLintRunner {
    walkers: Vec<WalkParallel>,
    config: Config,
    capacity: usize,
}

impl ParallelLintRunner {
    #[inline]
    pub fn new(patterns: &[PathBuf], config: Config, capacity: usize) -> Result<Self> {
        let walkers = WalkParallelBuilder::build(
            patterns,
            config.lint.respect_ignore,
            config.lint.respect_gitignore,
        )?;

        Ok(Self {
            walkers,
            config,
            capacity,
        })
    }

    #[inline]
    // TODO: Don't use expect
    #[expect(clippy::expect_used)]
    pub fn run(self) -> Result<Vec<Violation>> {
        let mutex_violations: Arc<Mutex<Vec<Violation>>> = Arc::new(Mutex::new(vec![]));
        let (tx, rx) = mpsc::sync_channel::<Vec<Violation>>(self.capacity);

        let local_mutex_violations = Arc::clone(&mutex_violations);
        let thread = thread::spawn(move || {
            for violations in rx {
                let mut acquired_violations = local_mutex_violations
                    .lock()
                    .expect("lock must be acquired");
                acquired_violations.extend(violations);
            }
        });

        // Each walk holds its visitor while it runs, so a batch gets one
        // visitor apiece and the batches run one after another. At least one
        // walk per batch: a batch of none would loop without ever moving on,
        // and a walk with no visitor would not be walked at all.
        let at_once = walks_at_once(thread_budget(), self.walkers.len()).max(1);
        let mut remaining = self.walkers;
        // A clone of the factory keeps the note of what has been said, so a
        // broken ignore file several groups read is reported once for the run.
        let first = MarkdownLintVisitorFactory::new(self.config, tx.clone())?;
        let alongside = at_once.min(remaining.len()).saturating_sub(1);
        let mut builders = vec![first.clone(); alongside];
        builders.push(first);
        while !remaining.is_empty() {
            let rest = remaining.split_off(remaining.len().min(at_once));
            thread::scope(|scope| {
                for (walker, builder) in remaining.into_iter().zip(builders.iter_mut()) {
                    scope.spawn(move || walker.visit(builder));
                }
            });
            remaining = rest;
        }

        // Wait for the completion
        drop(builders);
        drop(tx);
        thread
            .join()
            .map_err(|err| miette!("Failed to join thread. {:?}", err))?;

        // Take ownership of violations
        let lock =
            Arc::into_inner(mutex_violations).ok_or_else(|| miette!("Failed to unwrap Arc"))?;
        lock.into_inner().into_diagnostic()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLintRunner {
    string: String,
    config: Config,
}

impl StringLintRunner {
    #[inline]
    #[must_use]
    pub const fn new(string: String, config: Config) -> Self {
        Self { string, config }
    }

    #[inline]
    pub fn run(self) -> Result<Vec<Violation>> {
        let arena = Arena::new();
        let path = Path::new("(stdin)").to_path_buf();
        let doc = Document::new(&arena, path, self.string)?;
        let linter = Linter::from(&self.config);
        linter.check(&doc)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn parallel_lint_runner_new_empty_patterns() {
        let result = ParallelLintRunner::new(&[], Config::default(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn parallel_lint_runner_run_several_groups() -> Result<()> {
        let mut config = Config::default();
        config.lint.rules = vec![];

        // Nothing is read for a file and something is for a directory, so the
        // two are walked as two groups.
        let patterns = [
            Path::new("README.md").to_path_buf(),
            Path::new("src").to_path_buf(),
        ];
        let runner = ParallelLintRunner::new(&patterns, config, 0)?;
        let actual = runner.run()?;
        assert_eq!(actual, vec![]);
        Ok(())
    }

    #[test]
    fn parallel_lint_runner_run() -> Result<()> {
        let mut config = Config::default();
        config.lint.rules = vec![];

        let patterns = [Path::new(".").to_path_buf()];
        let runner = ParallelLintRunner::new(&patterns, config, 0)?;
        let actual = runner.run()?;
        assert_eq!(actual, vec![]);
        Ok(())
    }
}
