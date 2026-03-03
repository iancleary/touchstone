use std::process::Command;

#[test]
fn help_flag_prints_usage_and_exits_success() {
    let output = Command::new(env!("CARGO_BIN_EXE_touchstone"))
        .arg("--help")
        .output()
        .expect("failed to run touchstone --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("USAGE:"), "stdout: {stdout}");
    assert!(stdout.contains("touchstone cascade"), "stdout: {stdout}");
}

#[test]
fn version_flag_prints_version_and_exits_success() {
    let output = Command::new(env!("CARGO_BIN_EXE_touchstone"))
        .arg("--version")
        .output()
        .expect("failed to run touchstone --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("touchstone"), "stdout: {stdout}");
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "stdout: {stdout}");
}

#[test]
fn no_args_exits_nonzero_and_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_touchstone"))
        .output()
        .expect("failed to run touchstone without args");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Problem parsing arguments"), "stdout: {stdout}");
    assert!(stdout.contains("USAGE:"), "stdout: {stdout}");
}
