use core::num::NonZero;
use std::env;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::path::is_separator;
use std::thread;

use ignore::WalkBuilder;
use ignore::WalkParallel;
use ignore::types::Types;
use ignore::types::TypesBuilder;
use miette::IntoDiagnostic as _;
use miette::Result;
use miette::miette;

/// What the walker would spend on a walk of its own on a machine with `cores`.
/// Its own default stops at twelve however many there are, and a walk mado
/// sizes by hand would pass that without stopping there as well.
fn budget_for(cores: usize) -> usize {
    cores.min(12)
}

/// `budget_for` this machine.
pub(crate) fn thread_budget() -> usize {
    budget_for(thread::available_parallelism().map_or(1, NonZero::get))
}

/// How many of `walks` walks may run alongside each other on `budget` threads.
pub(crate) fn walks_at_once(budget: usize, walks: usize) -> usize {
    #[expect(
        clippy::integer_division_remainder_used,
        reason = "sharing a budget out is what division is for, and what is left over is meant to be dropped"
    )]
    let at_once = budget / threads_per_walk(budget, walks).max(1);

    at_once.min(walks).max(1)
}

/// What one walk may spend on itself where a command makes `walks` of them, and
/// `0` -- as many as it likes -- where it makes only one. Between them the ones
/// running alongside each other spend no more than `budget`.
///
/// A count is all there is to go on here, and it cannot say which walk holds
/// the tree, so this hands the budget to one walk at a time while there are no
/// more walks than threads. That is what a big walk among small ones needs and
/// what a crowd of small ones does not -- see #444.
pub(crate) fn threads_per_walk(budget: usize, walks: usize) -> usize {
    if walks < 2 {
        return 0;
    }

    #[expect(
        clippy::integer_division_remainder_used,
        reason = "sharing a budget out is what division is for, and what is left over is meant to be dropped"
    )]
    let threads = budget / walks.div_ceil(budget.max(1));

    threads.max(1)
}

#[non_exhaustive]
pub struct WalkParallelBuilder;

impl WalkParallelBuilder {
    /// Whether `path` holds a repository marker. `.git` is a directory in a
    /// clone and a file in a worktree or a submodule, and `.jj` is the other
    /// marker the `ignore` crate stops at.
    fn is_repository_root(path: &Path) -> bool {
        path.join(".git").exists() || path.join(".jj").exists()
    }

    /// `path` under a name a file can be opened by. `Path::parent` names the
    /// directory a relative path sits in with the empty path, which is not one.
    fn named(path: &Path) -> PathBuf {
        if path.as_os_str().is_empty() {
            Path::new(".").to_path_buf()
        } else {
            path.to_path_buf()
        }
    }

    /// The directory `path` sits in, named the way `path` names it, and `path`
    /// itself where there is nothing above it.
    fn parent_of(path: &Path) -> PathBuf {
        path.parent().map_or_else(|| Self::named(path), Self::named)
    }

    /// The answer for a `dir` with no boundary over it: it bounds itself, and
    /// nothing above it is read, unless it turns out to be in a repository
    /// after all.
    fn bounds_itself(dir: &Path) -> Option<Vec<PathBuf>> {
        let in_repository = dir
            .canonicalize()
            .is_ok_and(|path| path.ancestors().any(Self::is_repository_root));

        (!in_repository).then(Vec::new)
    }

    /// The ignore files a pattern sitting in `dir` needs handed back, or
    /// `None` in a repository, where the walker finds them on its own. They
    /// come from the directories between `current_dir` and `dir`, and are
    /// empty when `dir` lies outside `current_dir`, which leaves the pattern
    /// its own boundary. `above_is_repository` answers for the directories
    /// over `current_dir`, which no pattern can reach by walking up.
    ///
    /// Every name that gets here leads where it reads, so where each step
    /// stands can be read off the name rather than asked after.
    fn ignore_files_of(
        dir: &Path,
        current_dir: &Path,
        above_is_repository: bool,
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Option<Vec<PathBuf>> {
        let mut dirs = vec![];
        for at in dir.ancestors() {
            let named = Self::named(at);
            if Self::is_repository_root(&named) {
                return None;
            }

            let at_boundary = named == *current_dir || named == Path::new(".");
            dirs.push(named);
            if at_boundary {
                if above_is_repository {
                    return None;
                }
                dirs.reverse();
                return Some(Self::ignore_files_in(
                    &dirs,
                    respect_ignore,
                    respect_gitignore,
                ));
            }
        }

        // The walk up ran out without meeting the boundary, so `dir` is not
        // under it and the pattern is bounded by itself.
        Self::bounds_itself(dir)
    }

    /// Whether `path` takes something back. A line that opens with a `!` in an
    /// ignore file undoes what an earlier one said, and the walker ranks an
    /// `.ignore` over every `.gitignore` wherever the two sit, which nothing
    /// mado can hand a file back through will do.
    fn takes_something_back(path: &Path) -> bool {
        // A `!` opens a line that takes back; one written for itself is
        // escaped, and so opens with a backslash.
        fs::read_to_string(path).is_ok_and(|text| text.lines().any(|line| line.starts_with('!')))
    }

    /// Whether an `.ignore` over `pattern` takes something back that a
    /// `.gitignore` under it could be excluding. Reading these files for
    /// `pattern` -- or leaving them unread, which comes to the same -- puts
    /// them under every file the walk finds for itself, so the walk would drop
    /// what it should keep, quietly, and where a clone of the same tree keeps
    /// it. The walker's own arrangement is left to answer instead, which keeps
    /// what it should and reports more besides.
    fn taken_back_over(pattern: &Path) -> bool {
        pattern.canonicalize().is_ok_and(|at| {
            at.ancestors()
                .skip(1)
                .map(|over| over.join(".ignore"))
                .any(|file| file.is_file() && Self::takes_something_back(&file))
        })
    }

    /// The ignore files `dirs` hold, in the order to hand them over. Every
    /// `.gitignore` goes in before any `.ignore` so the kinds stay ranked, and
    /// each kind outermost first so the nearer file wins within a kind:
    /// `WalkBuilder` takes the last file handed to it as the one that wins.
    ///
    fn ignore_files_in(
        dirs: &[PathBuf],
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Vec<PathBuf> {
        [
            (".gitignore", respect_gitignore),
            (".ignore", respect_ignore),
        ]
        .into_iter()
        .filter(|&(_, wanted)| wanted)
        .flat_map(|(name, _)| dirs.iter().map(move |dir| dir.join(name)))
        .filter(|path| path.is_file())
        .collect()
    }

    /// `ignore_files_of` for the directory `pattern` sits in, remembering
    /// answers: a command naming every file in a directory asks the same
    /// question once per file.
    fn ignore_files_for(
        pattern: &Path,
        current_dir: &Path,
        above_is_repository: bool,
        respect_ignore: bool,
        respect_gitignore: bool,
        seen: &mut Vec<(PathBuf, Option<Vec<PathBuf>>)>,
    ) -> Option<Vec<PathBuf>> {
        // The walk hands back a path that is not a directory without asking
        // any ignore file about it, so reading them for one cannot change what
        // it reports, and a command can name a great many files.
        if !pattern.is_dir() {
            return Some(vec![]);
        }

        // Whatever mado does with the files over this one, they end up under
        // what the walk finds for itself, and one of them taking something
        // back is one the walk would then drop.
        if respect_gitignore && Self::taken_back_over(pattern) {
            return None;
        }
        // A name that does not lead where it reads bounds itself: nothing
        // above it can be rooted at a name the walk's answers begin with.
        if !Self::leads_where_it_reads(pattern) {
            return Self::bounds_itself(pattern);
        }

        let dir = Self::parent_of(pattern);
        let files = if let Some((_, files)) = seen.iter().find(|(at, _)| *at == dir) {
            files.clone()
        } else {
            let files = Self::ignore_files_of(
                &dir,
                current_dir,
                above_is_repository,
                respect_ignore,
                respect_gitignore,
            );
            seen.push((dir, files.clone()));
            files
        };

        files.as_ref()?;
        // A pattern that is a repository root is in one, whatever holds it.
        if Self::is_repository_root(pattern) {
            return None;
        }

        files
    }

    /// One walker for `patterns`, which need the same ignore files handed
    /// back. `files` is those, in the order to hand them over, or `None` for a
    /// repository, where the walker finds them itself. `threads` is what the
    /// walk may spend on itself, `0` meaning as many as it likes.
    fn build_group(
        patterns: &[&PathBuf],
        files: Option<&[PathBuf]>,
        types: &Types,
        threads: usize,
        reported: &mut Vec<PathBuf>,
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

        builder.threads(threads);
        builder.ignore(respect_ignore);
        builder.git_ignore(respect_gitignore);

        // `respect-gitignore` covers `.gitignore` files and nothing else. The
        // global Git ignore file and `.git/info/exclude` do not travel with
        // the tree, so reading them would make the result depend on the
        // machine and on the clone.
        builder.git_global(false);
        builder.git_exclude(false);

        if let Some(files) = files {
            // Outside a repository the walker drops `.gitignore` entirely.
            // `require_git(false)` puts it back, and `parents(false)` leaves
            // what the walker finds above a root out of the matching -- it
            // still reads it -- so the files between the boundary and the root
            // are handed back below.
            builder.require_git(false);
            builder.parents(false);

            for file in files {
                // `WalkBuilder` roots the files it is handed at whatever
                // `current_dir` was last set to, which is what lets each
                // directory's patterns keep their own anchoring.
                let dir = Self::named(file.parent().unwrap_or(file));
                builder.current_dir(dir);
                // The walker reads what is over a root itself, and reports the
                // globs it cannot parse there, whenever it has a `.gitignore`
                // to go looking for. Without one it never looks up there, and
                // this reading is the only one those files get. What it makes
                // of a file at a root rather than over one it keeps to itself,
                // here as on `main`.
                if let Some(err) = builder.add_ignore(file)
                    && !respect_gitignore
                    && !err.is_io()
                {
                    // Two groups can name one file two ways, and it is still
                    // one file with one thing wrong with it.
                    let at = file.canonicalize().unwrap_or_else(|_| file.clone());
                    if !reported.contains(&at) {
                        reported.push(at);
                        eprintln!("{err}");
                    }
                }
            }
        }

        builder.types(types.clone());

        Ok(builder.build_parallel())
    }

    /// Whether `pattern` leads where it reads. The walk answers by joining
    /// onto the name it was given, so a `.` written in the middle of that name
    /// is in the middle of every answer, and no ignore file above it can be
    /// rooted at a name those answers begin with. A `..` is the same, and a
    /// link makes the directory above a step something other than the name
    /// with that step taken off, which the walk up would otherwise read off
    /// the name.
    ///
    /// A separator on either end is not a step, and neither is a leading `./`,
    /// which the walker takes off a name and a root alike before comparing
    /// them. A separator run in the middle of a name is: it is dropped the way
    /// a `.` is, and leaves the same gap between the name and the answers.
    fn leads_where_it_reads(pattern: &Path) -> bool {
        // `Path::components` drops a `.` that is not the first step, so the
        // steps are counted off the name as it was written. Separators and a
        // `.` are ASCII, which survives a name that is not text being read as
        // text, and re-serialising the name would not: a `/` written on
        // Windows comes back a `\`.
        let written = pattern.to_string_lossy();
        let steps: Vec<&str> = written.split(is_separator).collect();
        let last = steps.len().saturating_sub(1);
        let says_nothing = steps.iter().enumerate().any(|(at, step)| {
            // A separator run says nothing where it is not the one that
            // starts the name or the one that ends it, and `Path::components`
            // drops it the way it drops a `.`.
            (*step == "." && at > 0) || (step.is_empty() && at > 0 && at < last)
        });
        if says_nothing {
            return false;
        }

        let mut at = PathBuf::new();
        for step in pattern.components() {
            if step == Component::ParentDir {
                return false;
            }

            at.push(step);
            if at.symlink_metadata().is_ok_and(|found| found.is_symlink()) {
                return false;
            }
        }

        true
    }

    /// One walker per set of patterns that need the same ignore files handed
    /// back. Which files those are is a property of a single pattern, but
    /// patterns that need the same ones can be walked together.
    ///
    /// Each walker is given a share of the threads rather than the run of the
    /// machine, on the understanding that `walks_at_once` of them are visited
    /// alongside each other. Visiting them one at a time is correct and slower;
    /// visiting all of them at once asks the machine for more than it has.
    fn build_from(
        patterns: &[PathBuf],
        current_dir: &Path,
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Result<Vec<WalkParallel>> {
        if patterns.is_empty() {
            return Err(miette!("files must be non-empty"));
        }

        // NOTE: Expect performance improvements with pre-filtering. Built the
        // once: every group selects the same thing, and reading the table of
        // file types in again for each of them costs more than the walk does.
        let types = TypesBuilder::new()
            .add_defaults()
            .select("markdown")
            .build()
            .into_diagnostic()?;

        let above_is_repository = current_dir
            .ancestors()
            .skip(1)
            .any(Self::is_repository_root);
        let mut seen = vec![];
        let mut groups: Vec<(Option<Vec<PathBuf>>, Vec<&PathBuf>)> = vec![];
        for pattern in patterns {
            let files = Self::ignore_files_for(
                pattern,
                current_dir,
                above_is_repository,
                respect_ignore,
                respect_gitignore,
                &mut seen,
            );
            // Two names for one directory are two keys, and have to be: the
            // walker roots a file it is handed at the name it was handed, and
            // matches it against paths built from the name a pattern was
            // written as. A group named `sub/../d1` cannot be handed the file
            // `./d1` walks by, so the two walk separately.
            match groups.iter_mut().find(|(at, _)| *at == files) {
                Some((_, members)) => members.push(pattern),
                None => groups.push((files, vec![pattern])),
            }
        }

        // The runner runs several of these alongside each other, so the walks
        // share out between them what one of them would have spent: a walk of
        // its own for every group would spend more on starting threads than on
        // walking.
        let threads = threads_per_walk(thread_budget(), groups.len());
        let mut reported = vec![];
        groups
            .iter()
            .map(|(files, members)| {
                Self::build_group(
                    members,
                    files.as_deref(),
                    &types,
                    threads,
                    &mut reported,
                    respect_ignore,
                    respect_gitignore,
                )
            })
            .collect()
    }

    /// The walkers a command's paths need, ready to be visited.
    ///
    /// Each is given a share of the threads rather than the run of the
    /// machine, on the understanding that several are visited alongside each
    /// other. Visiting them one at a time is correct and slower; visiting all
    /// of them at once asks the machine for more than it has.
    #[inline]
    pub fn build(
        patterns: &[PathBuf],
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> Result<Vec<WalkParallel>> {
        // The names paths are written under are compared with this, so it is
        // the one the operating system gives rather than one resolved into a
        // shape nobody writes: `fs::canonicalize` answers on Windows with a
        // `\\?\C:\...` that no name reaches by walking up. A directory mado
        // cannot name at all is one no pattern can be found under.
        let current_dir = env::current_dir().unwrap_or_default();

        Self::build_from(patterns, &current_dir, respect_ignore, respect_gitignore)
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::sync::Arc;
    use miette::{Context as _, IntoDiagnostic as _};
    use std::{
        env,
        ffi::OsStr,
        fs,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use ignore::{DirEntry, WalkState};
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    use super::{WalkParallelBuilder, budget_for, threads_per_walk, walks_at_once};

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
        let current_dir = env::current_dir()
            .and_then(|dir| dir.canonicalize())
            .into_diagnostic()?;
        walk_markdown_from(root, &current_dir, respect_ignore, respect_gitignore)
    }

    /// `walk_markdown` for a mado started in `current_dir` rather than
    /// wherever the tests happen to run.
    fn walk_markdown_from(
        root: &Path,
        current_dir: &Path,
        respect_ignore: bool,
        respect_gitignore: bool,
    ) -> miette::Result<Vec<PathBuf>> {
        let patterns = vec![root.to_path_buf()];
        let built = WalkParallelBuilder::build_from(
            &patterns,
            current_dir,
            respect_ignore,
            respect_gitignore,
        );
        let collector = PathCollector::new();

        for walker in built? {
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
    fn the_budget_is_what_the_walker_would_have_spent() {
        // The walker's own default stops at twelve however many cores it is
        // run on, and a walk mado sizes by hand would pass it without this.
        assert_eq!(budget_for(64), 12);
        assert_eq!(budget_for(12), 12);
        assert_eq!(budget_for(4), 4);
    }

    #[test]
    fn walks_never_spend_more_than_one_of_them_would() {
        // The runner leans on there being one to run, and asks for one of its
        // own besides.
        assert!(walks_at_once(0, 0) >= 1);
        assert!(walks_at_once(0, 8) >= 1);

        for budget in [1_usize, 2, 8, 10, 12] {
            // One walk is left to spend what the walker would have on it.
            assert_eq!(threads_per_walk(budget, 1), 0);
            assert_eq!(walks_at_once(budget, 1), 1);

            // Several share it out between them instead.
            for walks in [2_usize, 3, 5, 7, 12, 13, 200] {
                let at = format!("{budget} threads over {walks} walks");
                let threads = threads_per_walk(budget, walks);
                assert!(threads >= 1, "{at}");
                assert!(walks_at_once(budget, walks) * threads <= budget, "{at}");
            }
        }
    }

    #[test]
    fn build_groups_patterns_that_share_a_boundary() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;

        // Nothing has to be read for a file, so one walk covers them all.
        let alongside = vec![
            Path::new("README.md").to_path_buf(),
            Path::new("CHANGELOG.md").to_path_buf(),
            tmp_dir.path().join("keep.md"),
        ];
        let alongside_walkers = WalkParallelBuilder::build(&alongside, true, true)?;
        assert_eq!(alongside_walkers.len(), 1);

        // A directory has to be answered for, and answers differently from a
        // file, so it gets its own walk.
        let mut with_directory = alongside;
        with_directory.push(Path::new("action").to_path_buf());
        let directory_walkers = WalkParallelBuilder::build(&with_directory, true, true)?;
        assert_eq!(directory_walkers.len(), 2);

        tmp_dir.close().into_diagnostic()
    }

    /// A tree of sibling directories under one boundary, each holding a
    /// directory to lint and none of them an ignore file of its own.
    fn write_siblings(root: &Path, count: usize) -> miette::Result<Vec<PathBuf>> {
        (0..count)
            .map(|at| {
                let path = root.join(format!("d{at}/sub"));
                write(&path.join("a.md"), "# Fine\n")?;
                Ok(path)
            })
            .collect()
    }

    #[test]
    fn build_walks_siblings_under_one_boundary_together() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        let current_dir = tmp_dir.path().canonicalize().into_diagnostic()?;
        let patterns = write_siblings(&current_dir, 8)?;

        // Each of these sits in a directory of its own, but they all read the
        // same ignore files, so one walk covers them however many there are.
        let shared = WalkParallelBuilder::build_from(&patterns, &current_dir, true, true)?;
        assert_eq!(shared.len(), 1);

        // One of them holding an ignore file the others must not see splits
        // that one off, and no more.
        write(&current_dir.join("d0/.gitignore"), "a.md\n")?;
        let split = WalkParallelBuilder::build_from(&patterns, &current_dir, true, true)?;
        assert_eq!(split.len(), 2);

        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn build_walks_every_named_file_together() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        let current_dir = tmp_dir.path().canonicalize().into_diagnostic()?;
        write_siblings(&current_dir, 8)?;
        write(&current_dir.join("d0/.gitignore"), "a.md\n")?;

        // The walk hands a path that is not a directory back without asking
        // any ignore file about it, so no file needs one read for it and they
        // all belong to the same walk, whatever directories they sit in.
        let patterns: Vec<PathBuf> = (0..8)
            .map(|at| current_dir.join(format!("d{at}/sub/a.md")))
            .collect();
        let walkers = WalkParallelBuilder::build_from(&patterns, &current_dir, true, true)?;
        assert_eq!(walkers.len(), 1);

        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_up_to_a_repository_root_above_the_boundary() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        let root = tmp_dir.path().canonicalize().into_diagnostic()?;
        fs::create_dir(root.join(".git")).into_diagnostic()?;
        write(&root.join(".gitignore"), "above.md\n")?;
        let current_dir = root.join("proj");
        write(&current_dir.join("docs/above.md"), "# Above\n")?;
        write(&current_dir.join("docs/keep.md"), "# Keep\n")?;

        // The repository holds the directory mado was started in, so its root
        // is the boundary and its `.gitignore` still counts.
        let actual = walk_markdown_from(&current_dir.join("docs"), &current_dir, true, true)?;
        assert_eq!(actual, vec![Path::new("keep.md").to_path_buf()]);
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_for_a_path_that_is_not_there() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        write_tree(tmp_dir.path())?;

        // A path mado cannot look above must not stop the ones it can walk.
        let patterns = vec![
            tmp_dir.path().join("missing/a.md"),
            tmp_dir.path().to_path_buf(),
        ];
        let current_dir = tmp_dir.path().canonicalize().into_diagnostic()?;
        let walkers = WalkParallelBuilder::build_from(&patterns, &current_dir, true, true)?;
        let collector = PathCollector::new();
        for walker in walkers {
            walker.run(|| Box::new(collector.gen_visitor()));
        }
        let markdown = collector
            .paths()?
            .into_iter()
            .filter(|path| path.extension() == Some(OsStr::new("md")))
            .count();
        assert_eq!(markdown, kept().len());
        tmp_dir.close().into_diagnostic()
    }

    #[test]
    fn gitignore_when_the_boundary_cannot_be_named() -> miette::Result<()> {
        let tmp_dir = TempDir::new().into_diagnostic()?;
        // Canonical, so the name leads where it reads and the walk up is the
        // thing that runs out rather than the reading of the name.
        let at = tmp_dir.path().canonicalize().into_diagnostic()?;
        write(&at.join(".gitignore"), "keep.md\n")?;
        let root = at.join("project");
        write_tree(&root)?;

        // With no directory to measure against, every path bounds itself, so
        // nothing above it is read.
        let actual = walk_markdown_from(&root, Path::new(""), true, true)?;
        assert_eq!(actual, kept());
        tmp_dir.close().into_diagnostic()
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
