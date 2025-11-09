extern crate alloc;

use alloc::sync::Arc;
use comrak::Arena;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;

use ignore::WalkParallel;
use miette::miette;
use miette::{IntoDiagnostic as _, Result};

use super::visitor::MarkdownLintVisitorFactory;
use super::walker::WalkParallelBuilder;
use super::Linter;
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
            Self::Parallel(runner) => (*runner).run(),
            Self::String(runner) => runner.run(),
        }
    }
}

pub struct ParallelLintRunner {
    walker: WalkParallel,
    config: Config,
    capacity: usize,
}

impl ParallelLintRunner {
    #[inline]
    pub fn new(patterns: &[PathBuf], config: Config, capacity: usize) -> Result<Self> {
        let walker = WalkParallelBuilder::build(
            patterns,
            config.lint.respect_ignore,
            config.lint.respect_gitignore,
        )?;

        Ok(Self {
            walker,
            config,
            capacity,
        })
    }

    #[inline]
    // TODO: Don't use expect
    #[expect(clippy::expect_used)]
    #[expect(clippy::unwrap_in_result)]
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

        let mut builder = MarkdownLintVisitorFactory::new(self.config, tx)?;
        self.walker.visit(&mut builder);

        // Wait for the completion
        drop(builder);
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
        let linter = Linter::from_config(&self.config);
        linter.check(&doc)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use super::*;

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
