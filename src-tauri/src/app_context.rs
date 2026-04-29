#[cfg(target_os = "macos")]
pub fn frontmost_app_name() -> Option<String> {
    let output = std::process::Command::new("/usr/bin/lsappinfo")
        .args(["info", "-only", "name", "front"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    parse_lsappinfo_name(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_app_name() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn parse_lsappinfo_name(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(eq_idx) = trimmed.find('=') {
        let value = trimmed[eq_idx + 1..]
            .trim()
            .trim_matches(';')
            .trim_matches(',')
            .trim_matches('"')
            .trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }

    let first_quote = trimmed.find('"')?;
    let rest = &trimmed[first_quote + 1..];
    let end_quote = rest.find('"')?;
    let value = rest[..end_quote].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::parse_lsappinfo_name;

    #[test]
    fn parses_simple_assignment() {
        assert_eq!(
            parse_lsappinfo_name("name=\"Cursor\""),
            Some("Cursor".to_string())
        );
    }

    #[test]
    fn parses_key_value_output() {
        assert_eq!(
            parse_lsappinfo_name("ASN:0x0-0x123:\n    name = \"Mail\""),
            Some("Mail".to_string())
        );
    }
}
