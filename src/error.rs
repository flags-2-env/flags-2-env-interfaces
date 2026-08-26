#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceError {
    EmptyId,
    EmptyRevision,
    SchemaMismatch,
}

impl fmt::Display for InterfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "id must be non-empty"),
            Self::EmptyRevision => write!(f, "revision must be non-empty"),
            Self::SchemaMismatch => write!(f, "payload does not match the published schema"),
        }
    }
}

impl std::error::Error for InterfaceError {}

