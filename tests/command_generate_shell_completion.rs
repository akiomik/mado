use assert_cmd::Command;
use indoc::indoc;
use miette::{IntoDiagnostic as _, Result};

#[test]
fn generate_shell_completion_zsh() -> Result<()> {
    let mut cmd = Command::cargo_bin("mado").into_diagnostic()?;
    let assert = cmd.args(["generate-shell-completion", "zsh"]).assert();
    assert.success();
    Ok(())
}

#[test]
fn generate_shell_completion_invalid() -> Result<()> {
    let mut cmd = Command::cargo_bin("mado").into_diagnostic()?;
    let assert = cmd.args(["generate-shell-completion", "foo"]).assert();
    assert
        .failure()
        .stderr(indoc! {"
            \u{1b}[1m\u{1b}[31merror:\u{1b}[0m invalid value \'\u{1b}[33mfoo\u{1b}[0m\' for \'\u{1b}[1m<SHELL>\u{1b}[0m\'
              [possible values: \u{1b}[32mbash\u{1b}[0m, \u{1b}[32melvish\u{1b}[0m, \u{1b}[32mfish\u{1b}[0m, \u{1b}[32mpowershell\u{1b}[0m, \u{1b}[32mzsh\u{1b}[0m]
            
            For more information, try \'\u{1b}[1m--help\u{1b}[0m\'.
        "});
    Ok(())
}
