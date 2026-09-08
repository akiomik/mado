use std::env;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use ignore::WalkParallel;
use ignore::types::TypesBuilder;
use miette::IntoDiagnostic as _;
use miette::Result;
use miette::miette;

#[non_exhaustive]
pub struct WalkParallelBuilder;

impl WalkParallelBuilder {
    /// Whether `path` or any directory above it holds a repository marker.
    /// `.git` is a directory in a clone and a file in a worktree or a
    /// submodule, and `.jj` is the other marker the `ignore` crate stops at.
    fn in_repository(path: &Path) -> bool {
        path.canonicalize().is_ok_and(|path| {
            path.ancestors()
                .any(|dir| dir.join(".git").exists() || dir.join(".jj").exists())
        })
    }

    /// The directories above `pattern` up to and including the one mado was
    /// started in, outermost first. Each is named the way `pattern` names it,
    /// so that the ignore files found there match the paths the walk yields.
    /// Empty when `pattern` is that directory, or lies outside it.
    fn ancestors_up_to_current_dir(pattern: &Path) -> Vec<PathBuf> {
        // A directory mado cannot name is one no pattern can be found under.
        let current_dir = env::current_dir()
            .and_then(|dir| dir.canonicalize())
            .unwrap_or_default();
        if pattern.canonicalize().is_ok_and(|path| path == current_dir) {
            return vec![];
        }

        let mut dirs = vec![];
        let mut ancestor = pattern.parent();
        while let Some(dir) = ancestor {
            // `Path::parent` names the directory a relative path sits in with
            // the empty path, which no file can be opened under.
            let named = if dir.as_os_str().is_empty() {
                Path::new(".")
            } else {
                dir
            };
            dirs.push(named.to_path_buf());
            if named.canonicalize().is_ok_and(|path| path == current_dir) {
                dirs.reverse();
                return dirs;
            }
            ancestor = dir.parent();
        }

        vec![]
    }

    /// Adds `path` as an ignore file rooted at `dir`. `WalkBuilder` roots the
    /// files it is handed at whatever `current_dir` was last set to, which is
    /// what lets each directory's patterns keep their own anchoring.
    fn add_ignore_file(builder: &mut WalkBuilder, dir: &Path, name: &str) {
        let path = dir.join(name);
        if path.is_file() {
            builder.current_dir(dir);
            drop(builder.add_ignore(path));
        }
    }

    fn build_one(
        pattern: &Path,
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Result<WalkParallel> {
        let mut builder = WalkBuilder::new(pattern);
        builder.ignore(respect_ignore);
        builder.git_ignore(respect_gitignore);

        // `respect-gitignore` covers `.gitignore` files and nothing else. The
        // global Git ignore file and `.git/info/exclude` do not travel with
        // the tree, so reading them would make the result depend on the
        // machine and on the clone.
        builder.git_global(false);
        builder.git_exclude(false);

        if !Self::in_repository(pattern) {
            // Outside a repository the walker drops `.gitignore` entirely, and
            // `require_git(false)` on its own would read ignore files every
            // directory up to the filesystem root. Stop its parent search and
            // put back the files between here and the directory mado was
            // started in, which is the root the rest of mado resolves against.
            builder.require_git(false);
            builder.parents(false);

            for dir in Self::ancestors_up_to_current_dir(pattern) {
                // Added outermost first, and `.gitignore` before `.ignore`, so
                // that the nearer file wins where both name the same path.
                if respect_gitignore {
                    Self::add_ignore_file(&mut builder, &dir, ".gitignore");
                }
                if respect_ignore {
                    Self::add_ignore_file(&mut builder, &dir, ".ignore");
                }
            }
        }

        // NOTE: Expect performance improvements with pre-filtering
        let types = TypesBuilder::new()
            .add_defaults()
            .select("markdown")
            .build()
            .into_diagnostic()?;
        builder.types(types);

        Ok(builder.build_parallel())
    }

    /// One walker per pattern: the repository a pattern sits in, and so where
    /// its search for ignore files stops, is a property of that pattern alone.
    #[inline]
    pub fn build(
        patterns: &[PathBuf],
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Result<Vec<WalkParallel>> {
        if patterns.is_empty() {
            return Err(miette!("files must be non-empty"));
        }

        patterns
            .iter()
            .map(|pattern| Self::build_one(pattern, respect_ignore, respect_gitignore))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::sync::Arc;
    use miette::{Context as _, IntoDiagnostic as _};
    use std::{
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use ignore::{DirEntry, WalkState};
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

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

    /// Writes `content` to `path`, creating the directories leading to it.
    fn write(path: &Path, content: &str) -> miette::Result<()> {
        let parent = path.parent().wrap_err("path must have a parent")?;
        fs::create_dir_all(parent).into_diagnostic()?;
        fs::write(path, content).into_diagnostic()
    }

    /// Lays out a tree with a `.gitignore` at its root and another one nested
    /// inside it, each ignoring one of the Markdown files beside it.
    fn write_tree(root: &Path) -> miette::Result<()> {
        write(&root.join(".gitignore"), "root-ignored.md\n")?;
        write(&root.join("keep.md"), "# Keep\n")?;
        write(&root.join("root-ignored.md"), "# Root ignored\n")?;
        write(&root.join("nested/.gitignore"), "nested-ignored.md\n")?;
        write(&root.join("nested/keep.md"), "# Nested keep\n")?;
        write(&root.join("nested/nested-ignored.md"), "# Nested ignored\n")
    }

    /// Walks `root` and returns the Markdown files it yields, relative to
    /// `root` and sorted.
    fn walk_markdown(
        root: &Path,
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> miette::Result<Vec<PathBuf>> {
        let patterns = vec![root.to_path_buf()];
        let walkers = WalkParallelBuilder::build(&patterns, respect_ignore, respect_gitignore)?;
        let collector = PathCollector::new();

        for walker in walkers {
            walker.run(|| Box::new(collector.gen_visitor()));
        }

        let mut paths = collector
            .paths()?
            .into_iter()
            .filter(|path| path.extension() == Some(OsStr::new("md")))
            .map(|path| {
                path.strip_prefix(root)
                    .map(Path::to_path_buf)
                    .into_diagnostic()
            })
            .collect::<miette::Result<Vec<_>>>()?;
        paths.sort();
        Ok(paths)
    }

    fn kept() -> Vec<PathBuf> {
        vec![
            Path::new("keep.md").to_path_buf(),
            Path::new("nested/keep.md").to_path_buf(),
        ]
    }

    fn everything() -> Vec<PathBuf> {
        vec![
            Path::new("keep.md").to_path_buf(),
            Path::new("nested/keep.md").to_path_buf(),
            Path::new("nested/nested-ignored.md").to_path_buf(),
            Path::new("root-ignored.md").to_path_buf(),
        ]
    }

    #[test]
    fn build_and_run() -> miette::Result<()> {
        let paths = vec![
            Path::new("action").to_path_buf(),
            Path::new("mado.toml").to_path_buf(),
            Path::new("README.md").to_path_buf(),
        ];
        let walkers = WalkParallelBuilder::build(&paths, true, true)?;
        let collector = PathCollector::new();

        for walker in walkers {
            walker.run(|| Box::new(collector.gen_visitor()));
        }

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
        let result = WalkParallelBuilder::build(&[], true, true);
        assert!(result.is_err());
    }

    #[test]
    fn gitignore_without_git_metadata() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, true)?, kept());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_with_git_directory() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;
        fs::create_dir(tmp_dir.path().join(".git")).into_diagnostic()?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, true)?, kept());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_with_git_file() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;
        let git_file = tmp_dir.path().join(".git");
        write(&git_file, "gitdir: ../.git/worktrees/tree\n")?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, true)?, kept());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_disabled_without_git_metadata() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, false)?, everything());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_disabled_with_git_directory() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;
        fs::create_dir(tmp_dir.path().join(".git")).into_diagnostic()?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, false)?, everything());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn git_info_exclude_is_not_read() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;
        write(&tmp_dir.path().join(".git/info/exclude"), "keep.md\n")?;

        assert_eq!(walk_markdown(tmp_dir.path(), true, true)?, kept());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn ancestor_gitignore_without_git_metadata() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write(&tmp_dir.path().join(".gitignore"), "ancestor-ignored.md\n")?;
        let root = tmp_dir.path().join("project");
        write_tree(&root)?;
        write(&root.join("ancestor-ignored.md"), "# Ancestor ignored\n")?;

        let mut expected = kept();
        expected.insert(0, Path::new("ancestor-ignored.md").to_path_buf());
        assert_eq!(walk_markdown(&root, true, true)?, expected);
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn ancestor_gitignore_up_to_the_repository_root() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write(&tmp_dir.path().join(".gitignore"), "ancestor-ignored.md\n")?;
        fs::create_dir(tmp_dir.path().join(".git")).into_diagnostic()?;
        let root = tmp_dir.path().join("project");
        write_tree(&root)?;
        write(&root.join("ancestor-ignored.md"), "# Ancestor ignored\n")?;

        assert_eq!(walk_markdown(&root, true, true)?, kept());
        tmp_dir.close().into_diagnostic()
    }
}
