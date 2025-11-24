use assert_cmd::cargo::cargo_bin_cmd;

#[test]
fn cli_shows_help() {
    let mut cmd = cargo_bin_cmd!("cass");
    cmd.arg("--help").assert().success();
}
