use std::{fs, process::Command};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[test]
fn piped_output_is_plain_and_has_no_logo() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .args(["--logo", "/definitely/not/a/logo"])
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(!stdout.contains("\x1b["));
    assert!(stdout.contains('@'));
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
        .env("COLORFGBG", "0;7")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(!output.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
}

#[test]
fn no_color_environment_stays_plain() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .env("NO_COLOR", "1")
        .env("COLORFGBG", "0;7")
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
fn nerd_icons_are_the_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\u{f17c} OS:") && !stdout.contains("◉ OS:"));
}

#[test]
fn help_and_version_include_the_current_version() {
    let help = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--help")
        .output()
        .expect("run help");
    let version = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--version")
        .output()
        .expect("run version");

    let version_text = String::from_utf8_lossy(&version.stdout);
    assert!(help.status.success() && version.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains(version_text.trim()));
}

#[test]
fn probe_reports_detection_without_logo_or_ansi() {
    let probe = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--probe")
        .output()
        .expect("run probe");
    let alias = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .arg("--debug-sysinfo")
        .output()
        .expect("run probe alias");

    assert!(probe.status.success() && alias.status.success());
    let stdout = String::from_utf8_lossy(&probe.stdout);
    assert!(stdout.contains("minfetch probe"));
    assert!(stdout.contains("platform:") && stdout.contains("architecture:"));
    assert!(stdout.contains("terminal_size:") && stdout.contains("rows:"));
    assert!(!stdout.contains("\x1b[") && !stdout.contains("╭─╮"));
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
            .any(|line| line.contains("Shell:") && line.ends_with('—'))
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_user_is_rendered_as_missing() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .env("USER", std::ffi::OsString::from_vec(vec![0xff]))
        .env_remove("USERNAME")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.starts_with("—@"))
    );
}

#[test]
fn unicode_identity_values_render_without_loss() {
    let output = Command::new(env!("CARGO_BIN_EXE_minfetch"))
        .env("HOSTNAME", "机器")
        .env("USER", "José")
        .output()
        .expect("run minfetch");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("机器") && stdout.contains("José"));
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
            .any(|line| line.contains("Shell:") && line.ends_with('—'))
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
    assert!(configured_stdout.contains("OS:") && configured_stdout.contains("User:"));
    assert!(!configured_stdout.contains("Hostname:"));
    assert!(overridden_stdout.contains("◉ OS:"));
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
