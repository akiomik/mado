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
    /// Whether `path` or any of its ancestors holds Git metadata. `.git` is a
    /// directory in a clone and a file in a worktree or a submodule.
    fn in_git_repository(path: &Path) -> bool {
        path.canonicalize()
            .is_ok_and(|path| path.ancestors().any(|dir| dir.join(".git").exists()))
    }

    #[inline]
    pub fn build(
        patterns: &[PathBuf],
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Result<WalkParallel> {
        let (head_pattern, tail_patterns) = patterns
            .split_first()
            .ok_or_else(|| miette!("files must be non-empty"))?;
        let mut builder = WalkBuilder::new(head_pattern);
        for pattern in tail_patterns {
            builder.add(pattern);
        }

        builder.ignore(respect_ignore);
        builder.git_ignore(respect_gitignore);

        // `respect-gitignore` covers `.gitignore` files and nothing else. The
        // global Git ignore file and `.git/info/exclude` do not travel with the
        // tree, so reading them would make the result depend on the machine.
        builder.git_global(false);
        builder.git_exclude(false);

        // Outside a repository `ignore` drops `.gitignore` altogether, which
        // makes the result depend on whether the tree still carries `.git`.
        // `require_git(false)` applies it anyway, and with no repository root
        // to stop at, `parents(false)` bounds the search at the given paths
        // rather than letting it run to the filesystem root.
        if respect_gitignore && !patterns.iter().any(|p| Self::in_git_repository(p)) {
            builder.require_git(false);
            builder.parents(false);
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
        let walker = WalkParallelBuilder::build(&patterns, respect_ignore, respect_gitignore)?;
        let collector = PathCollector::new();

        walker.run(|| Box::new(collector.gen_visitor()));

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
        let builder = WalkParallelBuilder::build(&paths, true, true)?;
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
        write(
            &tmp_dir.path().join(".git"),
            "gitdir: ../.git/worktrees/tree\n",
        )?;

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
