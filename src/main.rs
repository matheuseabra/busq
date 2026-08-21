use std::{
    env, fs,
    io::{self, IsTerminal},
    process::Command,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MISSING: &str = "—";

#[derive(Default)]
struct Options {
    color: bool,
    icons: bool,
    logo: Option<String>,
}

pub fn version_string() -> String {
    format!("minfetch {}", env!("CARGO_PKG_VERSION"))
}

fn main() {
    let (options, help, version) = parse_args(env::args().skip(1)).unwrap_or_else(|error| {
        eprintln!("minfetch: {error}\nTry 'minfetch --help'.");
        std::process::exit(2);
    });
    if version {
        println!("{}", version_string());
        return;
    }
    if help {
        println!(
            "{}\n\nUsage: minfetch [--color-no] [--icons off] [--logo PATH]",
            version_string()
        );
        return;
    }
    let logo = load_logo(options.logo.as_deref(), io::stdout().is_terminal());
    let (width, height) = terminal_size();
    print!(
        "{}",
        render(&rows(), logo.as_deref(), width, height, options.icons)
    );
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<(Options, bool, bool), String> {
    let mut options = Options {
        color: env::var_os("NO_COLOR").is_none(),
        icons: true,
        logo: None,
    };
    let mut help = false;
    let mut version = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            "--color-no" | "--no-color" => options.color = false,
            "--icons" => {
                options.icons = match args.next().as_deref() {
                    Some("on") => true,
                    Some("off") => false,
                    Some(value) => return Err(format!("invalid --icons value {value}")),
                    None => return Err("--icons needs on or off".into()),
                }
            }
            "--logo" => options.logo = Some(args.next().ok_or("--logo needs a path")?),
            value if value.starts_with('-') => return Err(format!("unknown option {value}")),
            value => return Err(format!("unexpected argument {value}")),
        }
    }
    let _ = options.color; // Phase 1 emits no ANSI, including when stdout is piped.
    Ok((options, help, version))
}

fn rows() -> Vec<(String, String)> {
    vec![
        (
            "hostname".into(),
            first_env(&["HOSTNAME", "COMPUTERNAME"])
                .unwrap_or_else(|| command("hostname").unwrap_or_else(|| MISSING.into())),
        ),
        (
            "user".into(),
            first_env(&["USER", "USERNAME"]).unwrap_or_else(|| MISSING.into()),
        ),
        ("os".into(), env::consts::OS.into()),
        (
            "shell".into(),
            env::var("SHELL").unwrap_or_else(|_| MISSING.into()),
        ),
        ("uptime".into(), uptime().unwrap_or_else(|| MISSING.into())),
        ("cpu".into(), cpu().unwrap_or_else(|| MISSING.into())),
        ("memory".into(), memory().unwrap_or_else(|| MISSING.into())),
        (
            "disk".into(),
            command("df -h /")
                .and_then(|v| v.lines().nth(1).map(str::to_owned))
                .unwrap_or_else(|| MISSING.into()),
        ),
        (
            "kernel".into(),
            command("uname -sr").unwrap_or_else(|| MISSING.into()),
        ),
        (
            "terminal".into(),
            env::var("TERM").unwrap_or_else(|_| MISSING.into()),
        ),
        (
            "desktop".into(),
            first_env(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"])
                .unwrap_or_else(|| MISSING.into()),
        ),
        ("temperature".into(), MISSING.into()),
        ("gpu".into(), MISSING.into()),
    ]
}

fn load_logo(path: Option<&str>, stdout_is_terminal: bool) -> Option<String> {
    path.filter(|_| stdout_is_terminal)
        .and_then(|path| fs::read_to_string(path).ok())
}

fn terminal_size() -> (usize, usize) {
    let width = env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(80);
    let height = env::var("LINES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24);
    (width, height)
}

fn render(
    rows: &[(String, String)],
    logo: Option<&str>,
    width: usize,
    height: usize,
    icons: bool,
) -> String {
    let labels: Vec<String> = rows
        .iter()
        .map(|(label, _)| if icons { icon(label) } else { label.clone() })
        .collect();
    let info_width = labels
        .iter()
        .map(|label| display_width(label) + 1)
        .max()
        .unwrap_or(1);
    let logo_lines: Vec<&str> = logo
        .map(|value| value.lines().collect())
        .unwrap_or_default();
    let logo_width = logo_lines
        .iter()
        .map(|line| display_width(line))
        .max()
        .unwrap_or(0);
    let mut lines = Vec::new();

    if !logo_lines.is_empty() && width >= logo_width + info_width + 4 {
        let rows_height = rows.len().max(logo_lines.len());
        for index in 0..rows_height {
            let left = logo_lines.get(index).copied().unwrap_or("");
            let right = rows
                .get(index)
                .map(|(label, value)| {
                    let label = if icons { icon(label) } else { label.clone() };
                    format!(
                        "{}{}",
                        pad_display(&label, info_width),
                        truncate(value, width.saturating_sub(info_width + 4))
                    )
                })
                .unwrap_or_default();
            lines.push(format!("{}  {right}", pad_display(left, logo_width)));
        }
    } else {
        if !logo_lines.is_empty() && logo_lines.len() + rows.len() <= height {
            lines.extend(logo_lines.iter().map(|line| (*line).to_owned()));
        }
        for (label, value) in rows {
            let label = if icons { icon(label) } else { label.clone() };
            let value = truncate(value, width.saturating_sub(info_width + 1));
            if width <= 30 {
                lines.push(truncate(&label, width));
                lines.push(truncate(value.as_str(), width));
            } else {
                lines.push(format!("{} {value}", pad_display(&label, info_width)));
            }
        }
    }
    lines.truncate(height);
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn truncate(value: &str, width: usize) -> String {
    if display_width(value) <= width {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_owned();
    }
    let mut result = String::new();
    let mut used = 0;
    for character in value.chars() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > width - 1 {
            break;
        }
        result.push(character);
        used += character_width;
    }
    result.push('…');
    result
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn pad_display(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        " ".repeat(width.saturating_sub(display_width(value)))
    )
}

fn icon(label: &str) -> String {
    let symbol = match label {
        "cpu" => "◈",
        "memory" => "▣",
        "disk" => "◫",
        _ => "•",
    };
    format!("{symbol} {label}")
}

fn first_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env::var(name).ok())
}

fn command(command: &str) -> Option<String> {
    let mut parts = command.split_whitespace();
    let output = Command::new(parts.next()?).args(parts).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().into())
}

fn uptime() -> Option<String> {
    if let Ok(info) = fs::read_to_string("/proc/uptime")
        && let Some(seconds) = info
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
    {
        return Some(format!(
            "{}d {}h {}m",
            seconds / 86400,
            seconds / 3600 % 24,
            seconds / 60 % 60
        ));
    }
    command("sysctl -n kern.boottime")
}

fn cpu() -> Option<String> {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo")
        && let Some(cpu) = parse_cpuinfo(&info)
    {
        return Some(cpu);
    }
    let model = command("sysctl -n machdep.cpu.brand_string")?;
    Some(format!("{model} ({} cores)", command("sysctl -n hw.ncpu")?))
}

fn parse_cpuinfo(info: &str) -> Option<String> {
    let model = info.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(key, _)| key.trim() == "model name")
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
    })?;
    let cores = info
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    Some(format!("{model} ({cores} cores)"))
}

fn memory() -> Option<String> {
    let info = fs::read_to_string("/proc/meminfo").ok()?;
    let total = mem_kib(&info, "MemTotal")?;
    let available = mem_kib(&info, "MemAvailable").unwrap_or(total);
    Some(format!(
        "{} / {} MiB",
        (total - available) / 1024,
        total / 1024
    ))
}

fn mem_kib(info: &str, key: &str) -> Option<u64> {
    info.lines().find_map(|line| {
        line.strip_prefix(key)?
            .trim_start_matches(':')
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use super::{load_logo, mem_kib, parse_args, parse_cpuinfo, render};

    #[test]
    fn parses_meminfo_values() {
        assert_eq!(mem_kib("MemTotal: 1024 kB", "MemTotal"), Some(1024));
    }

    #[test]
    fn parses_cpuinfo_fixture() {
        let fixture = "processor\t: 0\nmodel name\t: Test CPU\nprocessor\t: 1\n";
        assert_eq!(
            parse_cpuinfo(fixture).as_deref(),
            Some("Test CPU (2 cores)")
        );
    }

    #[test]
    fn piped_output_never_loads_a_logo() {
        assert!(load_logo(Some("/definitely/not/a/logo"), false).is_none());
    }

    #[test]
    fn flags_disable_icons_and_color() {
        let (options, help, version) = parse_args(
            ["--icons", "off", "--color-no"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap();
        assert!(!options.icons && !options.color && !help && !version);
    }

    #[test]
    fn version_contains_package_version() {
        assert!(super::version_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn layout_stacks_logo_in_a_narrow_pane() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(render(&rows, Some("/\\"), 8, 4, false), "/\\\nos\nmac…\n");
    }

    #[test]
    fn layout_uses_single_column_at_thirty() {
        let rows = vec![("os".into(), "a-long-value".into())];
        assert_eq!(render(&rows, None, 30, 4, false), "os\na-long-value\n");
    }

    #[test]
    fn layout_truncates_to_height() {
        let rows = vec![("one".into(), "1".into()), ("two".into(), "2".into())];
        assert_eq!(render(&rows, None, 80, 1, false), "one  1\n");
    }

    #[test]
    fn truncation_respects_wide_glyphs() {
        assert_eq!(super::truncate("猫猫", 3), "猫…");
    }
}
