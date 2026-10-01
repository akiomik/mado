use std::fs::File;
use std::fs::create_dir_all;
use std::fs::write;
use std::io::Write as _;
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

fn with_tree<F>(entries: &[(&str, &str)], f: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let tmp_dir = tempdir().into_diagnostic()?;
    let root = tmp_dir.path();
    for (path, content) in entries {
        let path = root.join(path);
        if let Some(dir) = path.parent() {
            create_dir_all(dir).into_diagnostic()?;
        }
        write(&path, content).into_diagnostic()?;
    }

    f(root)?;

    tmp_dir.close().into_diagnostic()
}

fn check_in(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(cargo_bin!("mado"));
    cmd.current_dir(dir)
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .arg("check")
        .args(args);
    cmd
}

fn policy(root: &Path, value: &str) -> Result<()> {
    write(
        root.join("mado.toml"),
        formatdoc! {"
            [lint]
            respect-gitignore = \"{value}\"
        "},
    )
    .into_diagnostic()
}

/// A tree whose `.gitignore` lists a directory mado would otherwise report on:
/// what #141 ran into when the same tree arrived without its Git metadata.
const IGNORED_BUILD_OUTPUT: &[(&str, &str)] = &[
    (".gitignore", "target/\n"),
    ("target/generated.md", "#Hello."),
];

const REPORTED: &str = indoc! {"
    ./target/generated.md:1:1: MD018 No space after hash on atx style header
    ./target/generated.md:1:1: MD041 First line in file should be a top level header
    ./target/generated.md:1:1: MD047 File should end with a single newline character

    Found 3 errors.
"};

#[test]
fn check_reads_gitignore_in_a_repository() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        create_dir_all(root.join(".git")).into_diagnostic()?;

        let assert = check_in(root, &["."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_does_not_read_gitignore_without_git_metadata() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        // The default is what Git does, and Git does nothing here.
        let assert = check_in(root, &["."]).assert();
        assert.failure().stdout(REPORTED);
        Ok(())
    })
}

#[test]
fn check_reads_gitignore_without_git_metadata_when_asked() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        policy(root, "always")?;

        let assert = check_in(root, &["."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_reads_no_gitignore_when_told_never() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        create_dir_all(root.join(".git")).into_diagnostic()?;
        policy(root, "never")?;

        let assert = check_in(root, &["."]).assert();
        assert.failure().stdout(REPORTED);
        Ok(())
    })
}

#[test]
fn check_reads_a_git_file_marker_as_a_repository() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        // A worktree and a submodule carry `.git` as a file naming the real
        // directory, and it marks a repository just as the directory does.
        write(root.join(".git"), "gitdir: /elsewhere/.git/worktrees/w\n").into_diagnostic()?;

        let assert = check_in(root, &["."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_reads_a_jj_directory_as_a_repository() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        // The ignore crate stops at `.jj` as well as `.git`, so a Jujutsu
        // repository is a repository for this setting too.
        create_dir_all(root.join(".jj")).into_diagnostic()?;

        let assert = check_in(root, &["."]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}

#[test]
fn check_reads_a_nested_gitignore_where_it_sits() -> Result<()> {
    with_tree(
        &[
            (".gitignore", "root-ignored.md\n"),
            ("docs/.gitignore", "nested-ignored.md\n"),
            ("root-ignored.md", "#Hello."),
            ("docs/nested-ignored.md", "#Hello."),
            ("docs/kept.md", "# Kept\n"),
        ],
        |root| {
            policy(root, "always")?;

            let assert = check_in(root, &["."]).assert();
            assert.success().stdout("All checks passed!\n");
            Ok(())
        },
    )
}

#[test]
fn check_reads_a_gitignore_above_the_tree_when_told_always() -> Result<()> {
    with_tree(
        &[
            (".gitignore", "generated.md\n"),
            ("proj/generated.md", "#Hello."),
        ],
        |root| {
            let proj = root.join("proj");
            policy(&proj, "always")?;

            // Without a repository to stop at there is nothing to stop at:
            // `.gitignore` files apply however far above the tree they sit.
            // This is the `ignore` crate's rule, and `always` says so.
            let assert = check_in(&proj, &["."]).assert();
            assert.success().stdout("All checks passed!\n");
            Ok(())
        },
    )
}

#[test]
fn check_reads_a_gitignore_above_a_repository_when_told_always() -> Result<()> {
    with_tree(
        &[
            (".gitignore", "generated.md\n"),
            ("proj/generated.md", "#Hello."),
        ],
        |root| {
            let proj = root.join("proj");
            create_dir_all(proj.join(".git")).into_diagnostic()?;
            policy(&proj, "always")?;

            // `always` stops mado looking for a repository at all, so the
            // boundary Git keeps at a repository root is not kept here either.
            // This tree has one, and the file above it still applies.
            let assert = check_in(&proj, &["."]).assert();
            assert.success().stdout("All checks passed!\n");
            Ok(())
        },
    )
}

#[test]
fn check_does_not_stop_at_a_repository_below_the_path_when_told_always() -> Result<()> {
    with_tree(
        &[(".gitignore", "lib/\n"), ("vendor/lib/inner.md", "#Hello.")],
        |root| {
            create_dir_all(root.join("vendor/.git")).into_diagnostic()?;
            policy(root, "always")?;

            // Git would apply nothing from outside the repository holding this
            // file. `always` turns the marker off for the whole walk, so the
            // file is excluded here. See #440.
            let assert = check_in(root, &["."]).assert();
            assert.success().stdout("All checks passed!\n");
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
            let proj = root.join("proj");
            create_dir_all(proj.join(".git")).into_diagnostic()?;

            // It does not travel with the tree, so reading it would have the
            // same source lint differently on a different machine.
            let assert = check_in(&proj, &["."])
                .env("HOME", root.join("home"))
                .env("XDG_CONFIG_HOME", root.join("home/.config"))
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

#[test]
fn check_does_not_read_the_repository_exclude_file() -> Result<()> {
    with_tree(
        &[
            (".git/info/exclude", "locally-ignored.md\n"),
            ("locally-ignored.md", "#Hello."),
        ],
        |root| {
            // It lives in the Git metadata rather than the tree, so it does not
            // travel with a source archive either.
            let assert = check_in(root, &["."]).assert();
            assert.failure().stdout(indoc! {"
                ./locally-ignored.md:1:1: MD018 No space after hash on atx style header
                ./locally-ignored.md:1:1: MD041 First line in file should be a top level header
                ./locally-ignored.md:1:1: MD047 File should end with a single newline character

                Found 3 errors.
            "});
            Ok(())
        },
    )
}

#[test]
fn check_respects_ignore_files_whatever_gitignore_is_told() -> Result<()> {
    with_tree(
        &[(".ignore", "ignored.md\n"), ("ignored.md", "#Hello.")],
        |root| {
            policy(root, "never")?;

            // The two options are independent: `.ignore` is mado's own, and
            // nothing about `.gitignore` decides whether it is read.
            let assert = check_in(root, &["."]).assert();
            assert.success().stdout("All checks passed!\n");
            Ok(())
        },
    )
}

#[test]
fn check_rejects_the_boolean_spelling_of_the_policy() -> Result<()> {
    with_tree(IGNORED_BUILD_OUTPUT, |root| {
        write(root.join("mado.toml"), "[lint]\nrespect-gitignore = true\n").into_diagnostic()?;

        // 0.3.x wrote this as a boolean. Failing to load is the migration
        // notice, and it says which policy the boolean written meant rather
        // than leaving that to be looked up.
        let output = check_in(root, &["."]).output().into_diagnostic()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(stderr.contains("respect-gitignore"), "{stderr}");
        assert!(stderr.contains("repository-only"), "{stderr}");
        Ok(())
    })
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
