use std::{
    env, fs,
    io::{self, IsTerminal},
    process::Command,
};

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
    if options.logo.is_some() && io::stdout().is_terminal() {
        if let Ok(logo) = fs::read_to_string(options.logo.as_deref().unwrap()) {
            print!("{logo}");
        }
    }
    for (label, value) in rows() {
        let label = if options.icons { icon(&label) } else { label };
        println!("{label:<16} {value}");
    }
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
    if let Some(seconds) = fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()
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
    if let Ok(info) = fs::read_to_string("/proc/cpuinfo") {
        let model = info
            .lines()
            .find_map(|line| line.strip_prefix("model name\t: "))?;
        let cores = info
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        return Some(format!("{model} ({cores} cores)"));
    }
    let model = command("sysctl -n machdep.cpu.brand_string")?;
    Some(format!("{model} ({} cores)", command("sysctl -n hw.ncpu")?))
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
    use super::{mem_kib, parse_args};

    #[test]
    fn parses_meminfo_values() {
        assert_eq!(mem_kib("MemTotal: 1024 kB", "MemTotal"), Some(1024));
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
}
