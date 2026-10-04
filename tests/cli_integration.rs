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
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "stdout: {stdout}"
    );
}

#[test]
fn no_args_exits_nonzero_and_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_touchstone"))
        .output()
        .expect("failed to run touchstone without args");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Problem parsing arguments"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("USAGE:"), "stdout: {stdout}");
}

#[test]
fn cascade_rejects_one_port_network_without_panicking() {
    assert_cascade_failure(
        "one_port",
        "s1p",
        "# GHz S RI R 50\n1 0 0\n",
        "# GHz S RI R 50\n1 0 0 1 0 1 0 0 0\n",
        "Found ranks 1 and 2",
    );
}

#[test]
fn cascade_rejects_mismatched_impedances_without_panicking() {
    assert_cascade_failure(
        "impedance",
        "s2p",
        "# GHz S RI R 50\n1 0 0 1 0 1 0 0 0\n",
        "# GHz S RI R 75\n1 0 0 1 0 1 0 0 0\n",
        "different reference impedances: 50 and 75",
    );
}

#[test]
fn cascade_rejects_mismatched_frequencies_without_panicking() {
    assert_cascade_failure(
        "frequency",
        "s2p",
        "# GHz S RI R 50\n1 0 0 1 0 1 0 0 0\n",
        "# GHz S RI R 50\n2 0 0 1 0 1 0 0 0\n",
        "different frequency grids",
    );
}

fn assert_cascade_failure(
    case: &str,
    first_extension: &str,
    first_contents: &str,
    second_contents: &str,
    expected_cause: &str,
) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "touchstone_cli_{case}_{}_{nanos}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let first_path = directory.join(format!("first.{first_extension}"));
    let second_path = directory.join("second.s2p");
    std::fs::write(&first_path, first_contents).unwrap();
    std::fs::write(&second_path, second_contents).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_touchstone"))
        .arg("cascade")
        .arg(&first_path)
        .arg(&second_path)
        .env("RUST_LOG", "error")
        .output()
        .expect("failed to run touchstone cascade");
    let entries = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    std::fs::remove_dir_all(&directory).unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stdout.contains("Failed to cascade networks"),
        "stdout: {stdout}"
    );
    assert!(
        !stderr.contains("panicked") && !stdout.contains("panicked"),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    if cfg!(feature = "cli") {
        assert!(
            stdout.contains(expected_cause) || stderr.contains(expected_cause),
            "stdout: {stdout}\nstderr: {stderr}"
        );
    }
    assert_eq!(entries.len(), 2, "unexpected output files: {entries:?}");
    assert!(entries.contains(&first_path));
    assert!(entries.contains(&second_path));
}
