#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    env, fs,
    io::{self, IsTerminal},
    path::PathBuf,
    process::Command,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MISSING: &str = "—";
const DEFAULT_LOGO: &str = "╭─╮\n│·│\n╰─╯";
const ANSI_LABEL: &str = "\x1b[36m";
const ANSI_RESET: &str = "\x1b[0m";
type FetchResult = Result<String, String>;
type Snapshot = ((usize, usize), Vec<(String, String)>, Vec<String>);

#[cfg(unix)]
static RESIZED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Theme {
    Subtle,
    Mono,
}

#[derive(Default)]
struct Config {
    color: Option<ColorMode>,
    icons: Option<bool>,
    logo: Option<String>,
    rows: Option<Vec<String>>,
    theme: Option<Theme>,
}

struct Options {
    color: ColorMode,
    icons: bool,
    logo: Option<String>,
    rows: Option<Vec<String>>,
    theme: Theme,
    no_terminator: bool,
    no_term: bool,
    verbose: bool,
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
            "{}\n\nUsage: minfetch [--config PATH] [--color auto|always|never] [--theme subtle|mono] [--no-term] [--no-terminator] [--verbose] [--icons on|off] [--logo none|auto|PATH]",
            version_string()
        );
        return;
    }
    let stdout_is_terminal = io::stdout().is_terminal();
    let logo = load_logo(options.logo.as_deref(), stdout_is_terminal);
    let ((width, height), fetched_rows, errors) =
        fetch_snapshot(options.no_term, options.rows.as_deref());
    if options.verbose {
        for error in errors {
            eprintln!("  ↳ {error}");
        }
    }
    let output = render_with_color(
        &fetched_rows,
        logo.as_deref(),
        width,
        height,
        options.icons,
        options
            .theme
            .color_enabled(options.color.enabled(stdout_is_terminal)),
    );
    if options.no_terminator {
        print!("{}", output.strip_suffix('\n').unwrap_or(&output));
    } else {
        print!("{output}");
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<(Options, bool, bool), String> {
    let args = args.into_iter().collect::<Vec<_>>();
    let config_path = args
        .windows(2)
        .find(|pair| pair[0] == "--config")
        .map(|pair| pair[1].as_str());
    let config = load_config(config_path)?;
    let no_color = env::var_os("NO_COLOR").is_some();
    let mut options = Options {
        color: if no_color {
            ColorMode::Never
        } else {
            config.color.unwrap_or(ColorMode::Auto)
        },
        icons: config.icons.unwrap_or(true),
        logo: config.logo,
        rows: config.rows,
        theme: config.theme.unwrap_or(Theme::Subtle),
        no_terminator: false,
        no_term: false,
        verbose: false,
    };
    let mut help = false;
    let mut version = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            "--color-no" | "--no-color" => options.color = ColorMode::Never,
            "--color" => {
                options.color = match args.next().as_deref() {
                    Some("auto") => ColorMode::Auto,
                    Some("always") => ColorMode::Always,
                    Some("never") => ColorMode::Never,
                    Some(value) => return Err(format!("invalid --color value {value}")),
                    None => return Err("--color needs auto, always, or never".into()),
                }
            }
            "--config" => {
                args.next().ok_or("--config needs a path")?;
            }
            "--theme" => {
                options.theme = match args.next().as_deref() {
                    Some("subtle") => Theme::Subtle,
                    Some("mono") => Theme::Mono,
                    Some(value) => return Err(format!("invalid --theme value {value}")),
                    None => return Err("--theme needs subtle or mono".into()),
                }
            }
            "--no-terminator" => options.no_terminator = true,
            "--no-term" => options.no_term = true,
            "--verbose" => options.verbose = true,
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
    Ok((options, help, version))
}

const ROW_NAMES: &[&str] = &[
    "hostname",
    "user",
    "os",
    "shell",
    "uptime",
    "cpu",
    "memory",
    "disk",
    "kernel",
    "terminal",
    "desktop",
    "temperature",
    "gpu",
];

fn load_config(explicit: Option<&str>) -> Result<Config, String> {
    let path = explicit
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("XDG_CONFIG_HOME").map(|path| PathBuf::from(path).join("minfetch/config"))
        })
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/minfetch/config"))
        });
    let Some(path) = path else {
        return Ok(Config::default());
    };
    match fs::read_to_string(&path) {
        Ok(content) => parse_config(&content),
        Err(error) if error.kind() == io::ErrorKind::NotFound && explicit.is_none() => {
            Ok(Config::default())
        }
        Err(error) => Err(format!("cannot read config {}: {error}", path.display())),
    }
}

fn parse_config(content: &str) -> Result<Config, String> {
    let mut config = Config::default();
    for (line_number, raw_line) in content.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .map(|(key, value)| (key.trim(), value.trim()))
            .ok_or_else(|| format!("config line {} needs key = value", line_number + 1))?;
        match key {
            "color" => config.color = Some(parse_color(value, line_number + 1)?),
            "icons" => config.icons = Some(parse_on_off(value, "icons", line_number + 1)?),
            "logo" => config.logo = Some(value.to_owned()),
            "rows" => config.rows = Some(parse_rows(value, line_number + 1)?),
            "theme" => {
                config.theme = Some(match value {
                    "subtle" => Theme::Subtle,
                    "mono" => Theme::Mono,
                    _ => return Err(format!("invalid theme on config line {}", line_number + 1)),
                });
            }
            _ => {
                return Err(format!(
                    "unknown config key `{key}` on line {}",
                    line_number + 1
                ));
            }
        }
    }
    Ok(config)
}

fn parse_color(value: &str, line_number: usize) -> Result<ColorMode, String> {
    match value {
        "auto" => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never" => Ok(ColorMode::Never),
        _ => Err(format!("invalid color on config line {line_number}")),
    }
}

fn parse_on_off(value: &str, key: &str, line_number: usize) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("invalid {key} on config line {line_number}")),
    }
}

fn parse_rows(value: &str, line_number: usize) -> Result<Vec<String>, String> {
    let rows = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if rows.is_empty() || rows.iter().any(|row| !ROW_NAMES.contains(&row.as_str())) {
        return Err(format!("invalid rows on config line {line_number}"));
    }
    Ok(rows)
}

impl ColorMode {
    fn enabled(self, stdout_is_terminal: bool) -> bool {
        stdout_is_terminal && self != Self::Never
    }
}

impl Theme {
    fn color_enabled(self, color_enabled: bool) -> bool {
        color_enabled && self != Self::Mono
    }
}

fn rows(no_term: bool, selected: Option<&[String]>) -> (Vec<(String, String)>, Vec<String>) {
    let mut errors = Vec::new();
    let mut rows = vec![
        fetched_row("hostname", hostname(), &mut errors),
        fetched_row(
            "user",
            environment_value(&["USER", "USERNAME"]),
            &mut errors,
        ),
        ("os".into(), env::consts::OS.into()),
        fetched_row("shell", environment_value(&["SHELL"]), &mut errors),
        fetched_row("uptime", uptime(), &mut errors),
        fetched_row("cpu", cpu(), &mut errors),
        fetched_row("memory", memory(), &mut errors),
        fetched_row("disk", disk(), &mut errors),
        fetched_row("kernel", command("uname -sr"), &mut errors),
        fetched_row("terminal", environment_value(&["TERM"]), &mut errors),
        fetched_row(
            "desktop",
            environment_value(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]),
            &mut errors,
        ),
        ("temperature".into(), MISSING.into()),
        ("gpu".into(), MISSING.into()),
    ];
    if no_term {
        rows.retain(|(label, _)| label != "uptime");
    }
    if let Some(selected) = selected {
        rows.retain(|(label, _)| selected.iter().any(|value| value == label));
    }
    (rows, errors)
}

fn fetched_row(label: &str, result: FetchResult, errors: &mut Vec<String>) -> (String, String) {
    match result {
        Ok(value) => (label.into(), value),
        Err(error) => {
            errors.push(format!("{label}: {error}"));
            (label.into(), MISSING.into())
        }
    }
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

fn fetch_snapshot(no_term: bool, selected: Option<&[String]>) -> Snapshot {
    #[cfg(unix)]
    let previous_handler = install_resize_handler();
    let initial_size = terminal_size();
    let fetched_rows = rows(no_term, selected);
    let snapshot = if resize_seen() {
        (terminal_size(), rows(no_term, selected))
    } else {
        (initial_size, fetched_rows)
    };
    #[cfg(unix)]
    restore_resize_handler(previous_handler);
    let ((width, height), (rows, errors)) = snapshot;
    ((width, height), rows, errors)
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

#[cfg(test)]
fn render(
    rows: &[(String, String)],
    logo: Option<&str>,
    width: usize,
    height: usize,
    icons: bool,
) -> String {
    render_with_color(rows, logo, width, height, icons, false)
}

fn render_with_color(
    rows: &[(String, String)],
    logo: Option<&str>,
    width: usize,
    height: usize,
    icons: bool,
    color: bool,
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
                        colorize(&pad_display(&label, info_width), color),
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
                lines.push(colorize(&truncate(&label, width), color));
                lines.push(truncate(value.as_str(), width));
            } else {
                lines.push(format!(
                    "{} {value}",
                    colorize(&pad_display(&label, info_width), color)
                ));
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

fn colorize(value: &str, color: bool) -> String {
    if color {
        format!("{ANSI_LABEL}{value}{ANSI_RESET}")
    } else {
        value.to_owned()
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
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.is_empty()))
}

fn hostname() -> FetchResult {
    first_env(&["HOSTNAME", "COMPUTERNAME"])
        .ok_or_else(|| "HOSTNAME/COMPUTERNAME is unset".to_owned())
        .or_else(|_| command("hostname"))
}

fn environment_value(names: &[&str]) -> FetchResult {
    first_env(names).ok_or_else(|| format!("{} is unset", names.join("/")))
}

fn command(command: &str) -> FetchResult {
    let mut parts = command.split_whitespace();
    let program = parts.next().ok_or_else(|| "empty command".to_owned())?;
    let output = Command::new(program)
        .args(parts)
        .output()
        .map_err(|error| format!("{command}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command}: exited with {}", output.status));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!value.is_empty())
        .then_some(value)
        .ok_or_else(|| format!("{command}: returned no output"))
}

fn disk() -> FetchResult {
    command("df -h /")?
        .lines()
        .nth(1)
        .map(str::to_owned)
        .ok_or_else(|| "df -h /: returned no filesystem row".into())
}

fn uptime() -> FetchResult {
    if let Ok(info) = fs::read_to_string("/proc/uptime")
        && let Some(seconds) = info
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
    {
        return Ok(format!(
            "{}d {}h {}m",
            seconds / 86400,
            seconds / 3600 % 24,
            seconds / 60 % 60
        ));
    }
    command("sysctl -n kern.boottime")
}

fn cpu() -> FetchResult {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo")
        && let Some(cpu) = parse_cpuinfo(&info)
    {
        return Ok(cpu);
    }
    let model = command("sysctl -n machdep.cpu.brand_string")?;
    let cores = command("sysctl -n hw.ncpu")?;
    Ok(format!("{model} ({cores} cores)"))
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

fn memory() -> FetchResult {
    if let Ok(info) = fs::read_to_string("/proc/meminfo")
        && let Some(total) = mem_kib(&info, "MemTotal")
    {
        let available = mem_kib(&info, "MemAvailable").unwrap_or(total);
        return Ok(format!(
            "{} / {} MiB",
            total.saturating_sub(available) / 1024,
            total / 1024
        ));
    }

    let total = command("sysctl -n hw.memsize")?
        .parse::<u64>()
        .map_err(|error| format!("hw.memsize: {error}"))?;
    let used = command("vm_stat")
        .ok()
        .and_then(|info| parse_vm_stat(&info, total))
        .unwrap_or(total);
    Ok(format!(
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
        ColorMode, DEFAULT_LOGO, Theme, load_logo, mem_kib, parse_args, parse_config,
        parse_cpuinfo, parse_vm_stat, render, render_with_color,
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
            [
                "--icons",
                "off",
                "--color",
                "never",
                "--no-terminator",
                "--no-term",
                "--verbose",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(options.color, ColorMode::Never);
        assert!(!options.icons && options.no_terminator && options.no_term && options.verbose);
        assert!(!help && !version);
    }

    #[test]
    fn color_modes_parse_and_never_disables_output() {
        let (options, _, _) =
            parse_args(["--color", "always"].into_iter().map(str::to_owned)).unwrap();
        assert_eq!(options.color, ColorMode::Always);
        assert!(!ColorMode::Never.enabled(true));
        assert!(!ColorMode::Always.enabled(false));
    }

    #[test]
    fn parses_config_defaults() {
        let config = parse_config(
            "color = never\nicons = off\nlogo = none\nrows = os, user\ntheme = mono\n",
        )
        .unwrap();

        assert_eq!(config.color, Some(ColorMode::Never));
        assert_eq!(config.icons, Some(false));
        assert_eq!(config.logo.as_deref(), Some("none"));
        assert_eq!(
            config.rows.as_deref(),
            Some(["os".into(), "user".into()].as_slice())
        );
        assert_eq!(config.theme, Some(Theme::Mono));
    }

    #[test]
    fn color_wraps_labels_without_changing_layout_width() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(
            render_with_color(&rows, None, 40, 4, false, true),
            "\x1b[36mos \x1b[0m macos\n"
        );
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

    #[test]
    fn failed_rows_stay_missing_and_collect_diagnostics() {
        let labels = [
            "hostname", "user", "shell", "uptime", "cpu", "memory", "disk", "kernel", "terminal",
            "desktop",
        ];
        let mut errors = Vec::new();
        let rows = labels
            .iter()
            .map(|label| super::fetched_row(label, Err("fixture failure".into()), &mut errors))
            .collect::<Vec<_>>();

        assert!(rows.iter().all(|(_, value)| value == super::MISSING));
        assert_eq!(errors.len(), labels.len());
        assert!(errors.iter().all(|error| error.contains("fixture failure")));
        assert!(super::render(&rows, None, 80, 20, false).contains("hostname  —"));
    }
}
