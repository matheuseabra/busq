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

#[test]
fn no_terminator_omits_only_the_final_newline() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--no-terminator"])
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(!output.stdout.ends_with(b"\n"));
}

#[test]
fn forced_color_stays_plain_when_stdout_is_piped() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--color", "always"])
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(!output.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
}

#[test]
fn no_term_omits_the_uptime_row() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--no-term"])
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("uptime"));
}

#[test]
fn empty_shell_is_rendered_as_missing() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .env("SHELL", "")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.contains("shell") && line.ends_with('—'))
    );
}

#[test]
fn verbose_reports_missing_environment_rows_on_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .env_remove("SHELL")
        .env_remove("TERM")
        .env_remove("USER")
        .env_remove("USERNAME")
        .env_remove("XDG_CURRENT_DESKTOP")
        .env_remove("DESKTOP_SESSION")
        .arg("--verbose")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("↳ shell:"));
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.contains("shell") && line.ends_with('—'))
    );
}
