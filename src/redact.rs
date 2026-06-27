use sha2::{Digest, Sha256};

pub fn redact_email(value: &str) -> String {
    let trimmed = value.trim();
    let Some((local, domain)) = trimmed.split_once('@') else {
        return "[redacted]".to_string();
    };
    if local.is_empty() || domain.contains('@') || !is_host_token(domain) {
        return "[redacted-email]".to_string();
    }
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{}", domain.to_ascii_lowercase())
}

pub fn email_hash(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    hex::encode(&digest[..12])
}

pub fn redact_sensitive_text(value: &str) -> String {
    let mut output = value.to_string();
    for marker in [
        "password",
        "passwd",
        "usertoken",
        "token",
        "cookie",
        "license",
        "api_key",
        "smtp_password",
        "bounce_password",
    ] {
        output = redact_marker(&output, marker);
    }
    output = redact_urls(&output);
    output = redact_email_addresses(&output);
    output = redact_host_tokens(&output);
    output
}

fn redact_marker(input: &str, marker: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if !lower.contains(marker) {
        return input.to_string();
    }

    let mut redact_next = false;
    let mut output = Vec::new();
    for part in input.split_whitespace() {
        if redact_next {
            if is_secret_separator(part) {
                output.push(part.to_string());
                continue;
            }
            output.push(redact_secret_value_token(part));
            redact_next = false;
            continue;
        }

        let part_lower = part.to_ascii_lowercase();
        if part_lower.contains(marker) {
            output.push("[redacted]".to_string());
            redact_next = marker_needs_following_secret_redaction(part, marker);
        } else {
            output.push(part.to_string());
        }
    }
    output.join(" ")
}

fn marker_needs_following_secret_redaction(part: &str, marker: &str) -> bool {
    let lower = part.to_ascii_lowercase();
    let Some(marker_start) = lower.find(marker) else {
        return false;
    };
    let after_marker = &part[marker_start + marker.len()..];
    if after_marker
        .chars()
        .any(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '@' | '/' | '.'))
    {
        return false;
    }

    true
}

fn is_secret_separator(part: &str) -> bool {
    part.chars()
        .all(|ch| matches!(ch, ':' | '=' | '-' | '>' | '"' | '\''))
}

fn redact_secret_value_token(part: &str) -> String {
    let leading_len = part
        .char_indices()
        .find_map(|(idx, ch)| is_token_char(ch).then_some(idx))
        .unwrap_or(part.len());
    let trailing_start = part
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| is_token_char(ch).then_some(idx + ch.len_utf8()))
        .unwrap_or(leading_len);
    let leading = &part[..leading_len];
    let trailing = &part[trailing_start..];
    format!("{leading}[redacted]{trailing}")
}

fn redact_urls(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        let rest = &input[cursor..];
        let Some(offset) = find_url_start(rest) else {
            output.push_str(rest);
            break;
        };

        output.push_str(&rest[..offset]);
        let start = cursor + offset;
        let mut end = start;
        for (relative, ch) in input[start..].char_indices() {
            if ch.is_whitespace() || matches!(ch, '"' | '\'' | '<' | '>') {
                break;
            }
            end = start + relative + ch.len_utf8();
        }
        let (url, trailing) = trim_trailing_url_punctuation(&input[start..end]);
        output.push_str("[redacted-url]");
        output.push_str(trailing);
        cursor = start + url.len() + trailing.len();
    }
    output
}

fn find_url_start(input: &str) -> Option<usize> {
    match (input.find("http://"), input.find("https://")) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }
}

fn trim_trailing_url_punctuation(value: &str) -> (&str, &str) {
    let mut end = value.len();
    while end > 0 {
        let ch = value[..end].chars().last().unwrap_or_default();
        if matches!(ch, ',' | '.' | ';' | ':' | ')' | ']' | '}') {
            end -= ch.len_utf8();
            continue;
        }
        break;
    }
    (&value[..end], &value[end..])
}

fn redact_email_addresses(input: &str) -> String {
    redact_tokens(input, is_email_token, "[redacted-email]")
}

fn redact_host_tokens(input: &str) -> String {
    redact_tokens(input, is_host_token, "[redacted-host]")
}

fn redact_tokens(input: &str, predicate: fn(&str) -> bool, replacement: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut token = String::new();
    for ch in input.chars() {
        if is_token_char(ch) {
            token.push(ch);
            continue;
        }

        flush_redacted_token(&mut output, &mut token, predicate, replacement);
        output.push(ch);
    }
    flush_redacted_token(&mut output, &mut token, predicate, replacement);
    output
}

fn flush_redacted_token(
    output: &mut String,
    token: &mut String,
    predicate: fn(&str) -> bool,
    replacement: &str,
) {
    if token.is_empty() {
        return;
    }

    if predicate(token) {
        output.push_str(replacement);
    } else {
        output.push_str(token);
    }
    token.clear();
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '@' | '.' | '_' | '-' | '+' | ':' | '/')
}

fn is_email_token(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && is_host_token(domain)
}

fn is_host_token(value: &str) -> bool {
    let without_port = value
        .rsplit_once(':')
        .and_then(|(host, port)| port.chars().all(|ch| ch.is_ascii_digit()).then_some(host))
        .unwrap_or(value);

    if without_port.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }

    let candidate = without_port
        .strip_prefix("www.")
        .unwrap_or(without_port)
        .trim_end_matches('.');
    let labels = candidate.split('.').collect::<Vec<_>>();
    if labels.len() < 2 {
        return false;
    }
    let Some(tld) = labels.last() else {
        return false;
    };
    if tld.len() < 2 || !tld.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }

    labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_email_without_hiding_domain_context() {
        assert_eq!(redact_email("Reporter@Example.COM"), "R***@example.com");
    }

    #[test]
    fn redacts_email_lists_without_leaking_second_address() {
        let output = redact_email("alice@example.com,bob@example.net");

        assert!(!output.contains("alice"));
        assert!(!output.contains("bob"));
        assert!(!output.contains("example.com"));
        assert!(!output.contains("example.net"));
        assert_eq!(output, "[redacted-email]");
    }

    #[test]
    fn redacts_plus_addressed_emails_without_leaking_localpart() {
        let output = redact_sensitive_text("recipient alice+news@example.com queued");

        assert!(!output.contains("alice"));
        assert!(!output.contains("+news"));
        assert_eq!(output, "recipient [redacted-email] queued");
    }

    #[test]
    fn redacts_known_email_fields_with_invalid_internal_addresses() {
        assert_eq!(redact_email("staff@internal"), "[redacted-email]");
        assert_eq!(redact_email("staff@localhost"), "[redacted-email]");
        assert_eq!(redact_email("staff@@example.com"), "[redacted-email]");
    }

    #[test]
    fn hashes_email_stably_without_returning_email() {
        let hash = email_hash("person@example.com");
        assert_eq!(hash, email_hash("PERSON@example.com"));
        assert!(!hash.contains("person"));
        assert_eq!(hash.len(), 24);
    }

    #[test]
    fn redacts_sensitive_markers() {
        let output = redact_sensitive_text("smtp_password=secret cookie=session usertoken=abc");
        assert!(!output.contains("secret"));
        assert!(!output.contains("session"));
        assert!(!output.contains("abc"));
    }

    #[test]
    fn redacts_separated_sensitive_marker_values() {
        let output = redact_sensitive_text(
            r#"password: hunter2 token abc123 cookie = session-value api_key = key-secret "api_token": "quoted-secret""#,
        );

        assert!(!output.contains("hunter2"));
        assert!(!output.contains("abc123"));
        assert!(!output.contains("session-value"));
        assert!(!output.contains("key-secret"));
        assert!(!output.contains("quoted-secret"));
        assert!(output.matches("[redacted]").count() >= 8);
    }

    #[test]
    fn redacts_free_text_email_urls_and_hosts() {
        let output = redact_sensitive_text(
            "request failed for alice@example.com at https://iem.example.net/admin/index.php?Page=Lists; dns iem.example.net:443",
        );

        assert!(!output.contains("alice"));
        assert!(!output.contains("example.com"));
        assert!(!output.contains("https://"));
        assert!(!output.contains("iem.example.net"));
        assert!(!output.contains(":443"));
        assert!(output.contains("[redacted-email]"));
        assert!(output.contains("[redacted-url]"));
        assert!(output.contains("[redacted-host]"));
    }

    #[test]
    fn redacts_punctuation_separated_email_tokens() {
        let output =
            redact_sensitive_text("to alice@example.com;bob@example.net, cc c@example.org");

        assert!(!output.contains("alice"));
        assert!(!output.contains("bob"));
        assert!(!output.contains("example.net"));
        assert_eq!(output.matches("[redacted-email]").count(), 3);
    }
}
