#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    env, fs,
    io::{self, IsTerminal},
    process::Command,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MISSING: &str = "—";
const DEFAULT_LOGO: &str = "╭─╮\n│·│\n╰─╯";

#[cfg(unix)]
static RESIZED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct Options {
    color: bool,
    icons: bool,
    logo: Option<String>,
    no_terminator: bool,
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
            "{}\n\nUsage: minfetch [--color-no] [--no-terminator] [--icons on|off] [--logo none|auto|PATH]",
            version_string()
        );
        return;
    }
    let stdout_is_terminal = io::stdout().is_terminal();
    let logo = load_logo(options.logo.as_deref(), stdout_is_terminal);
    let ((width, height), fetched_rows) = fetch_snapshot();
    let output = render(&fetched_rows, logo.as_deref(), width, height, options.icons);
    if options.no_terminator {
        print!("{}", output.strip_suffix('\n').unwrap_or(&output));
    } else {
        print!("{output}");
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<(Options, bool, bool), String> {
    let mut options = Options {
        color: env::var_os("NO_COLOR").is_none(),
        icons: true,
        logo: None,
        no_terminator: false,
    };
    let mut help = false;
    let mut version = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            "--color-no" | "--no-color" => options.color = false,
            "--no-terminator" => options.no_terminator = true,
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
    if !stdout_is_terminal {
        return None;
    }
    match path {
        Some("none") => None,
        Some("auto") | None => Some(DEFAULT_LOGO.into()),
        Some(path) => fs::read_to_string(path).ok(),
    }
}

fn fetch_snapshot() -> ((usize, usize), Vec<(String, String)>) {
    #[cfg(unix)]
    let previous_handler = install_resize_handler();
    let initial_size = terminal_size();
    let fetched_rows = rows();
    let snapshot = if resize_seen() {
        (terminal_size(), rows())
    } else {
        (initial_size, fetched_rows)
    };
    #[cfg(unix)]
    restore_resize_handler(previous_handler);
    snapshot
}

#[cfg(unix)]
extern "C" fn handle_resize(_: libc::c_int) {
    RESIZED.store(true, Ordering::Relaxed);
}

#[cfg(unix)]
fn install_resize_handler() -> Option<libc::sighandler_t> {
    // SAFETY: the handler only performs an atomic store and has C ABI.
    let handler = handle_resize as *const () as libc::sighandler_t;
    let previous = unsafe { libc::signal(libc::SIGWINCH, handler) };
    (previous != libc::SIG_ERR).then_some(previous)
}

#[cfg(unix)]
fn restore_resize_handler(previous: Option<libc::sighandler_t>) {
    if let Some(previous) = previous {
        // SAFETY: `previous` came from the same SIGWINCH handler slot.
        unsafe { libc::signal(libc::SIGWINCH, previous) };
    }
}

#[cfg(unix)]
fn resize_seen() -> bool {
    RESIZED.swap(false, Ordering::Relaxed)
}

#[cfg(not(unix))]
fn resize_seen() -> bool {
    false
}

fn terminal_size() -> (usize, usize) {
    #[cfg(unix)]
    if io::stdout().is_terminal() {
        let fd = io::stdout().as_raw_fd();
        let mut size = std::mem::MaybeUninit::<libc::winsize>::zeroed();
        // SAFETY: `size` points to writable memory for the kernel's winsize result;
        // the ioctl only writes that struct and does not retain the pointer.
        let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, size.as_mut_ptr()) };
        if result == 0 {
            // SAFETY: a successful TIOCGWINSZ call initialized the struct.
            let size = unsafe { size.assume_init() };
            if size.ws_col > 0 && size.ws_row > 0 {
                return (size.ws_col.into(), size.ws_row.into());
            }
        }
    }

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
    if let Ok(info) = fs::read_to_string("/proc/meminfo")
        && let Some(total) = mem_kib(&info, "MemTotal")
    {
        let available = mem_kib(&info, "MemAvailable").unwrap_or(total);
        return Some(format!(
            "{} / {} MiB",
            total.saturating_sub(available) / 1024,
            total / 1024
        ));
    }

    let total = command("sysctl -n hw.memsize")?.parse::<u64>().ok()?;
    let used = command("vm_stat")
        .and_then(|info| parse_vm_stat(&info, total))
        .unwrap_or(total);
    Some(format!(
        "{} / {} MiB",
        used / 1024 / 1024,
        total / 1024 / 1024
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

fn parse_vm_stat(info: &str, total_bytes: u64) -> Option<u64> {
    let page_size = info
        .lines()
        .find_map(|line| line.strip_prefix("Page size of "))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(4096);
    let available_pages = ["Pages free", "Pages inactive", "Pages speculative"]
        .into_iter()
        .map(|key| {
            info.lines()
                .find_map(|line| {
                    line.strip_prefix(key)?
                        .split_once(':')?
                        .1
                        .trim()
                        .trim_end_matches('.')
                        .parse::<u64>()
                        .ok()
                })
                .unwrap_or(0)
        })
        .sum::<u64>();
    Some(total_bytes.saturating_sub(available_pages.saturating_mul(page_size)))
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_LOGO, load_logo, mem_kib, parse_args, parse_cpuinfo, parse_vm_stat, render,
    };

    #[test]
    fn parses_meminfo_values() {
        assert_eq!(mem_kib("MemTotal: 1024 kB", "MemTotal"), Some(1024));
    }

    #[test]
    fn parses_vm_stat_fixture() {
        let fixture = "Page size of 4096 bytes\nPages free: 10.\nPages inactive: 20.\nPages speculative: 5.\n";
        assert_eq!(parse_vm_stat(fixture, 200 * 4096), Some(165 * 4096));
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
    fn logo_modes_select_the_neutral_default() {
        assert_eq!(load_logo(None, true).as_deref(), Some(DEFAULT_LOGO));
        assert!(load_logo(Some("none"), true).is_none());
        assert_eq!(load_logo(Some("auto"), true).as_deref(), Some(DEFAULT_LOGO));
    }

    #[test]
    fn flags_disable_icons_and_color() {
        let (options, help, version) = parse_args(
            ["--icons", "off", "--color-no", "--no-terminator"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap();
        assert!(!options.icons && !options.color && options.no_terminator && !help && !version);
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
    fn layout_uses_side_by_side_logo_when_wide() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(render(&rows, Some("/\\"), 20, 4, false), "/\\  os macos\n");
    }

    #[test]
    fn layout_uses_single_column_at_thirty() {
        let rows = vec![
            ("os".into(), "macos".into()),
            ("user".into(), "matheus".into()),
        ];
        assert_eq!(
            render(&rows, None, 30, 4, false),
            "os\nmacos\nuser\nmatheus\n"
        );
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
