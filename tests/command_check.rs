use std::fs::File;
use std::fs::create_dir_all;
use std::fs::read_to_string;
use std::fs::write;
use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_cmd::cargo_bin;
use indoc::formatdoc;
use indoc::indoc;
use mado::Config;
use miette::Context as _;
use miette::IntoDiagnostic as _;
use miette::Result;
use miette::miette;
use tempfile::tempdir;

fn with_tmp_file<F>(name: &str, content: &str, f: F) -> Result<()>
where
    F: FnOnce(PathBuf) -> Result<()>,
{
    let tmp_dir = tempdir().into_diagnostic()?;
    let path = tmp_dir.path().join(name);
    let mut tmp_file = File::create(path.clone()).into_diagnostic()?;
    write!(tmp_file, "{content}").into_diagnostic()?;

    f(path)?;

    tmp_dir.close().into_diagnostic()
}

#[test]
fn check() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.args(["check", "."]).assert();
    assert.success().stdout("All checks passed!\n");
}

#[test]
fn check_quiet() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.args(["check", "--quiet", "."]).assert();
    assert.success().stdout("");
}

#[test]
fn check_quiet_with_config() -> Result<()> {
    let mut config = Config::default();
    config.lint.quiet = true;
    config.lint.md013.tables = false;
    config.lint.md013.code_blocks = false;
    config.lint.md024.allow_different_nesting = true;
    let content = toml::to_string(&config).into_diagnostic()?;

    with_tmp_file("mado.toml", &content, |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd.args(["--config", path_str, "check", "."]).assert();
        assert.success().stdout("");
        Ok(())
    })
}

#[test]
fn check_stdin() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd
        .env("CLICOLOR_FORCE", "1")
        .write_stdin("#Hello.")
        .args(["check"])
        .assert();
    assert.failure().stdout(
        indoc! {"
            \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD018\u{1b}[0m No space after hash on atx style header
            \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD041\u{1b}[0m First line in file should be a top level header
            \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD047\u{1b}[0m File should end with a single newline character

            Found 3 errors.
        "}
    );
}

#[test]
fn check_stdin_no_color() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .write_stdin("#Hello.")
        .args(["check"])
        .assert();
    assert.failure().stdout(indoc! {"
        (stdin):1:1: MD018 No space after hash on atx style header
        (stdin):1:1: MD041 First line in file should be a top level header
        (stdin):1:1: MD047 File should end with a single newline character

        Found 3 errors.
    "});
}

#[test]
fn check_empty_stdin() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.write_stdin("").args(["check"]).assert();
    assert.success().stdout("All checks passed!\n");
}

#[test]
fn check_empty_stdin_with_file() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd
            .env("CLICOLOR_FORCE", "1")
            .write_stdin("")
            .args(["check", path_str])
            .assert();
        assert.failure().stdout(
            formatdoc! {"
                \u{1b}[1m{path_str}\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD018\u{1b}[0m No space after hash on atx style header
                \u{1b}[1m{path_str}\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD041\u{1b}[0m First line in file should be a top level header
                \u{1b}[1m{path_str}\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD047\u{1b}[0m File should end with a single newline character

                Found 3 errors.
            "}
        );
        Ok(())
    })
}

#[test]
fn check_empty_stdin_with_file_no_color() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd
            .env_remove("CLICOLOR_FORCE")
            .env("NO_COLOR", "1")
            .write_stdin("")
            .args(["check", path_str])
            .assert();
        assert.failure().stdout(formatdoc! {"
            {path_str}:1:1: MD018 No space after hash on atx style header
            {path_str}:1:1: MD041 First line in file should be a top level header
            {path_str}:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_stdin_with_file() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd
            .env("CLICOLOR_FORCE", "1")
            .write_stdin("#Hello.")
            .args(["check", path_str])
            .assert();
        assert.failure().stdout(
            indoc! {"
                \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD018\u{1b}[0m No space after hash on atx style header
                \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD041\u{1b}[0m First line in file should be a top level header
                \u{1b}[1m(stdin)\u{1b}[0m\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m1\u{1b}[34m:\u{1b}[0m \u{1b}[1;31mMD047\u{1b}[0m File should end with a single newline character

                Found 3 errors.
            "}
        );
        Ok(())
    })
}

#[test]
fn check_stdin_with_file_no_color() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd
            .env_remove("CLICOLOR_FORCE")
            .env("NO_COLOR", "1")
            .write_stdin("#Hello.")
            .args(["check", path_str])
            .assert();
        assert.failure().stdout(indoc! {"
            (stdin):1:1: MD018 No space after hash on atx style header
            (stdin):1:1: MD041 First line in file should be a top level header
            (stdin):1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_exclusion() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd.args(["check", path_str, "--exclude", "*.md"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

// The next three tests each pin down a different half of the fix for #168.
// `--exclude` patterns and walked file paths are only guaranteed to match
// when both `Lint::exclude_set` (src/config/lint.rs) and
// `MarkdownLintVisitor::visit_inner` (src/service/visitor.rs) strip a
// leading "./" the same way. Dropping the normalization on just one side
// makes at least one of these fail:
//   - default target (walked path carries "./") + pattern without "./"
//     needs visit_inner to normalize the walked path.
//   - explicit target (walked path has no "./") + pattern with "./"
//     needs exclude_set to normalize the pattern.
//   - default target + pattern with "./"
//     needs both sides to agree, since walked path and pattern both carry
//     "./" and must be stripped the same way to still match afterwards.

#[test]
fn check_exclusion_default_target_without_dot_slash_prefix() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let dir = path.parent().wrap_err("failed to get parent dir")?;
        let mut cmd = Command::new(cargo_bin!("mado"));
        let assert = cmd
            .current_dir(dir)
            .args(["check", "--exclude", "test.md"])
            .assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_exclusion_explicit_target_with_dot_slash_prefix() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let dir = path.parent().wrap_err("failed to get parent dir")?;
        let mut cmd = Command::new(cargo_bin!("mado"));
        let assert = cmd
            .current_dir(dir)
            .args(["check", "test.md", "--exclude", "./test.md"])
            .assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_exclusion_default_target_with_dot_slash_prefix() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let dir = path.parent().wrap_err("failed to get parent dir")?;
        let mut cmd = Command::new(cargo_bin!("mado"));
        let assert = cmd
            .current_dir(dir)
            .args(["check", "--exclude", "./test.md"])
            .assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

/// Fails where an `.ignore` above `root` takes a path back with a `!` line.
/// mado reads every parent directory looking for one, so a file like that on
/// the machine running the tests changes what these trees report, and the
/// failure would be nothing to do with the tree under test.
fn nothing_above_takes_a_path_back(root: &Path) -> Result<()> {
    let at = root.canonicalize().into_diagnostic()?;
    for over in at.ancestors().skip(1) {
        let file = over.join(".ignore");
        let takes_back =
            read_to_string(&file).is_ok_and(|text| text.lines().any(|line| line.starts_with('!')));
        if takes_back {
            return Err(miette!(
                "{} takes a path back, and these tests need no such file over them",
                file.display()
            ));
        }
    }

    Ok(())
}

/// Lays out a tree of files under a temporary directory. A path ending in `/`
/// is created as an empty directory.
fn with_tree<F>(entries: &[(&str, &str)], f: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let tmp_dir = tempdir().into_diagnostic()?;
    let root = tmp_dir.path();
    nothing_above_takes_a_path_back(root)?;
    for (path, content) in entries {
        let path = root.join(path);
        if let Some(dir) = path.parent() {
            create_dir_all(dir).into_diagnostic()?;
        }
        if content.is_empty() && path.to_string_lossy().ends_with('/') {
            create_dir_all(&path).into_diagnostic()?;
        } else {
            write(&path, content).into_diagnostic()?;
        }
    }

    f(root)?;

    tmp_dir.close().into_diagnostic()
}

/// A `mado check` run rooted in `dir`, with colour out of the way.
fn check_in(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(cargo_bin!("mado"));
    cmd.current_dir(dir)
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .arg("check")
        .args(args);
    cmd
}

/// A tree whose `.gitignore` lists a directory mado would otherwise report on,
/// with no Git metadata anywhere: what a source archive or a Docker context
/// copied without `.git` looks like.
const IGNORED_BUILD_OUTPUT: &[(&str, &str)] = &[
    (".gitignore", "target/\n"),
    ("target/generated.md", "#Hello."),
];

#[test]
fn check_respects_gitignore_without_git_metadata() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        let assert = check_in(root, &["."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_without_respect_gitignore_walks_ignored_files() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        write(
            root.join("mado.toml"),
            "[lint]\nrespect-gitignore = false\n",
        )
        .into_diagnostic()?;
        let assert = check_in(root, &["."]).assert();
        assert.failure().stdout(indoc! {"
            ./target/generated.md:1:1: MD018 No space after hash on atx style header
            ./target/generated.md:1:1: MD041 First line in file should be a top level header
            ./target/generated.md:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

/// A project whose root `.gitignore` excludes a file in a subdirectory, linted
/// by naming that subdirectory rather than the root.
const IGNORED_BELOW_A_SUBDIRECTORY: &[(&str, &str)] = &[
    ("proj/.gitignore", "docs/generated.md\n"),
    ("proj/docs/generated.md", "#Hello."),
    ("proj/docs/keep.md", "# Fine\n"),
];

#[test]
fn check_subdirectory_reads_the_project_root_gitignore_without_git_metadata() -> Result<()> {
    with_tree(IGNORED_BELOW_A_SUBDIRECTORY, |root| {
        let assert = check_in(&root.join("proj"), &["docs"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_subdirectory_reads_the_project_root_gitignore_with_git_metadata() -> Result<()> {
    with_tree(IGNORED_BELOW_A_SUBDIRECTORY, |root| {
        create_dir_all(root.join("proj/.git")).into_diagnostic()?;
        let assert = check_in(&root.join("proj"), &["docs"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_subdirectory_reads_the_project_root_gitignore_with_jj_metadata() -> Result<()> {
    with_tree(IGNORED_BELOW_A_SUBDIRECTORY, |root| {
        create_dir_all(root.join("proj/.jj")).into_diagnostic()?;
        let assert = check_in(&root.join("proj"), &["docs"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

/// Two `.gitignore` files whose patterns are anchored to their own directory,
/// so a pattern read against the wrong root matches the wrong file.
const ANCHORED_AT_TWO_LEVELS: &[(&str, &str)] = &[
    ("proj/.gitignore", "/only-at-top.md\n"),
    ("proj/docs/.gitignore", "/sub/only-in-sub.md\n"),
    ("proj/docs/sub/only-at-top.md", "#Hello."),
    ("proj/docs/sub/only-in-sub.md", "#Hello."),
    ("proj/docs/sub/keep.md", "# Fine\n"),
];

/// Only `only-in-sub.md` is anchored at a directory that contains it, so it is
/// the only one of the two the walk drops.
const ANCHORED_AT_TWO_LEVELS_REPORT: &str = indoc! {"
    docs/sub/only-at-top.md:1:1: MD018 No space after hash on atx style header
    docs/sub/only-at-top.md:1:1: MD041 First line in file should be a top level header
    docs/sub/only-at-top.md:1:1: MD047 File should end with a single newline character

    Found 3 errors.
"};

#[test]
fn check_keeps_gitignore_anchoring_per_directory_without_git_metadata() -> Result<()> {
    with_tree(ANCHORED_AT_TWO_LEVELS, |root| {
        let assert = check_in(&root.join("proj"), &["docs/sub"]).assert();
        assert.failure().stdout(ANCHORED_AT_TWO_LEVELS_REPORT);
        Ok(())
    })
}

#[test]
fn check_keeps_gitignore_anchoring_per_directory_with_git_metadata() -> Result<()> {
    with_tree(ANCHORED_AT_TWO_LEVELS, |root| {
        create_dir_all(root.join("proj/.git")).into_diagnostic()?;
        let assert = check_in(&root.join("proj"), &["docs/sub"]).assert();
        assert.failure().stdout(ANCHORED_AT_TWO_LEVELS_REPORT);
        Ok(())
    })
}

/// An `.ignore` and a `.gitignore` in different parent directories disagreeing
/// about the same file. `.ignore` wins wherever either sits, so the file is
/// dropped and mado has nothing to report.
const CONFLICTING_KINDS: &[(&str, &str)] = &[
    ("proj/.ignore", "docs/sub/conflict.md\n"),
    ("proj/docs/.gitignore", "!sub/conflict.md\n"),
    ("proj/docs/sub/conflict.md", "#Hello."),
];

/// An `.ignore` above the path taking back what a `.gitignore` under it
/// excludes. The walker ranks the `.ignore` over the `.gitignore` wherever the
/// two sit; a file handed back to it ranks under both.
const TAKEN_BACK_FROM_ABOVE: &[(&str, &str)] = &[
    ("proj/.ignore", "!ignored.md\n"),
    ("proj/docs/.gitignore", "ignored.md\n"),
    ("proj/docs/ignored.md", "#Hello."),
];

#[test]
fn check_keeps_what_an_ignore_file_above_the_path_takes_back() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        // Handing the `.ignore` back would rank it under the `.gitignore` and
        // drop this file, quietly, where a clone of the same tree keeps it.
        let assert = check_in(&root.join("proj"), &["docs"]).assert();
        assert.failure().stdout(indoc! {"
            docs/ignored.md:1:1: MD018 No space after hash on atx style header
            docs/ignored.md:1:1: MD041 First line in file should be a top level header
            docs/ignored.md:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_keeps_what_an_ignore_file_above_the_path_takes_back_in_a_repository() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        create_dir_all(root.join("proj/.git")).into_diagnostic()?;

        // The walker finds these files itself here, so there is nothing for
        // mado to arrange and nothing to say about having left it alone.
        let assert = check_in(&root.join("proj"), &["docs"]).assert();
        assert.failure().stderr("").stdout(indoc! {"
            docs/ignored.md:1:1: MD018 No space after hash on atx style header
            docs/ignored.md:1:1: MD041 First line in file should be a top level header
            docs/ignored.md:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_reads_gitignore_over_an_ignore_file_it_does_not_respect() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        let proj = root.join("proj");
        write(proj.join("mado.toml"), "[lint]\nrespect-ignore = false\n").into_diagnostic()?;

        // Neither the walk nor mado reads the `.ignore`, so nothing of its
        // stands over what the walk finds and the `.gitignore` is left to say.
        let assert = check_in(&proj, &["docs"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_reports_every_file_it_cannot_read() -> Result<()> {
    with_tree(&[], |root| {
        // Two files with nothing to tell them apart in what the reader says
        // about them, and each of them is still a file of its own.
        write(root.join("a.md"), b"\xff\xfe").into_diagnostic()?;
        write(root.join("b.md"), b"\xff\xfe").into_diagnostic()?;

        let output = check_in(root, &["."]).output().into_diagnostic()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.lines().count(), 2);
        Ok(())
    })
}

#[test]
fn check_reports_a_broken_glob_two_groups_read_once() -> Result<()> {
    with_tree(
        &[
            ("proj/d1/.gitignore", "{a\n"),
            ("proj/d1/docs/.gitignore", "nothing.md\n"),
            ("proj/d1/docs/a.md", "# Fine\n"),
            ("proj/d1/docs/deep/b.md", "# Fine\n"),
        ],
        |root| {
            // The two names need different files handed back, so they walk as
            // two groups, and the file both are handed is one file with one
            // thing wrong with it.
            let output = check_in(&root.join("proj"), &["d1/docs", "d1/docs/deep"])
                .output()
                .into_diagnostic()?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            let complaints = stderr
                .lines()
                .filter(|line| line.contains("error parsing glob"))
                .count();
            assert_eq!(complaints, 1);
            Ok(())
        },
    )
}

#[test]
fn check_says_which_ignore_file_sent_gitignore_back_to_git() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        // Two names under the one `.ignore`, and a reader who cannot see why
        // `.gitignore` stopped applying is told once which file it was.
        let output = check_in(&root.join("proj"), &["docs", "docs"])
            .output()
            .into_diagnostic()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.lines().count(), 1);
        assert!(stderr.contains(".ignore: takes a path back"));
        Ok(())
    })
}

#[test]
fn check_says_nothing_about_a_path_that_is_not_a_directory() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        let proj = root.join("proj");
        write(proj.join("loose.md"), "# Fine\n").into_diagnostic()?;

        // The walk hands a file back without asking any ignore file about it,
        // so which files mado would have handed over changes nothing for it.
        let assert = check_in(&proj, &["loose.md"]).assert();
        assert.success().stderr("");
        Ok(())
    })
}

#[test]
fn check_says_nothing_where_no_ignore_file_is_read() -> Result<()> {
    with_tree(TAKEN_BACK_FROM_ABOVE, |root| {
        let proj = root.join("proj");
        write(proj.join("mado.toml"), "[lint]\nrespect-ignore = false\n").into_diagnostic()?;

        let assert = check_in(&proj, &["docs"]).assert();
        assert.success().stderr("");
        Ok(())
    })
}

#[test]
fn check_keeps_ignore_ahead_of_gitignore_without_git_metadata() -> Result<()> {
    with_tree(CONFLICTING_KINDS, |root| {
        let assert = check_in(&root.join("proj"), &["docs/sub"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_keeps_ignore_ahead_of_gitignore_with_git_metadata() -> Result<()> {
    with_tree(CONFLICTING_KINDS, |root| {
        create_dir_all(root.join("proj/.git")).into_diagnostic()?;
        let assert = check_in(&root.join("proj"), &["docs/sub"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_keeps_each_spelling_of_a_directory_anchored_to_itself() -> Result<()> {
    with_tree(
        &[
            ("proj/.gitignore", "/d1/docs/ignored.md\n"),
            ("proj/d1/docs/ignored.md", "#Hello."),
            ("proj/d1/docs/keep.md", "# Fine\n"),
            ("proj/sub/keep.md", "# Fine\n"),
        ],
        |root| {
            // The plain name reads what is above it; the one that steps back
            // over itself bounds itself, and the walk of each is its own.
            let assert = check_in(&root.join("proj"), &["./d1/docs", "sub/../d1/docs"]).assert();
            assert.failure().stdout(indoc! {"
                sub/../d1/docs/ignored.md:1:1: MD018 No space after hash on atx style header
                sub/../d1/docs/ignored.md:1:1: MD041 First line in file should be a top level header
                sub/../d1/docs/ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

/// A project whose root `.gitignore` excludes a file in a subdirectory by a
/// pattern anchored to that root, which only reaches a name it prefixes.
const ANCHORED_BELOW_A_SUBDIRECTORY: &[(&str, &str)] = &[
    ("proj/.gitignore", "/docs/ignored.md\n"),
    ("proj/docs/ignored.md", "#Hello."),
    ("proj/docs/keep.md", "# Fine\n"),
];

#[test]
fn check_reads_an_anchored_parent_pattern_for_a_name_that_leads_where_it_reads() -> Result<()> {
    with_tree(ANCHORED_BELOW_A_SUBDIRECTORY, |root| {
        let assert = check_in(&root.join("proj"), &["./docs"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_reads_an_anchored_parent_pattern_for_a_name_that_ends_in_a_separator() -> Result<()> {
    with_tree(ANCHORED_BELOW_A_SUBDIRECTORY, |root| {
        // A separator on the end is not a step: joining onto the name takes
        // it, which is what the walk does to answer.
        let assert = check_in(&root.join("proj"), &["docs/"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_bounds_a_name_with_a_separator_run_in_the_middle_of_it() -> Result<()> {
    with_tree(
        &[
            ("proj/.gitignore", "ignored.md\n"),
            ("proj/docs/ignored.md", "#Hello."),
            ("proj/docs/keep.md", "# Fine\n"),
        ],
        |root| {
            // A run of separators is dropped the way a `.` is, and leaves the
            // same gap between the name and the answers the walk builds from
            // it: read against the file above, `.//docs/ignored.md` arrives
            // with a leading separator and matches a pattern it should not.
            let assert = check_in(&root.join("proj"), &[".//docs"]).assert();
            assert.failure().stdout(indoc! {"
                .//docs/ignored.md:1:1: MD018 No space after hash on atx style header
                .//docs/ignored.md:1:1: MD041 First line in file should be a top level header
                .//docs/ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[test]
fn check_bounds_a_name_that_does_not_lead_where_it_reads() -> Result<()> {
    with_tree(ANCHORED_BELOW_A_SUBDIRECTORY, |root| {
        // `docs/.` has the walk answering about `docs/./ignored.md`, which no
        // ignore file above `docs` can be rooted at a name for, so the path
        // bounds itself and what is above it goes unread.
        let assert = check_in(&root.join("proj"), &["docs/."]).assert();
        assert.failure().stdout(indoc! {"
            docs/./ignored.md:1:1: MD018 No space after hash on atx style header
            docs/./ignored.md:1:1: MD041 First line in file should be a top level header
            docs/./ignored.md:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_does_not_bound_a_name_that_does_not_lead_where_it_reads_in_a_repository() -> Result<()> {
    with_tree(ANCHORED_BELOW_A_SUBDIRECTORY, |root| {
        create_dir_all(root.join("proj/.git")).into_diagnostic()?;

        // Nothing is handed back in a repository, so nothing rests on how the
        // name is spelled: the walk finds the files above `docs` itself.
        let assert = check_in(&root.join("proj"), &["docs/."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_walks_the_directory_a_name_that_undoes_itself_lands_in() -> Result<()> {
    with_tree(
        &[("docs/bad.md", "#Hello."), ("top.md", "#Hello.")],
        |root| {
            // `docs/..` is the directory mado was started in, not nothing at all.
            let assert = check_in(root, &["docs/.."]).assert();
            assert.failure().stdout(indoc! {"
            docs/../docs/bad.md:1:1: MD018 No space after hash on atx style header
            docs/../docs/bad.md:1:1: MD041 First line in file should be a top level header
            docs/../docs/bad.md:1:1: MD047 File should end with a single newline character
            docs/../top.md:1:1: MD018 No space after hash on atx style header
            docs/../top.md:1:1: MD041 First line in file should be a top level header
            docs/../top.md:1:1: MD047 File should end with a single newline character

            Found 6 errors.
        "});
            Ok(())
        },
    )
}

#[cfg(unix)]
#[test]
fn check_does_not_read_the_boundary_gitignore_for_a_tree_a_link_leads_out_to() -> Result<()> {
    with_tree(
        &[
            ("proj/.gitignore", "ignored.md\n"),
            ("outside/docs/ignored.md", "#Hello."),
        ],
        |root| {
            symlink(root.join("outside"), root.join("proj/link")).into_diagnostic()?;

            // `link/docs` reads as a name under `proj` and is not one.
            let assert = check_in(&root.join("proj"), &["link/docs"]).assert();
            assert.failure().stdout(indoc! {"
                link/docs/ignored.md:1:1: MD018 No space after hash on atx style header
                link/docs/ignored.md:1:1: MD041 First line in file should be a top level header
                link/docs/ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[cfg(unix)]
#[test]
fn check_does_not_read_the_boundary_gitignore_for_a_link_named_on_its_own() -> Result<()> {
    with_tree(
        &[
            ("proj/.gitignore", "ignored.md\n"),
            ("outside/ignored.md", "#Hello."),
        ],
        |root| {
            symlink(root.join("outside"), root.join("proj/link")).into_diagnostic()?;

            // The walk descends into the name it was given, so where that name
            // leads counts as much as where the ones above it lead.
            let assert = check_in(&root.join("proj"), &["link"]).assert();
            assert.failure().stdout(indoc! {"
                link/ignored.md:1:1: MD018 No space after hash on atx style header
                link/ignored.md:1:1: MD041 First line in file should be a top level header
                link/ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[cfg(unix)]
#[test]
fn check_names_a_file_the_way_the_command_named_the_path() -> Result<()> {
    with_tree(&[("docs/bad.md", "#Hello.")], |root| {
        // What is reported is built by joining onto the name mado was given,
        // and mado does not rewrite that name.
        let assert = check_in(root, &["docs/./."]).assert();
        assert.failure().stdout(indoc! {"
            docs/././bad.md:1:1: MD018 No space after hash on atx style header
            docs/././bad.md:1:1: MD041 First line in file should be a top level header
            docs/././bad.md:1:1: MD047 File should end with a single newline character

            Found 3 errors.
        "});
        Ok(())
    })
}

#[test]
fn check_reads_each_root_gitignore_when_only_one_is_in_a_repository() -> Result<()> {
    with_tree(
        &[
            ("repo/.git/HEAD", "ref: refs/heads/main\n"),
            ("repo/.gitignore", "ignored.md\n"),
            ("repo/ignored.md", "#Hello."),
            ("archive/.gitignore", "ignored.md\n"),
            ("archive/ignored.md", "#Hello."),
        ],
        |root| {
            let assert = check_in(root, &["repo", "archive"]).assert();
            assert.success().stdout("All checks passed!\n");
            Ok(())
        },
    )
}

#[test]
fn check_respect_ignore_does_not_depend_on_respect_gitignore() -> Result<()> {
    with_tree(
        &[
            ("proj/.ignore", "skipped.md\n"),
            ("proj/docs/skipped.md", "#Hello."),
        ],
        |root| {
            let proj = root.join("proj");
            for respect_ignore in [true, false] {
                for respect_gitignore in [true, false] {
                    let config = formatdoc! {"
                        [lint]
                        respect-ignore = {respect_ignore}
                        respect-gitignore = {respect_gitignore}
                    "};
                    write(proj.join("mado.toml"), config).into_diagnostic()?;

                    // `.ignore` decides this on its own: `respect-gitignore`
                    // must not move the directories `.ignore` is read from.
                    let assert = check_in(&proj, &["docs"]).assert();
                    if respect_ignore {
                        assert.success();
                    } else {
                        assert.failure();
                    }
                }
            }
            Ok(())
        },
    )
}

/// A parent `.ignore` whose only pattern the glob parser rejects, alongside a
/// clean file to lint, and `respect-gitignore` turned off.
const BROKEN_PARENT_IGNORE: &[(&str, &str)] =
    &[("proj/.ignore", "{a\n"), ("proj/docs/a.md", "# Fine\n")];

#[test]
fn check_reports_a_broken_glob_in_a_parent_ignore_file() -> Result<()> {
    with_tree(BROKEN_PARENT_IGNORE, |root| {
        let proj = root.join("proj");
        write(
            proj.join("mado.toml"),
            "[lint]\nrespect-gitignore = false\n",
        )
        .into_diagnostic()?;

        // With no `.gitignore` to look for, the walk never reads the
        // directories above the one it was given, so mado is the only reader
        // left to say what it could not parse.
        let assert = check_in(&proj, &["docs"]).assert();
        assert.success().stderr(indoc! {"
            ./.ignore: line 1: error parsing glob '{a': unclosed alternate group; missing '}' (maybe escape '{' with '[{]'?)
        "});
        Ok(())
    })
}

#[test]
fn check_leaves_a_broken_glob_the_walk_reads_to_the_walk() -> Result<()> {
    with_tree(
        &[(".gitignore", "{a\n"), ("docs/a.md", "# Fine\n")],
        |root| {
            // What the walk reads it reports for itself, and what it makes of a
            // file at a root rather than over one it keeps to itself. mado says
            // neither of those over again.
            let assert = check_in(root, &["."]).assert();
            assert.success().stderr("");
            Ok(())
        },
    )
}

#[test]
fn check_reports_a_broken_glob_in_a_parent_ignore_file_once() -> Result<()> {
    with_tree(
        &[
            ("proj/d1/.ignore", "{a\n"),
            ("proj/d1/docs/a.md", "# Fine\n"),
        ],
        |root| {
            let proj = root.join("proj");
            write(
                proj.join("mado.toml"),
                "[lint]\nrespect-gitignore = false\n",
            )
            .into_diagnostic()?;

            // Two names for one directory are two walks, each handed the file
            // above it under the name it walks by, and it is still one file
            // with one thing wrong with it.
            let output = check_in(&proj, &["./d1/docs", "d1/docs"])
                .output()
                .into_diagnostic()?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            let complaints = stderr
                .lines()
                .filter(|line| line.contains("error parsing glob"))
                .count();
            assert_eq!(complaints, 1);
            Ok(())
        },
    )
}

#[test]
fn check_does_not_read_the_current_directory_gitignore_for_a_path_above_it() -> Result<()> {
    with_tree(
        &[
            ("outer/above.md", "#Hello."),
            ("outer/proj/.gitignore", "above.md\n"),
            ("outer/proj/keep.md", "# Fine\n"),
        ],
        |root| {
            // `..` climbs out of the directory mado was started in, so that
            // directory's `.gitignore` has nothing to say about what is there.
            let assert = check_in(&root.join("outer/proj"), &[".."]).assert();
            assert.failure().stdout(indoc! {"
                ../above.md:1:1: MD018 No space after hash on atx style header
                ../above.md:1:1: MD041 First line in file should be a top level header
                ../above.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[test]
fn check_lints_a_file_it_is_handed_though_gitignore_lists_it() -> Result<()> {
    with_tree(
        &[(".gitignore", "ignored.md\n"), ("ignored.md", "#Hello.")],
        |root| {
            // Naming a file is asking for it, and the walk hands one back
            // without asking any ignore file about it.
            let assert = check_in(root, &["ignored.md"]).assert();
            assert.failure().stdout(indoc! {"
                ignored.md:1:1: MD018 No space after hash on atx style header
                ignored.md:1:1: MD041 First line in file should be a top level header
                ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[test]
fn check_does_not_read_a_gitignore_above_the_current_directory() -> Result<()> {
    with_tree(
        &[(".gitignore", "leaked.md\n"), ("proj/leaked.md", "#Hello.")],
        |root| {
            let assert = check_in(&root.join("proj"), &["."]).assert();
            assert.failure().stdout(indoc! {"
                ./leaked.md:1:1: MD018 No space after hash on atx style header
                ./leaked.md:1:1: MD041 First line in file should be a top level header
                ./leaked.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[test]
fn check_does_not_read_the_global_git_ignore_file() -> Result<()> {
    with_tree(
        &[
            ("home/.config/git/ignore", "globally-ignored.md\n"),
            ("proj/globally-ignored.md", "#Hello."),
        ],
        |root| {
            let home = root.join("home");
            let mut cmd = check_in(&root.join("proj"), &["."]);
            let assert = cmd
                .env("HOME", &home)
                .env("XDG_CONFIG_HOME", home.join(".config"))
                .assert();
            assert.failure().stdout(indoc! {"
                ./globally-ignored.md:1:1: MD018 No space after hash on atx style header
                ./globally-ignored.md:1:1: MD041 First line in file should be a top level header
                ./globally-ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}
