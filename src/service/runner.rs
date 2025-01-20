extern crate alloc;

use alloc::sync::Arc;
use std::sync::{mpsc, Mutex};
use std::thread;

use ignore::WalkParallel;
use miette::miette;
use miette::{IntoDiagnostic as _, Result};

use super::visitor::MarkdownLintVisitorFactory;
use crate::config::Config;
use crate::Diagnostic;

pub struct ParallelLintRunner {
    walker: WalkParallel,
    config: Config,
    capacity: usize,
}

impl ParallelLintRunner {
    #[inline]
    #[must_use]
    pub const fn new(walker: WalkParallel, config: Config, capacity: usize) -> Self {
        Self {
            walker,
            config,
            capacity,
        }
    }

    #[inline]
    // TODO: Don't use expect
    #[expect(clippy::expect_used)]
    #[expect(clippy::unwrap_in_result)]
    pub fn run(self) -> Result<Vec<Diagnostic>> {
        let mutex_diagnostics: Arc<Mutex<Vec<Diagnostic>>> = Arc::new(Mutex::new(vec![]));
        let (tx, rx) = mpsc::sync_channel::<Vec<Diagnostic>>(self.capacity);

        let local_mutex_diagnostics = Arc::clone(&mutex_diagnostics);
        let thread = thread::spawn(move || {
            for diagnostics in rx {
                let mut acquired_diagnostics = local_mutex_diagnostics
                    .lock()
                    .expect("lock must be acquired");
                acquired_diagnostics.extend(diagnostics);
            }
        });

        let mut builder = MarkdownLintVisitorFactory::new(self.config, tx);
        self.walker.visit(&mut builder);

        // Wait for the completion
        drop(builder);
        thread
            .join()
            .map_err(|err| miette!("Failed to join thread. {:?}", err))?;

        // Take ownership of diagnostics
        let lock =
            Arc::into_inner(mutex_diagnostics).ok_or_else(|| miette!("Failed to unwrap Arc"))?;
        lock.into_inner().into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::service::walker::WalkParallelBuilder;

    use super::*;

    #[test]
    fn parallel_lint_runner_run() {
        let mut config = Config::default();
        config.lint.rules = vec![];

        let patterns = [Path::new(".").to_path_buf()];
        let walker = WalkParallelBuilder::build(&patterns).unwrap();
        let runner = ParallelLintRunner::new(walker, config, 0);
        let actual = runner.run().unwrap();
        assert_eq!(actual, vec![]);
    }
}
