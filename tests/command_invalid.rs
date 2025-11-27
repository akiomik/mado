use assert_cmd::Command;
use assert_cmd::cargo_bin;
use indoc::formatdoc;

#[test]
fn no_command() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.assert();
    assert.failure();
}

#[test]
fn unknown_command() {
    let mut cmd = Command::new(cargo_bin!("mado"));
    let assert = cmd.args(["foobar"]).assert();
    assert.failure().stderr(formatdoc! {"
        \u{1b}[1m\u{1b}[31merror:\u{1b}[0m unrecognized subcommand \'\u{1b}[33mfoobar\u{1b}[0m\'

        \u{1b}[1m\u{1b}[4mUsage:\u{1b}[0m \u{1b}[1mmado\u{1b}[0m [OPTIONS] <COMMAND>

        For more information, try \'\u{1b}[1m--help\u{1b}[0m\'.
    "});
}
