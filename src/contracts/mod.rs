//! Unified contracts for durable and transferred Zebflow objects.
//!
//! Every authoritative Zebflow document uses the same envelope:
//! `apiVersion`, `kind`, `metadata`, and `spec`. Domain modules continue to own
//! their runtime models and semantic rules; this module owns identity, version
//! dispatch, canonical serialization, and durable I/O.

mod envelope;
mod error;
mod io;
pub mod kinds;
mod metadata;
mod registry;

pub use envelope::{ContractDocument, PlatformContract};
pub use error::ContractError;
pub use io::{
    decode_contract, decode_contract_value, decode_contract_yaml, encode_contract,
    encode_contract_yaml, read_optional_contract, read_optional_contract_yaml, write_contract,
    write_contract_yaml,
};
pub use metadata::ContractMetadata;
pub use registry::{
    CONTRACT_API_VERSION, ContractDescriptor, ContractKind, ContractRepresentation,
    contract_descriptor,
};
