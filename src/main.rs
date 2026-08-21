pub fn version_string() -> String {
    format!("minfetch {}", env!("CARGO_PKG_VERSION"))
}

fn main() {
    println!("{}", version_string());
}

#[cfg(test)]
mod tests {
    use super::version_string;

    #[test]
    fn test_version_string_contains_version() {
        let out = version_string();
        assert!(out.starts_with("minfetch "));
        assert!(out.contains(env!("CARGO_PKG_VERSION")));
    }
}
