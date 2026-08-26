#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::InterfaceError;

pub const PROTOCOL_VERSION: &str = "1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    pub ok: bool,
    pub service: String,
    pub protocol: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlagCatalog {
    pub id: String,
    pub revision: String,
    #[serde(default)]
    pub payload: Value,
}

impl FlagCatalog {
    pub fn parse(id: String, revision: String, payload: Value) -> Result<Self, InterfaceError> {
        if id.trim().is_empty() {
            return Err(InterfaceError::EmptyId);
        }
        if revision.trim().is_empty() {
            return Err(InterfaceError::EmptyRevision);
        }
        Ok(Self { id, revision, payload })
    }
}

