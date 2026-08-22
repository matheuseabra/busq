use std::{fs, process::Command};

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
fn no_icons_alias_removes_row_symbols() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--no-icons")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("• "));
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

#[test]
fn config_defaults_apply_and_flags_override_them() {
    let path = std::env::temp_dir().join(format!("minfetch-config-{}", std::process::id()));
    fs::write(
        &path,
        "rows = os, user\nicons = off\nlogo = none\ntheme = mono\n",
    )
    .expect("write config");

    let configured = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--config", path.to_str().expect("config path")])
        .output()
        .expect("run configured minfetch");
    let overridden = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args([
            "--config",
            path.to_str().expect("config path"),
            "--icons",
            "on",
        ])
        .output()
        .expect("run overridden minfetch");
    fs::remove_file(path).expect("remove config");

    assert!(configured.status.success() && overridden.status.success());
    let configured_stdout = String::from_utf8_lossy(&configured.stdout);
    let overridden_stdout = String::from_utf8_lossy(&overridden.stdout);
    assert!(configured_stdout.contains("os") && configured_stdout.contains("user"));
    assert!(!configured_stdout.contains("hostname"));
    assert!(overridden_stdout.contains("• os"));
}

#[cfg(not(feature = "json"))]
#[test]
fn json_explains_the_feature_build_requirement() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--json")
        .output()
        .expect("run minfetch");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--features json"));
}
