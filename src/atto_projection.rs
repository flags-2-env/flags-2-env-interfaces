#![forbid(unsafe_code)]

//! Strict host JSON bridge for downstream AttoLang FlagCatalog projection.
//! The authored TypeSpec and JSON Schema remain independent peer authorities.
use serde::Deserialize;
use serde_json::{Map, Value};
use crate::protocol::FlagCatalog;

pub const MAX_CATALOG_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    InvalidBudget,
    TooLarge,
    InvalidJsonOrShape,
    EmptyId,
    EmptyRevision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictFlagCatalog {
    id: String,
    revision: String,
    payload: Map<String, Value>,
}

/// Bound input before serde processing, reject duplicate/missing/unknown root
/// fields, preserve arbitrary object payloads, and use JSON Schema minLength:1.
/// This does not grant AttoLang runtime permissions or validate nested payloads.
pub fn decode_catalog_for_atto(
    bytes: &[u8],
    byte_budget: usize,
) -> Result<FlagCatalog, ProjectionError> {
    if byte_budget == 0 || byte_budget > MAX_CATALOG_BYTES {
        return Err(ProjectionError::InvalidBudget);
    }
    if bytes.len() > byte_budget {
        return Err(ProjectionError::TooLarge);
    }
    let decoded: StrictFlagCatalog =
        serde_json::from_slice(bytes).map_err(|_| ProjectionError::InvalidJsonOrShape)?;
    if decoded.id.is_empty() {
        return Err(ProjectionError::EmptyId);
    }
    if decoded.revision.is_empty() {
        return Err(ProjectionError::EmptyRevision);
    }
    Ok(FlagCatalog {
        id: decoded.id,
        revision: decoded.revision,
        payload: Value::Object(decoded.payload),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(source: &str) -> Result<FlagCatalog, ProjectionError> {
        decode_catalog_for_atto(source.as_bytes(), MAX_CATALOG_BYTES)
    }

    #[test]
    fn nested_payload_and_whitespace_nonempty_names_match_schema() {
        let catalog = decode(r#"{"id":" ","revision":" ","payload":{"values":[true,3,null]}}"#)
            .expect("authored schema permits whitespace-only nonempty identifiers");
        assert_eq!(catalog.id, " ");
        assert_eq!(catalog.revision, " ");
        assert_eq!(catalog.payload["values"][0], Value::Bool(true));
    }

    #[test]
    fn unknown_missing_and_duplicate_root_keys_fail_closed() {
        for invalid in [
            r#"{"id":"x","revision":"1","payload":{},"extra":true}"#,
            r#"{"id":"x","revision":"1"}"#,
            r#"{"id":"x","id":"y","revision":"1","payload":{}}"#,
            r#"{"id":"x","revision":"1","revision":"2","payload":{}}"#,
            r#"{"id":"x","revision":"1","payload":{},"payload":{}}"#,
        ] {
            assert_eq!(decode(invalid), Err(ProjectionError::InvalidJsonOrShape));
        }
    }

    #[test]
    fn non_object_payloads_bad_field_types_and_empty_values_fail_closed() {
        for invalid in [
            r#"{"id":"x","revision":"1","payload":null}"#,
            r#"{"id":"x","revision":"1","payload":[]}"#,
            r#"{"id":7,"revision":"1","payload":{}}"#,
            r#"{"id":"x","revision":1,"payload":{}}"#,
        ] {
            assert_eq!(decode(invalid), Err(ProjectionError::InvalidJsonOrShape));
        }
        assert_eq!(
            decode(r#"{"id":"","revision":"1","payload":{}}"#),
            Err(ProjectionError::EmptyId)
        );
        assert_eq!(
            decode(r#"{"id":"x","revision":"","payload":{}}"#),
            Err(ProjectionError::EmptyRevision)
        );
    }

    #[test]
    fn invalid_and_oversize_budgets_are_rejected_before_json_parsing() {
        let valid = br#"{"id":"x","revision":"1","payload":{}}"#;
        assert_eq!(
            decode_catalog_for_atto(valid, 0),
            Err(ProjectionError::InvalidBudget)
        );
        assert_eq!(
            decode_catalog_for_atto(valid, MAX_CATALOG_BYTES + 1),
            Err(ProjectionError::InvalidBudget)
        );
        assert_eq!(
            decode_catalog_for_atto(valid, valid.len() - 1),
            Err(ProjectionError::TooLarge)
        );
        assert!(decode_catalog_for_atto(valid, valid.len()).is_ok());
    }
}
