#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    env, fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MISSING: &str = "—";
const DEFAULT_LOGO: &str = "╭─╮\n│·│\n╰─╯";
const MACOS_LOGO: &str = "      .:'\n   __ :__\n  (______)\n   \\____/";
const LINUX_LOGO: &str =
    "   .--.\n  |o_o |\n  |:_/ |\n //   \\ \\\n(|     | )\n/'\\_   _/`\\\n\\___)=(___/";
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IconMode {
    Unicode,
    Nerd,
    Off,
}

#[derive(Default)]
struct Config {
    color: Option<ColorMode>,
    icons: Option<IconMode>,
    logo: Option<String>,
    rows: Option<Vec<String>>,
    theme: Option<Theme>,
}

struct Options {
    color: ColorMode,
    icons: IconMode,
    logo: Option<String>,
    rows: Option<Vec<String>>,
    theme: Option<Theme>,
    no_terminator: bool,
    no_term: bool,
    verbose: bool,
    probe: bool,
    #[cfg(feature = "json")]
    json: bool,
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
            "{}\n\nUsage: minfetch{} [--config PATH] [--color auto|always|never] [--theme subtle|mono] [--probe] [--no-term] [--no-terminator] [--verbose] [--icons on|off|nerd] [--logo [none|auto|PATH]]",
            version_string(),
            if cfg!(feature = "json") {
                " [--json]"
            } else {
                ""
            }
        );
        return;
    }
    let stdout_is_terminal = io::stdout().is_terminal();
    let theme = options
        .theme
        .unwrap_or_else(|| detect_theme(stdout_is_terminal, env::var("COLORFGBG").ok().as_deref()));
    let ((width, height), fetched_rows, errors) =
        fetch_snapshot(options.no_term, options.rows.as_deref());
    if options.probe {
        let output = render_probe(&fetched_rows, &errors, width, height, stdout_is_terminal);
        if options.no_terminator {
            print!("{}", output.strip_suffix('\n').unwrap_or(&output));
        } else {
            print!("{output}");
        }
        return;
    }
    if options.verbose {
        for error in errors {
            eprintln!("  ↳ {error}");
        }
    }
    let logo = load_logo(options.logo.as_deref(), stdout_is_terminal);
    #[cfg(feature = "json")]
    if options.json {
        let output = render_json(&fetched_rows);
        if options.no_terminator {
            print!("{}", output.strip_suffix('\n').unwrap_or(&output));
        } else {
            print!("{output}");
        }
        return;
    }
    let output = render_with_color(
        &fetched_rows,
        logo.as_deref(),
        width,
        height,
        options.icons,
        theme.color_enabled(options.color.enabled(stdout_is_terminal)),
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
        icons: config.icons.unwrap_or(IconMode::Off),
        logo: config.logo,
        rows: config.rows,
        theme: config.theme,
        no_terminator: false,
        no_term: false,
        verbose: false,
        probe: false,
        #[cfg(feature = "json")]
        json: false,
    };
    let mut help = false;
    let mut version = false;
    let mut args = args.into_iter().peekable();
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
                options.theme = Some(match args.next().as_deref() {
                    Some("subtle") => Theme::Subtle,
                    Some("mono") => Theme::Mono,
                    Some(value) => return Err(format!("invalid --theme value {value}")),
                    None => return Err("--theme needs subtle or mono".into()),
                })
            }
            "--no-terminator" => options.no_terminator = true,
            "--no-term" => options.no_term = true,
            "--verbose" => options.verbose = true,
            "--probe" | "--debug-sysinfo" => options.probe = true,
            "--no-icons" => options.icons = IconMode::Off,
            "--json" => {
                #[cfg(feature = "json")]
                {
                    options.json = true;
                }
                #[cfg(not(feature = "json"))]
                return Err("--json requires rebuilding with --features json".into());
            }
            "--icons" => {
                options.icons = match args.next().as_deref() {
                    Some("on") => IconMode::Unicode,
                    Some("off") => IconMode::Off,
                    Some("nerd") => IconMode::Nerd,
                    Some(value) => return Err(format!("invalid --icons value {value}")),
                    None => return Err("--icons needs on, off, or nerd".into()),
                }
            }
            "--logo" => {
                options.logo = Some(match args.peek() {
                    Some(value) if !value.starts_with('-') => {
                        args.next().expect("peeked argument exists")
                    }
                    _ => "auto".into(),
                })
            }
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
const DEFAULT_ROWS: &[&str] = &[
    "hostname", "user", "os", "kernel", "uptime", "shell", "terminal", "cpu", "gpu", "memory",
    "disk",
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
            "icons" => config.icons = Some(parse_icons(value, line_number + 1)?),
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

fn parse_icons(value: &str, line_number: usize) -> Result<IconMode, String> {
    match value {
        "on" => Ok(IconMode::Unicode),
        "off" => Ok(IconMode::Off),
        "nerd" => Ok(IconMode::Nerd),
        _ => Err(format!("invalid icons on config line {line_number}")),
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

fn detect_theme(stdout_is_terminal: bool, colorfgbg: Option<&str>) -> Theme {
    if !stdout_is_terminal {
        return Theme::Subtle;
    }
    colorfgbg.and_then(parse_colorfgbg).unwrap_or(Theme::Subtle)
}

fn parse_colorfgbg(value: &str) -> Option<Theme> {
    let background = value.rsplit(';').next()?.parse::<u8>().ok()?;
    match background {
        0..=6 | 8 => Some(Theme::Subtle),
        7 | 15 => Some(Theme::Mono),
        _ => None,
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
        fetched_row("kernel", command("uname -sr"), &mut errors),
        fetched_row("uptime", uptime(), &mut errors),
        fetched_row("shell", environment_value(&["SHELL"]), &mut errors),
        fetched_row("terminal", environment_value(&["TERM"]), &mut errors),
        fetched_row("cpu", cpu(), &mut errors),
        fetched_row("gpu", gpu(), &mut errors),
        fetched_row("memory", memory(), &mut errors),
        fetched_row("disk", disk(), &mut errors),
        fetched_row(
            "desktop",
            environment_value(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]),
            &mut errors,
        ),
        fetched_row("temperature", temperature(), &mut errors),
    ];
    if no_term {
        rows.retain(|(label, _)| label != "uptime");
    }
    let selected = selected
        .map(|rows| rows.iter().map(String::as_str).collect())
        .unwrap_or_else(|| DEFAULT_ROWS.to_vec());
    rows.retain(|(label, _)| selected.iter().any(|value| *value == label));
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
        Some("auto") => Some(os_logo().into()),
        None => None,
        Some(path) => fs::read_to_string(path).ok(),
    }
}

fn os_logo() -> &'static str {
    match env::consts::OS {
        "macos" => MACOS_LOGO,
        "linux" => LINUX_LOGO,
        _ => DEFAULT_LOGO,
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
    icons: IconMode,
) -> String {
    render_with_color(rows, logo, width, height, icons, false)
}

fn render_with_color(
    rows: &[(String, String)],
    logo: Option<&str>,
    width: usize,
    height: usize,
    icons: IconMode,
    color: bool,
) -> String {
    let identity = rows
        .iter()
        .find(|(label, _)| label == "user")
        .zip(rows.iter().find(|(label, _)| label == "hostname"))
        .map(|(user, hostname)| format!("{}@{}", user.1, hostname.1));
    let rows = if identity.is_some() {
        rows.iter()
            .filter(|(label, _)| label != "user" && label != "hostname")
            .collect::<Vec<_>>()
    } else {
        rows.iter().collect()
    };
    let labels: Vec<String> = rows
        .iter()
        .map(|(label, _)| label_text(label, icons))
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

    let header = identity.map(|value| truncate(&value, width));
    if !logo_lines.is_empty() && width >= logo_width + info_width + 4 {
        let rows_height = (rows.len() + header.iter().count() * 2).max(logo_lines.len());
        for index in 0..rows_height {
            let left = logo_lines.get(index).copied().unwrap_or("");
            let right = if let Some(header) = header.as_deref() {
                if index == 0 {
                    colorize(header, color)
                } else if index == 1 {
                    "-".repeat(display_width(header))
                } else {
                    rows.get(index - 2)
                        .map(|(label, value)| {
                            let label = label_text(label, icons);
                            format!(
                                "{} {}",
                                colorize(&pad_display(&label, info_width), color),
                                truncate(value, width.saturating_sub(info_width + 4))
                            )
                        })
                        .unwrap_or_default()
                }
            } else {
                rows.get(index)
                    .map(|(label, value)| {
                        let label = label_text(label, icons);
                        format!(
                            "{} {}",
                            colorize(&pad_display(&label, info_width), color),
                            truncate(value, width.saturating_sub(info_width + 4))
                        )
                    })
                    .unwrap_or_default()
            };
            lines.push(format!("{}  {right}", pad_display(left, logo_width)));
        }
    } else {
        if !logo_lines.is_empty() && logo_lines.len() + rows.len() <= height {
            lines.extend(logo_lines.iter().map(|line| (*line).to_owned()));
        }
        if let Some(header) = header.as_deref() {
            lines.push(colorize(header, color));
            lines.push("-".repeat(display_width(header)));
        }
        for (label, value) in rows {
            let label = label_text(label, icons);
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

fn render_probe(
    rows: &[(String, String)],
    errors: &[String],
    width: usize,
    height: usize,
    stdout_is_terminal: bool,
) -> String {
    let mut output = format!(
        "minfetch probe\nplatform: {}\narchitecture: {}\nstdout_tty: {}\nterminal_size: {}x{}\nrows:\n",
        env::consts::OS,
        env::consts::ARCH,
        stdout_is_terminal,
        width,
        height
    );
    for (label, value) in rows {
        output.push_str(&format!("{label}: {value}\n"));
    }
    if !errors.is_empty() {
        output.push_str("errors:\n");
        for error in errors {
            output.push_str(&format!("  {error}\n"));
        }
    }
    output
}

#[cfg(feature = "json")]
fn render_json(rows: &[(String, String)]) -> String {
    let rows = rows
        .iter()
        .map(|(label, value)| {
            format!(
                "{{\"label\":\"{}\",\"value\":\"{}\"}}",
                json_escape(label),
                json_escape(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"rows\":[{rows}]}}\n")
}

#[cfg(feature = "json")]
fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
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

fn icon(label: &str, nerd: bool) -> String {
    let symbol = if nerd {
        match label {
            "hostname" => "\u{f0ac}",
            "user" => "\u{f007}",
            "os" => "\u{f17c}",
            "kernel" => "\u{f0c9}",
            "uptime" => "\u{f017}",
            "shell" | "terminal" => "\u{f120}",
            "cpu" => "\u{f2db}",
            "gpu" | "desktop" => "\u{f108}",
            "memory" => "\u{f538}",
            "disk" => "\u{f0a0}",
            "temperature" => "\u{f2c9}",
            _ => "\u{f128}",
        }
    } else {
        match label {
            "os" => "◉",
            "kernel" => "◇",
            "uptime" => "◷",
            "shell" => "›",
            "terminal" => "▹",
            "cpu" => "◈",
            "gpu" => "◐",
            "memory" => "▣",
            "disk" => "◫",
            "desktop" => "▧",
            "temperature" => "◌",
            _ => "•",
        }
    };
    format!("{symbol} {}", label_text(label, IconMode::Off))
}

fn label_text(label: &str, icons: IconMode) -> String {
    match icons {
        IconMode::Unicode => return icon(label, false),
        IconMode::Nerd => return icon(label, true),
        IconMode::Off => {}
    }
    let label = match label {
        "os" => "OS",
        "cpu" => "CPU",
        "gpu" => "GPU",
        _ => label,
    };
    let mut characters = label.chars();
    match characters.next() {
        Some(first) => format!("{}{}:", first.to_ascii_uppercase(), characters.as_str()),
        None => String::new(),
    }
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
    let native = command("df -kP /").and_then(|output| {
        let fields = output
            .lines()
            .nth(1)
            .ok_or_else(|| "df -kP /: returned no filesystem row".to_owned())?
            .split_whitespace()
            .collect::<Vec<_>>();
        let total = fields
            .get(1)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| "df -kP /: invalid total".to_owned())?
            * 1024;
        let used = fields
            .get(2)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| "df -kP /: invalid used".to_owned())?
            * 1024;
        Ok(format_usage(used, total))
    });
    native.or_else(|error| sysinfo_disk().ok_or(error))
}

fn uptime() -> FetchResult {
    if let Ok(info) = fs::read_to_string("/proc/uptime")
        && let Some(seconds) = info
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
    {
        return Ok(format_uptime(seconds));
    }
    command("sysctl -n kern.boottime").and_then(|value| {
        parse_boottime(&value)
            .and_then(|boot| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()?
                    .as_secs()
                    .checked_sub(boot)
            })
            .map(format_uptime)
            .ok_or_else(|| "kern.boottime: invalid value".into())
    })
}

fn parse_boottime(value: &str) -> Option<u64> {
    value
        .split("sec = ")
        .nth(1)?
        .split(|character: char| !character.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds / 3_600 % 24;
    let minutes = seconds / 60 % 60;
    match (days, hours, minutes) {
        (0, 0, minutes) => format!("{minutes}m"),
        (0, hours, minutes) => format!("{hours}h {minutes}m"),
        (days, hours, minutes) => format!("{days}d {hours}h {minutes}m"),
    }
}

fn format_usage(used: u64, total: u64) -> String {
    let gib = 1024_f64.powi(3);
    let percent = used.saturating_mul(100).checked_div(total).unwrap_or(0);
    format!(
        "{:.2} GiB / {:.2} GiB ({percent}%)",
        used as f64 / gib,
        total as f64 / gib
    )
}

fn cpu() -> FetchResult {
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo")
        && let Some(cpu) = parse_cpuinfo(&info)
    {
        return Ok(cpu);
    }
    let native = (|| {
        let model = command("sysctl -n machdep.cpu.brand_string")?;
        let cores = command("sysctl -n hw.ncpu")?;
        Ok(format!("{model} ({cores} cores)"))
    })();
    native.or_else(|error| sysinfo_cpu().ok_or(error))
}

fn temperature() -> FetchResult {
    #[cfg(target_os = "linux")]
    {
        return linux_temperature(Path::new("/sys/class/thermal"));
    }
    #[cfg(target_os = "macos")]
    {
        return command("ioreg -r -c IOHWSensor -l").and_then(|info| {
            parse_ioreg_temperature(&info)
                .map(|value| format!("{value} °C"))
                .ok_or_else(|| "IOHWSensor reported no temperature".into())
        });
    }
    #[allow(unreachable_code)]
    Err("CPU temperature is unsupported on this platform".into())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn linux_temperature(root: &Path) -> FetchResult {
    let entries = fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let mut temperatures = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let value = fs::read_to_string(path.join("temp")).ok();
        let kind = fs::read_to_string(path.join("type")).unwrap_or_default();
        if let Some(value) = value.as_deref().and_then(parse_millidegrees) {
            temperatures.push((kind, value));
        }
    }
    temperatures
        .iter()
        .find(|(kind, _)| {
            let kind = kind.to_ascii_lowercase();
            kind.contains("cpu") || kind.contains("package")
        })
        .or_else(|| temperatures.first())
        .map(|(_, value)| format!("{value} °C"))
        .ok_or_else(|| "no readable thermal zone".into())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_millidegrees(value: &str) -> Option<i64> {
    let value = value.trim().parse::<i64>().ok()?;
    (value > -273_150).then_some(value / 1000)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_ioreg_temperature(info: &str) -> Option<i64> {
    info.lines().find_map(|line| {
        let value = line.split_once("temperature")?.1.split_once('=')?.1.trim();
        let value = value.split_whitespace().next()?.parse::<i64>().ok()?;
        Some(if value > 1_000 { value / 65_536 } else { value })
    })
}

fn gpu() -> FetchResult {
    #[cfg(target_os = "linux")]
    {
        return linux_gpu(Path::new("/sys/class/drm"));
    }
    #[cfg(target_os = "macos")]
    {
        return command("system_profiler SPDisplaysDataType -detailLevel mini").and_then(|info| {
            parse_system_profiler_gpu(&info).ok_or_else(|| "system_profiler reported no GPU".into())
        });
    }
    #[allow(unreachable_code)]
    Err("GPU identity is unsupported on this platform".into())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn linux_gpu(root: &Path) -> FetchResult {
    let entries = fs::read_dir(root).map_err(|error| format!("{}: {error}", root.display()))?;
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("card"))
        .find_map(|entry| fs::read_to_string(entry.path().join("device/uevent")).ok())
        .and_then(|info| parse_drm_uevent(&info))
        .ok_or_else(|| "no DRM GPU identity".into())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_drm_uevent(info: &str) -> Option<String> {
    let driver = info.lines().find_map(|line| line.strip_prefix("DRIVER="))?;
    let pci_id = info.lines().find_map(|line| line.strip_prefix("PCI_ID="));
    Some(match pci_id {
        Some(pci_id) => format!("{driver} ({pci_id})"),
        None => driver.to_owned(),
    })
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_system_profiler_gpu(info: &str) -> Option<String> {
    info.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Chipset Model:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
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
        return Ok(format_usage(
            total.saturating_sub(available) * 1024,
            total * 1024,
        ));
    }

    let native = (|| {
        let total = command("sysctl -n hw.memsize")?
            .parse::<u64>()
            .map_err(|error| format!("hw.memsize: {error}"))?;
        let used = command("vm_stat")
            .ok()
            .and_then(|info| parse_vm_stat(&info, total))
            .unwrap_or(total);
        Ok(format_usage(used, total))
    })();
    native.or_else(|error| sysinfo_memory().ok_or(error))
}

#[cfg(feature = "sysinfo")]
fn sysinfo_cpu() -> Option<String> {
    let system = sysinfo::System::new_all();
    let model = system.cpus().first()?.brand();
    let cores = sysinfo::System::physical_core_count().unwrap_or(system.cpus().len());
    (!model.is_empty() && cores > 0).then(|| format!("{model} ({cores} cores)"))
}

#[cfg(not(feature = "sysinfo"))]
fn sysinfo_cpu() -> Option<String> {
    None
}

#[cfg(feature = "sysinfo")]
fn sysinfo_memory() -> Option<String> {
    let system = sysinfo::System::new_all();
    let total = system.total_memory();
    (total > 0).then(|| format_usage(total.saturating_sub(system.available_memory()), total))
}

#[cfg(not(feature = "sysinfo"))]
fn sysinfo_memory() -> Option<String> {
    None
}

#[cfg(feature = "sysinfo")]
fn sysinfo_disk() -> Option<String> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .find(|disk| disk.mount_point() == Path::new("/"))
        .or_else(|| disks.list().first())?;
    let total = disk.total_space();
    (total > 0).then(|| format_usage(total.saturating_sub(disk.available_space()), total))
}

#[cfg(not(feature = "sysinfo"))]
fn sysinfo_disk() -> Option<String> {
    None
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
        ColorMode, IconMode, Theme, detect_theme, format_uptime, format_usage, label_text,
        linux_gpu, linux_temperature, load_config, load_logo, mem_kib, os_logo, parse_args,
        parse_boottime, parse_color, parse_colorfgbg, parse_config, parse_cpuinfo,
        parse_drm_uevent, parse_icons, parse_ioreg_temperature, parse_millidegrees, parse_rows,
        parse_system_profiler_gpu, parse_vm_stat, render, render_probe, render_with_color,
    };

    #[test]
    fn parses_meminfo_values() {
        assert_eq!(mem_kib("MemTotal: 1024 kB", "MemTotal"), Some(1024));
    }

    #[test]
    fn formats_compact_fastfetch_style_stats() {
        assert_eq!(format_uptime(3), "0m");
        assert_eq!(format_uptime(3_660), "1h 1m");
        assert_eq!(format_uptime(93_784), "1d 2h 3m");
        assert_eq!(
            parse_boottime("{ sec = 1787264167, usec = 728186 }"),
            Some(1_787_264_167)
        );
        assert_eq!(
            format_usage(8 * 1024_u64.pow(3), 16 * 1024_u64.pow(3)),
            "8.00 GiB / 16.00 GiB (50%)"
        );
        assert_eq!(format_usage(0, 0), "0.00 GiB / 0.00 GiB (0%)");
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
    fn parses_linux_temperature_fixture() {
        assert_eq!(
            parse_millidegrees(include_str!("../tests/fixtures/linux-temperature")),
            Some(62)
        );
    }

    #[test]
    fn parses_macos_temperature_fixture() {
        assert_eq!(
            parse_ioreg_temperature(include_str!("../tests/fixtures/macos-ioreg-temperature")),
            Some(54)
        );
    }

    #[test]
    fn parses_linux_gpu_fixture() {
        assert_eq!(
            parse_drm_uevent(include_str!("../tests/fixtures/linux-drm-uevent")).as_deref(),
            Some("amdgpu (1002:73bf)")
        );
    }

    #[test]
    fn parses_macos_gpu_fixture() {
        assert_eq!(
            parse_system_profiler_gpu(include_str!("../tests/fixtures/macos-system-profiler-gpu"))
                .as_deref(),
            Some("Apple M1 Pro")
        );
    }

    #[test]
    fn missing_hardware_paths_return_errors() {
        let root = std::env::temp_dir().join(format!("minfetch-hardware-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create empty fixture directory");
        assert!(linux_temperature(&root).is_err());
        assert!(linux_gpu(&root).is_err());
        std::fs::remove_dir(&root).expect("remove empty fixture directory");
    }

    #[test]
    fn parses_linux_hardware_directories() {
        let root =
            std::env::temp_dir().join(format!("minfetch-hardware-fixture-{}", std::process::id()));
        let thermal = root.join("thermal_zone0");
        let card = root.join("card0/device");
        std::fs::create_dir_all(&thermal).expect("create thermal fixture");
        std::fs::create_dir_all(&card).expect("create DRM fixture");
        std::fs::write(thermal.join("temp"), "62000\n").expect("write temperature");
        std::fs::write(thermal.join("type"), "x86_pkg_temp\n").expect("write sensor type");
        std::fs::write(card.join("uevent"), "DRIVER=amdgpu\nPCI_ID=1002:73bf\n")
            .expect("write DRM uevent");

        assert_eq!(linux_temperature(&root).as_deref(), Ok("62 °C"));
        assert_eq!(linux_gpu(&root).as_deref(), Ok("amdgpu (1002:73bf)"));
        std::fs::remove_dir_all(root).expect("remove fixture directory");
    }

    #[cfg(feature = "sysinfo")]
    #[test]
    fn sysinfo_fallbacks_report_host_values() {
        assert!(super::sysinfo_cpu().is_some());
        assert!(super::sysinfo_memory().is_some());
        assert!(super::sysinfo_disk().is_some());
    }

    #[test]
    fn missing_proc_fixtures_stay_unavailable() {
        assert!(parse_cpuinfo("").is_none());
        assert!(mem_kib("not proc data", "MemTotal").is_none());
        assert!(parse_millidegrees("-273151").is_none());
        assert!(parse_millidegrees("invalid").is_none());
        assert!(parse_ioreg_temperature("temperature = nope").is_none());
        assert!(parse_system_profiler_gpu("no chipset here").is_none());
        assert_eq!(
            parse_drm_uevent("DRIVER=amdgpu\n").as_deref(),
            Some("amdgpu")
        );
    }

    #[test]
    fn piped_output_never_loads_a_logo() {
        assert!(load_logo(Some("/definitely/not/a/logo"), false).is_none());
    }

    #[test]
    fn logo_modes_select_the_os_logo() {
        assert!(load_logo(None, true).is_none());
        assert!(load_logo(Some("none"), true).is_none());
        assert_eq!(load_logo(Some("auto"), true).as_deref(), Some(os_logo()));
    }

    #[test]
    fn custom_logo_files_and_invalid_config_paths_report_their_result() {
        let path = std::env::temp_dir().join(format!("minfetch-logo-{}", std::process::id()));
        std::fs::write(&path, "logo").expect("write logo");
        assert_eq!(load_logo(path.to_str(), true).as_deref(), Some("logo"));
        std::fs::remove_file(&path).expect("remove logo");

        let directory = std::env::temp_dir();
        assert!(load_config(Some(directory.to_str().expect("temp directory"))).is_err());
    }

    #[test]
    fn logo_flag_selects_auto_without_consuming_the_next_flag() {
        let (options, _, _) =
            parse_args(["--logo", "--icons", "off"].into_iter().map(str::to_owned)).unwrap();
        assert_eq!(options.logo.as_deref(), Some("auto"));
        assert_eq!(options.icons, IconMode::Off);
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
        assert_eq!(options.icons, IconMode::Off);
        assert!(options.no_terminator && options.no_term && options.verbose);
        assert!(!help && !version);
    }

    #[test]
    fn defaults_hide_icons_and_logo() {
        let (options, _, _) = parse_args(std::iter::empty()).expect("parse default options");
        assert_eq!(options.icons, IconMode::Off);
        assert!(options.logo.is_none());
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
    fn invalid_config_values_and_flags_are_rejected() {
        assert!(parse_color("bright", 2).is_err());
        assert!(parse_icons("emoji", 2).is_err());
        assert!(parse_rows("", 2).is_err());
        assert!(parse_rows("os, unknown", 2).is_err());
        for arguments in [
            vec!["--color", "bright"],
            vec!["--color"],
            vec!["--theme", "bright"],
            vec!["--theme"],
            vec!["--icons", "emoji"],
            vec!["--icons"],
            vec!["--config"],
            vec!["--unknown"],
            vec!["value"],
        ] {
            assert!(parse_args(arguments.into_iter().map(str::to_owned)).is_err());
        }
        for content in ["broken", "theme = bright", "unknown = value"] {
            assert!(parse_config(content).is_err());
        }
    }

    #[test]
    fn colorfgbg_detects_dark_and_light_backgrounds() {
        assert_eq!(parse_colorfgbg("15;0"), Some(Theme::Subtle));
        assert_eq!(parse_colorfgbg("0;7"), Some(Theme::Mono));
        assert_eq!(parse_colorfgbg("7"), Some(Theme::Mono));
    }

    #[test]
    fn malformed_colorfgbg_uses_the_fallback() {
        assert_eq!(parse_colorfgbg("not-a-palette"), None);
        assert_eq!(parse_colorfgbg("1;99"), None);
        assert_eq!(detect_theme(true, None), Theme::Subtle);
        assert_eq!(detect_theme(true, Some("not-a-palette")), Theme::Subtle);
        assert_eq!(detect_theme(false, Some("0;7")), Theme::Subtle);
    }

    #[test]
    fn parses_config_defaults() {
        let config = parse_config(
            "color = never\nicons = off\nlogo = none\nrows = os, user\ntheme = mono\n",
        )
        .unwrap();

        assert_eq!(config.color, Some(ColorMode::Never));
        assert_eq!(config.icons, Some(IconMode::Off));
        assert_eq!(config.logo.as_deref(), Some("none"));
        assert_eq!(
            config.rows.as_deref(),
            Some(["os".into(), "user".into()].as_slice())
        );
        assert_eq!(config.theme, Some(Theme::Mono));
    }

    #[test]
    fn explicit_theme_overrides_config_theme() {
        let path = std::env::temp_dir().join(format!("minfetch-theme-{}", std::process::id()));
        std::fs::write(&path, "theme = mono\n").expect("write config");

        let (configured, _, _) = parse_args(
            ["--config", path.to_str().expect("config path")]
                .into_iter()
                .map(str::to_owned),
        )
        .expect("parse configured args");
        let (explicit, _, _) = parse_args(
            [
                "--config",
                path.to_str().expect("config path"),
                "--theme",
                "subtle",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .expect("parse explicit args");
        std::fs::remove_file(path).expect("remove config");

        assert_eq!(configured.theme, Some(Theme::Mono));
        assert_eq!(explicit.theme, Some(Theme::Subtle));
    }

    #[test]
    fn probe_output_includes_environment_and_fetch_results() {
        let rows = vec![("os".into(), "test-os".into())];
        let output = render_probe(&rows, &["cpu: fixture failure".into()], 80, 24, false);

        assert!(output.contains("minfetch probe"));
        assert!(output.contains("platform:"));
        assert!(output.contains("architecture:"));
        assert!(output.contains("terminal_size: 80x24"));
        assert!(output.contains("os: test-os"));
        assert!(output.contains("  cpu: fixture failure"));
        assert!(!output.contains("\x1b["));
    }

    #[cfg(feature = "json")]
    #[test]
    fn json_output_escapes_row_values() {
        let rows = vec![("user\"name".into(), "line\nvalue".into())];
        assert_eq!(
            super::render_json(&rows),
            "{\"rows\":[{\"label\":\"user\\\"name\",\"value\":\"line\\nvalue\"}]}\n"
        );
    }

    #[test]
    fn color_wraps_labels_without_changing_layout_width() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(
            render_with_color(&rows, None, 40, 4, IconMode::Off, true),
            "\x1b[36mOS: \x1b[0m macos\n"
        );
    }

    #[test]
    fn version_contains_package_version() {
        assert!(super::version_string().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn layout_stacks_logo_in_a_narrow_pane() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(
            render(&rows, Some("/\\"), 8, 4, IconMode::Off),
            "/\\\nOS:\nma…\n"
        );
    }

    #[test]
    fn layout_uses_side_by_side_logo_when_wide() {
        let rows = vec![("os".into(), "macos".into())];
        assert_eq!(
            render(&rows, Some("/\\"), 20, 4, IconMode::Off),
            "/\\  OS:  macos\n"
        );
    }

    #[test]
    fn layout_uses_single_column_at_thirty() {
        let rows = vec![
            ("os".into(), "macos".into()),
            ("user".into(), "matheus".into()),
        ];
        assert_eq!(
            render(&rows, None, 30, 4, IconMode::Off),
            "OS:\nmacos\nUser:\nmatheus\n"
        );
    }

    #[test]
    fn layout_truncates_to_height() {
        let rows = vec![("one".into(), "1".into()), ("two".into(), "2".into())];
        assert_eq!(render(&rows, None, 80, 1, IconMode::Off), "One:  1\n");
    }

    #[test]
    fn truncation_respects_wide_glyphs() {
        assert_eq!(super::truncate("hello", 0), "");
        assert_eq!(super::truncate("hello", 1), "…");
        assert_eq!(super::truncate("猫猫", 3), "猫…");
    }

    #[test]
    fn icon_modes_keep_labels_readable() {
        assert_eq!(label_text("cpu", IconMode::Off), "CPU:");
        assert!(label_text("cpu", IconMode::Unicode).starts_with('◈'));
        assert!(label_text("cpu", IconMode::Nerd).contains("CPU:"));
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
        let output = super::render(&rows, None, 80, 20, IconMode::Off);
        assert_eq!(output.matches(super::MISSING).count(), rows.len());
    }
}
