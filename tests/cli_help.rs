use std::process::Command;

#[test]
fn cli_help_exits_successfully() {
    let exe = env!("CARGO_BIN_EXE_depdown");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("failed to run depdown --help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("depdown"));
}
