use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn cli_shows_help() {
    let mut cmd = cargo_bin_cmd!("coding-agent-search");
    cmd.arg("--help").assert().success();
}
