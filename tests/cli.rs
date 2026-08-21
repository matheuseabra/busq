use std::process::Command;

#[test]
fn piped_output_is_plain_and_has_no_logo() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--logo", "/definitely/not/a/logo"])
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(!stdout.contains("\x1b["));
    assert!(stdout.contains("hostname"));
}
