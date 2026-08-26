#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.

pub mod error;
pub mod protocol;
pub mod schema;

pub use error::InterfaceError;
pub use protocol::{Health, FlagCatalog, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};

