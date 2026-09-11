use std::path::PathBuf;

use ignore::WalkBuilder;
use ignore::WalkParallel;
use ignore::types::TypesBuilder;
use miette::IntoDiagnostic as _;
use miette::Result;
use miette::miette;

use crate::config::GitignorePolicy;

#[non_exhaustive]
pub struct WalkParallelBuilder;

impl WalkParallelBuilder {
    #[inline]
    pub fn build(
        patterns: &[PathBuf],
        respect_ignore: bool,
        respect_gitignore: GitignorePolicy,
    ) -> Result<WalkParallel> {
        let (head_pattern, tail_patterns) = patterns
            .split_first()
            .ok_or_else(|| miette!("files must be non-empty"))?;
        let mut builder = WalkBuilder::new(head_pattern);
        for pattern in tail_patterns {
            builder.add(pattern);
        }

        builder.ignore(respect_ignore);
        builder.git_ignore(respect_gitignore != GitignorePolicy::Never);
        // `require_git` decides whether a repository has to be there for a
        // `.gitignore` to apply at all. It also decides whether a repository
        // marker bounds the search upward, which is why `Always` reads
        // `.gitignore` files from every parent directory.
        builder.require_git(respect_gitignore != GitignorePolicy::Always);
        // Neither of these travels with the tree being linted, so reading them
        // makes the same source lint differently on different machines.
        builder.git_global(false);
        builder.git_exclude(false);

        // NOTE: Expect performance improvements with pre-filtering
        let types = TypesBuilder::new()
            .add_defaults()
            .select("markdown")
            .build()
            .into_diagnostic()?;
        builder.types(types);

        Ok(builder.build_parallel())
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use crate::config::GitignorePolicy;
    use alloc::sync::Arc;
    use miette::{Context as _, IntoDiagnostic as _};
    use std::{
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use ignore::{DirEntry, WalkState};
    use pretty_assertions::assert_eq;

    use super::WalkParallelBuilder;

    struct PathCollector {
        paths: Arc<Mutex<Vec<PathBuf>>>,
    }

    impl PathCollector {
        fn new() -> Self {
            Self {
                paths: Arc::new(Mutex::new(vec![])),
            }
        }

        fn gen_visitor(&self) -> impl Fn(Result<DirEntry, ignore::Error>) -> WalkState {
            let paths = Arc::clone(&self.paths);

            move |either_entry: Result<DirEntry, _>| {
                if let Ok(entry) = either_entry
                    && let Ok(mut paths) = paths.lock()
                {
                    paths.push(entry.into_path());
                }

                WalkState::Continue
            }
        }

        fn paths(self) -> miette::Result<Vec<PathBuf>> {
            let mutex = Arc::into_inner(self.paths).wrap_err("failed to get inner ownership")?;
            mutex.into_inner().into_diagnostic()
        }
    }

    #[test]
    fn build_and_run() -> miette::Result<()> {
        let paths = vec![
            Path::new("action").to_path_buf(),
            Path::new("mado.toml").to_path_buf(),
            Path::new("README.md").to_path_buf(),
        ];
        let builder = WalkParallelBuilder::build(&paths, true, GitignorePolicy::default())?;
        let collector = PathCollector::new();

        builder.run(|| Box::new(collector.gen_visitor()));

        let mut actual = collector.paths()?;
        actual.sort();

        let expected = vec![
            Path::new("README.md").to_path_buf(),
            Path::new("action").to_path_buf(),
            Path::new("mado.toml").to_path_buf(),
        ];
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn build_empty_patterns() {
        let result = WalkParallelBuilder::build(&[], true, GitignorePolicy::default());
        assert!(result.is_err());
    }
}
