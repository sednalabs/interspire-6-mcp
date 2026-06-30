use crate::{
    config::OciSendLedgerConfig,
    redact,
    response::{short_hash as ledger_hash, OciLedgerPreflightReport, OciLedgerPreflightRequest},
};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub fn verify_preflight(
    config: &OciSendLedgerConfig,
    request: Option<&OciLedgerPreflightRequest>,
    expected_recipient_count: u64,
    interspire_campaign_id: Option<u64>,
) -> OciLedgerPreflightReport {
    let configured = config
        .path
        .as_deref()
        .is_some_and(|path| !path.trim().is_empty());
    let required = config.required_for_sends;
    let Some(request) = request else {
        return OciLedgerPreflightReport::skipped(
            required,
            configured,
            "OCI send ledger preflight was not requested.",
        );
    };

    if request.expected_rows != expected_recipient_count {
        return OciLedgerPreflightReport::blocked(
            required,
            configured,
            Some(request),
            "OCI send ledger expected row count does not match the Interspire send recipient count"
                .to_string(),
        );
    }
    if !valid_identifier(&request.campaign_id) || !valid_identifier(&request.batch_id) {
        return OciLedgerPreflightReport::blocked(
            required,
            configured,
            Some(request),
            "OCI send ledger campaign_id and batch_id must be non-empty printable identifiers"
                .to_string(),
        );
    }
    if let Some(expected_campaign_id) = interspire_campaign_id {
        if request.campaign_id.trim() != expected_campaign_id.to_string() {
            return OciLedgerPreflightReport::blocked(
                required,
                configured,
                Some(request),
                "OCI send ledger campaign_id must match the Interspire campaign being sent"
                    .to_string(),
            );
        }
    }
    if let Some(domain) = request.sender_domain.as_deref() {
        if !valid_domain(domain) {
            return OciLedgerPreflightReport::blocked(
                required,
                configured,
                Some(request),
                "OCI send ledger sender_domain must be a valid domain token".to_string(),
            );
        }
    }
    if let Some(manifest) = request.expected_manifest_sha256.as_deref() {
        if !valid_sha256(manifest) {
            return OciLedgerPreflightReport::blocked(
                required,
                configured,
                Some(request),
                "OCI send ledger expected_manifest_sha256 must be a 64-character hex SHA-256"
                    .to_string(),
            );
        }
    }

    let Some(path) = config
        .path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    else {
        return OciLedgerPreflightReport::blocked(
            required,
            false,
            Some(request),
            "INTERSPIRE_OCI_SEND_LEDGER_PATH is not configured; guarded send refused before the Interspire final send boundary"
                .to_string(),
        );
    };
    let Ok(file) = File::open(path) else {
        return OciLedgerPreflightReport::blocked(
            required,
            true,
            Some(request),
            "configured OCI send ledger could not be opened; guarded send refused before the Interspire final send boundary"
                .to_string(),
        );
    };

    let mut matched_rows = 0u64;
    let mut rows_with_recipient_key = 0u64;
    let mut rows_with_trace_key = 0u64;
    let mut invalid_rows = 0u64;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            invalid_rows += 1;
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            invalid_rows += 1;
            continue;
        };
        if !value.is_object() {
            invalid_rows += 1;
            continue;
        }
        if !row_matches(request, &value) {
            continue;
        }
        matched_rows += 1;
        if has_any_string(
            &value,
            &[
                "recipient_id",
                "recipientId",
                "recipient_id_hash",
                "recipientIdHash",
                "recipient",
                "recipient_email",
                "recipientEmail",
                "recipient_address_hash",
                "recipientAddressHash",
                "recipient_hash",
                "recipientHash",
            ],
        ) {
            rows_with_recipient_key += 1;
        }
        if has_any_string(
            &value,
            &[
                "message_id",
                "messageId",
                "provider_message_id",
                "providerMessageId",
                "message_id_hash",
                "messageIdHash",
                "correlation_id",
                "correlationId",
                "correlation_id_hash",
                "correlationIdHash",
                "header_value_hash",
                "headerValueHash",
            ],
        ) {
            rows_with_trace_key += 1;
        }
    }

    let mut warnings = Vec::new();
    if matched_rows != request.expected_rows {
        warnings.push(format!(
            "OCI send ledger matched {matched_rows} rows but expected {} rows",
            request.expected_rows
        ));
    }
    if rows_with_recipient_key != matched_rows {
        warnings.push(
            "one or more matched OCI send ledger rows lack a recipient key or recipient hash"
                .to_string(),
        );
    }
    if rows_with_trace_key != matched_rows {
        warnings.push(
            "one or more matched OCI send ledger rows lack a message or correlation key"
                .to_string(),
        );
    }
    if invalid_rows > 0 {
        warnings.push("one or more OCI send ledger rows were invalid JSON objects".to_string());
    }

    OciLedgerPreflightReport {
        required,
        configured: true,
        requested: true,
        verified: warnings.is_empty(),
        campaign_hash: Some(ledger_hash(&request.campaign_id)),
        batch_hash: Some(ledger_hash(&request.batch_id)),
        sender_domain: request
            .sender_domain
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase()),
        expected_rows: Some(request.expected_rows),
        matched_rows,
        rows_with_recipient_key,
        rows_with_trace_key,
        invalid_rows,
        manifest_sha256: request
            .expected_manifest_sha256
            .as_ref()
            .map(|value| value.trim().to_ascii_lowercase()),
        raw_payload_returned: false,
        warnings: warnings
            .into_iter()
            .map(|warning| redact::redact_sensitive_text(&warning))
            .collect(),
    }
}

fn row_matches(request: &OciLedgerPreflightRequest, value: &Value) -> bool {
    if !matches_identifier(
        &request.campaign_id,
        string_any(value, &["campaign_id", "campaignId"]),
        string_any(
            value,
            &[
                "campaign_hash",
                "campaignHash",
                "campaign_id_hash",
                "campaignIdHash",
            ],
        ),
    ) {
        return false;
    }
    if !matches_identifier(
        &request.batch_id,
        string_any(value, &["batch_id", "batchId"]),
        string_any(
            value,
            &["batch_hash", "batchHash", "batch_id_hash", "batchIdHash"],
        ),
    ) {
        return false;
    }
    if let Some(sender_domain) = request.sender_domain.as_deref() {
        let expected = sender_domain.trim().to_ascii_lowercase();
        let actual = string_any(value, &["sender_domain", "senderDomain"])
            .and_then(domain_from_address_or_domain)
            .or_else(|| {
                string_any(value, &["sender", "approved_sender", "approvedSender"])
                    .and_then(email_domain)
            });
        if actual.as_deref() != Some(expected.as_str()) {
            return false;
        }
    }
    if let Some(expected_manifest) = request.expected_manifest_sha256.as_deref() {
        if !matches_sha256(
            expected_manifest,
            string_any(value, &["manifest_sha256", "manifestSha256"]),
        ) {
            return false;
        }
    }
    true
}

fn matches_identifier(filter: &str, raw_value: Option<&str>, hash_value: Option<&str>) -> bool {
    let normalized_filter = filter.trim();
    raw_value.is_some_and(|value| value.trim() == normalized_filter)
        || hash_value
            .map(normalized_hash)
            .is_some_and(|value| value == ledger_hash(normalized_filter))
}

fn matches_sha256(filter: &str, raw_value: Option<&str>) -> bool {
    let normalized_filter = filter.trim();
    raw_value.is_some_and(|value| value.trim().eq_ignore_ascii_case(normalized_filter))
}

fn normalized_hash(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() == 20 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        trimmed.to_ascii_lowercase()
    } else {
        ledger_hash(trimmed)
    }
}

fn string_any<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
}

fn has_any_string(value: &Value, keys: &[&str]) -> bool {
    string_any(value, keys).is_some_and(|item| !item.trim().is_empty())
}

fn domain_from_address_or_domain(value: &str) -> Option<String> {
    if value.contains('@') {
        return email_domain(value);
    }
    valid_domain(value).then(|| value.trim().to_ascii_lowercase())
}

fn email_domain(value: &str) -> Option<String> {
    let (_local, domain) = value.trim().split_once('@')?;
    valid_domain(domain).then(|| domain.trim().to_ascii_lowercase())
}

fn valid_identifier(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 200
        && !trimmed.chars().any(char::is_control)
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
}

fn valid_domain(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 253
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        && trimmed.contains('.')
        && !trimmed.starts_with('.')
        && !trimmed.ends_with('.')
}

fn valid_sha256(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.len() == 64 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn preflight_verifies_matching_private_ledger_rows_without_raw_output() {
        let path = fixture_path("valid");
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(
            &path,
            format!(
                "{{\"campaign_hash\":\"{}\",\"batch_hash\":\"{}\",\"sender\":\"news@example.invalid\",\"recipient_hash\":\"{}\",\"message_id_hash\":\"{}\"}}\n\
                 {{\"campaign_id\":\"other\",\"batch_id\":\"batch-private\",\"sender\":\"news@example.invalid\",\"recipient\":\"person@example.invalid\",\"message_id\":\"msg-2\"}}\n",
                ledger_hash("campaign-private"),
                ledger_hash("batch-private"),
                ledger_hash("person@example.invalid"),
                ledger_hash("msg-1")
            ),
        )
        .expect("write fixture");
        let report = verify_preflight(
            &OciSendLedgerConfig {
                path: Some(path.to_string_lossy().to_string()),
                required_for_sends: true,
            },
            Some(&OciLedgerPreflightRequest {
                campaign_id: "campaign-private".to_string(),
                batch_id: "batch-private".to_string(),
                expected_rows: 1,
                sender_domain: Some("example.invalid".to_string()),
                expected_manifest_sha256: None,
            }),
            1,
            None,
        );
        let body = serde_json::to_string(&report).expect("serialize report");

        assert!(report.verified);
        assert_eq!(report.matched_rows, 1);
        assert_eq!(report.rows_with_recipient_key, 1);
        assert_eq!(report.rows_with_trace_key, 1);
        assert!(!report.raw_payload_returned);
        assert!(!body.contains("person@example.invalid"));
        assert!(!body.contains("campaign-private"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn preflight_blocks_missing_rows_when_required() {
        let path = fixture_path("missing");
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(&path, "").expect("write fixture");
        let report = verify_preflight(
            &OciSendLedgerConfig {
                path: Some(path.to_string_lossy().to_string()),
                required_for_sends: true,
            },
            Some(&OciLedgerPreflightRequest {
                campaign_id: "campaign-private".to_string(),
                batch_id: "batch-private".to_string(),
                expected_rows: 1,
                sender_domain: Some("example.invalid".to_string()),
                expected_manifest_sha256: None,
            }),
            1,
            None,
        );

        assert!(!report.verified);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("expected 1 rows")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn preflight_blocks_invalid_manifest_without_echoing_raw_input() {
        let report = verify_preflight(
            &OciSendLedgerConfig {
                path: Some("/unused/private-ledger.jsonl".to_string()),
                required_for_sends: true,
            },
            Some(&OciLedgerPreflightRequest {
                campaign_id: "7".to_string(),
                batch_id: "batch-private".to_string(),
                expected_rows: 1,
                sender_domain: Some("example.invalid".to_string()),
                expected_manifest_sha256: Some("private-manifest-token".to_string()),
            }),
            1,
            Some(7),
        );
        let body = serde_json::to_string(&report).expect("serialize report");

        assert!(!report.verified);
        assert!(report.sender_domain.is_none());
        assert!(report.manifest_sha256.is_none());
        assert!(!body.contains("private-manifest-token"));
    }

    fn fixture_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        PathBuf::from(format!(
            "target/interspire-oci-ledger-tests/{label}-{unique}.jsonl"
        ))
    }
}
