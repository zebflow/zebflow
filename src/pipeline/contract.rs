//! Canonical persisted pipeline-source contract.
//!
//! Every pipeline reader must use this parser before applying structural or
//! node-specific validation. This keeps draft files, active snapshots,
//! composite functions, and runtime synchronization on the same envelope.

use crate::infra::io::durable::{
    DurableJsonError, JsonContract, JsonContractField, JsonContractValue, parse_versioned_json,
};

use super::PipelineGraph;

/// Current persisted pipeline kind.
pub const PIPELINE_SOURCE_KIND: &str = "zebflow.pipeline";
/// Current persisted pipeline schema version.
pub const PIPELINE_SOURCE_VERSION: &str = "0.1";

const PIPELINE_SOURCE_CONTRACT: JsonContract = JsonContract {
    name: "pipeline source",
    fields: &[
        JsonContractField {
            name: "kind",
            expected: JsonContractValue::String(PIPELINE_SOURCE_KIND),
        },
        JsonContractField {
            name: "version",
            expected: JsonContractValue::String(PIPELINE_SOURCE_VERSION),
        },
    ],
};

/// Parses one pipeline graph after enforcing its persisted identity and version.
pub fn parse_pipeline_graph(bytes: &[u8]) -> Result<PipelineGraph, DurableJsonError> {
    parse_versioned_json(bytes, PIPELINE_SOURCE_CONTRACT)
}
