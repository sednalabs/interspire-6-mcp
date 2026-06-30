use crate::redact;
use serde::Serialize;

#[derive(Debug, Clone, serde::Deserialize, rmcp::schemars::JsonSchema)]
pub struct OciLedgerPreflightRequest {
    #[schemars(length(min = 1, max = 200))]
    pub campaign_id: String,
    #[schemars(length(min = 1, max = 200))]
    pub batch_id: String,
    #[schemars(range(min = 1))]
    pub expected_rows: u64,
    #[serde(default)]
    #[schemars(length(min = 1, max = 253))]
    pub sender_domain: Option<String>,
    #[serde(default)]
    #[schemars(length(min = 64, max = 64))]
    pub expected_manifest_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OciLedgerPreflightReport {
    pub required: bool,
    pub configured: bool,
    pub requested: bool,
    pub verified: bool,
    pub campaign_hash: Option<String>,
    pub batch_hash: Option<String>,
    pub sender_domain: Option<String>,
    pub expected_rows: Option<u64>,
    pub matched_rows: u64,
    pub rows_with_recipient_key: u64,
    pub rows_with_trace_key: u64,
    pub invalid_rows: u64,
    pub manifest_sha256: Option<String>,
    pub raw_payload_returned: bool,
    pub warnings: Vec<String>,
}

impl OciLedgerPreflightReport {
    pub fn skipped(required: bool, configured: bool, note: &str) -> Self {
        Self {
            required,
            configured,
            requested: false,
            verified: !required,
            campaign_hash: None,
            batch_hash: None,
            sender_domain: None,
            expected_rows: None,
            matched_rows: 0,
            rows_with_recipient_key: 0,
            rows_with_trace_key: 0,
            invalid_rows: 0,
            manifest_sha256: None,
            raw_payload_returned: false,
            warnings: vec![note.to_string()],
        }
    }

    pub fn blocked(
        required: bool,
        configured: bool,
        request: Option<&OciLedgerPreflightRequest>,
        warning: String,
    ) -> Self {
        Self {
            required,
            configured,
            requested: request.is_some(),
            verified: false,
            campaign_hash: request.map(|item| short_hash(&item.campaign_id)),
            batch_hash: request.map(|item| short_hash(&item.batch_id)),
            sender_domain: None,
            expected_rows: request.map(|item| item.expected_rows),
            matched_rows: 0,
            rows_with_recipient_key: 0,
            rows_with_trace_key: 0,
            invalid_rows: 0,
            manifest_sha256: None,
            raw_payload_returned: false,
            warnings: vec![redact::redact_sensitive_text(&warning)],
        }
    }

    pub fn fixture_verified() -> Self {
        Self {
            required: true,
            configured: true,
            requested: true,
            verified: true,
            campaign_hash: Some(short_hash("fixture-campaign")),
            batch_hash: Some(short_hash("fixture-batch")),
            sender_domain: Some("example.invalid".to_string()),
            expected_rows: Some(1),
            matched_rows: 1,
            rows_with_recipient_key: 1,
            rows_with_trace_key: 1,
            invalid_rows: 0,
            manifest_sha256: None,
            raw_payload_returned: false,
            warnings: Vec::new(),
        }
    }
}

pub(crate) fn short_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};

    let normalized = value.trim().to_ascii_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    hex::encode(&digest[..10])
}
