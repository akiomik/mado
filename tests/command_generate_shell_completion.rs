use assert_cmd::Command;
use assert_cmd::cargo_bin;
use indoc::indoc;

#[test]
fn generate_shell_completion_zsh() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.args(["generate-shell-completion", "zsh"]).assert();
    assert.success();
}

#[test]
fn generate_shell_completion_invalid() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.args(["generate-shell-completion", "foo"]).assert();
    assert
        .failure()
        .stderr(indoc! {"
            \u{1b}[1m\u{1b}[31merror:\u{1b}[0m invalid value \'\u{1b}[33mfoo\u{1b}[0m\' for \'\u{1b}[1m<SHELL>\u{1b}[0m\'
              [possible values: \u{1b}[32mbash\u{1b}[0m, \u{1b}[32melvish\u{1b}[0m, \u{1b}[32mfish\u{1b}[0m, \u{1b}[32mpowershell\u{1b}[0m, \u{1b}[32mzsh\u{1b}[0m]
            
            For more information, try \'\u{1b}[1m--help\u{1b}[0m\'.
        "});
}
