use std::fs::File;
use std::io::Write as _;
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
    let assert = cmd.write_stdin("#Hello.").args(["check"]).assert();
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
        let assert = cmd.write_stdin("").args(["check", path_str]).assert();
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
fn check_stdin_with_file() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd
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
fn check_exclusion() -> Result<()> {
    with_tmp_file("test.md", "#Hello.", |path| {
        let mut cmd = Command::new(cargo_bin!("mado"));
        let path_str = path.to_str().wrap_err("failed to convert string")?;
        let assert = cmd.args(["check", path_str, "--exclude", "*.md"]).assert();
        assert.success().stdout("All checks passed!\n");
        Ok(())
    })
}
