use std::io;
use std::path::Path;

use saphyr_parser::{Event as YamlEvent, Parser as YamlParser};
use serde_json::{Map, Value};

use crate::infra::io::durable::atomic_write;

use super::{
    CONTRACT_API_VERSION, ContractDocument, ContractError, ContractKind, ContractMetadata,
    PlatformContract, contract_descriptor,
};

const ROOT_FIELDS: &[&str] = &["apiVersion", "kind", "metadata", "spec"];
const MAX_YAML_CONTRACT_BYTES: usize = 4 * 1024 * 1024;
const MAX_YAML_EVENTS: usize = 100_000;
const MAX_YAML_DEPTH: usize = 128;

/// Parses and validates one canonical contract document.
pub fn decode_contract<C: PlatformContract>(
    bytes: &[u8],
) -> Result<ContractDocument<C::Spec>, ContractError> {
    let value: Value = serde_json::from_slice(bytes).map_err(ContractError::Json)?;
    decode_contract_value::<C>(value)
}

/// Parses one strict YAML 1.2 contract document.
///
/// Zebflow accepts only the JSON-compatible YAML subset. Anchors, aliases,
/// explicit tags, multiple documents, non-string mapping keys, and unbounded
/// structures are rejected before typed deserialization.
pub fn decode_contract_yaml<C: PlatformContract>(
    bytes: &[u8],
) -> Result<ContractDocument<C::Spec>, ContractError> {
    if bytes.len() > MAX_YAML_CONTRACT_BYTES {
        return Err(ContractError::invalid(format!(
            "YAML contract exceeds the {} byte limit",
            MAX_YAML_CONTRACT_BYTES
        )));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|_| ContractError::invalid("YAML contract must be valid UTF-8"))?;
    validate_yaml_events(source)?;
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(bytes).map_err(ContractError::Yaml)?;
    let json = yaml_to_json(yaml, 0, &mut 0)?;
    decode_contract_value::<C>(json)
}

/// Parses a contract from an existing JSON value.
pub fn decode_contract_value<C: PlatformContract>(
    value: Value,
) -> Result<ContractDocument<C::Spec>, ContractError> {
    let object = value
        .as_object()
        .ok_or_else(|| ContractError::invalid("root must be a JSON object"))?;
    reject_unknown_root_fields(object)?;

    let api_version = required_string(object, "apiVersion")?;
    if api_version != CONTRACT_API_VERSION {
        return Err(ContractError::UnsupportedApiVersion(
            api_version.to_string(),
        ));
    }

    let kind_name = required_string(object, "kind")?;
    let actual_kind = ContractKind::parse(kind_name)
        .ok_or_else(|| ContractError::UnknownKind(kind_name.to_string()))?;
    if actual_kind != C::KIND {
        return Err(ContractError::UnexpectedKind {
            expected: C::KIND.to_string(),
            actual: actual_kind.to_string(),
        });
    }
    let _descriptor = contract_descriptor(actual_kind);

    let metadata_value = object
        .get("metadata")
        .cloned()
        .ok_or_else(|| ContractError::invalid("missing required root field 'metadata'"))?;
    let metadata: ContractMetadata =
        serde_json::from_value(metadata_value).map_err(ContractError::Json)?;
    metadata.validate()?;

    let spec_value = object
        .get("spec")
        .cloned()
        .ok_or_else(|| ContractError::invalid("missing required root field 'spec'"))?;
    let spec: C::Spec = serde_json::from_value(spec_value).map_err(ContractError::Json)?;
    C::validate(&metadata, &spec)?;

    Ok(ContractDocument {
        api_version: CONTRACT_API_VERSION,
        kind: C::KIND.as_str(),
        metadata,
        spec,
    })
}

/// Validates and serializes one canonical contract document.
pub fn encode_contract<C: PlatformContract>(
    metadata: ContractMetadata,
    spec: C::Spec,
) -> Result<Vec<u8>, ContractError> {
    metadata.validate()?;
    C::validate(&metadata, &spec)?;
    let document = ContractDocument {
        api_version: CONTRACT_API_VERSION,
        kind: C::KIND.as_str(),
        metadata,
        spec,
    };
    let mut bytes = serde_json::to_vec_pretty(&document).map_err(ContractError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Validates and serializes one canonical YAML contract document.
pub fn encode_contract_yaml<C: PlatformContract>(
    metadata: ContractMetadata,
    spec: C::Spec,
) -> Result<Vec<u8>, ContractError> {
    metadata.validate()?;
    C::validate(&metadata, &spec)?;
    let document = ContractDocument {
        api_version: CONTRACT_API_VERSION,
        kind: C::KIND.as_str(),
        metadata,
        spec,
    };
    let text = serde_yaml_ng::to_string(&document).map_err(ContractError::Yaml)?;
    if text.len() > MAX_YAML_CONTRACT_BYTES {
        return Err(ContractError::invalid(format!(
            "YAML contract exceeds the {} byte limit",
            MAX_YAML_CONTRACT_BYTES
        )));
    }
    Ok(text.into_bytes())
}

/// Reads an optional contract. Only `NotFound` becomes `None`.
pub fn read_optional_contract<C: PlatformContract>(
    path: &Path,
) -> Result<Option<ContractDocument<C::Spec>>, ContractError> {
    match std::fs::read(path) {
        Ok(bytes) => decode_contract::<C>(&bytes).map(Some),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ContractError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Reads an optional strict YAML contract. Only `NotFound` becomes `None`.
pub fn read_optional_contract_yaml<C: PlatformContract>(
    path: &Path,
) -> Result<Option<ContractDocument<C::Spec>>, ContractError> {
    match std::fs::read(path) {
        Ok(bytes) => decode_contract_yaml::<C>(&bytes).map(Some),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ContractError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Durably writes a canonical contract document.
pub fn write_contract<C: PlatformContract>(
    path: &Path,
    metadata: ContractMetadata,
    spec: C::Spec,
) -> Result<(), ContractError> {
    let bytes = encode_contract::<C>(metadata, spec)?;
    atomic_write(path, &bytes).map_err(|source| ContractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Durably writes a canonical YAML contract document.
pub fn write_contract_yaml<C: PlatformContract>(
    path: &Path,
    metadata: ContractMetadata,
    spec: C::Spec,
) -> Result<(), ContractError> {
    let bytes = encode_contract_yaml::<C>(metadata, spec)?;
    atomic_write(path, &bytes).map_err(|source| ContractError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_yaml_events(source: &str) -> Result<(), ContractError> {
    let mut documents = 0usize;
    let mut depth = 0usize;
    let mut events = 0usize;
    for event in YamlParser::new_from_str(source) {
        let (event, _) = event.map_err(|err| ContractError::invalid(err.to_string()))?;
        events += 1;
        if events > MAX_YAML_EVENTS {
            return Err(ContractError::invalid(format!(
                "YAML contract exceeds the {} event limit",
                MAX_YAML_EVENTS
            )));
        }
        match event {
            YamlEvent::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    return Err(ContractError::invalid(
                        "YAML contract must contain exactly one document",
                    ));
                }
            }
            YamlEvent::Alias(_) => {
                return Err(ContractError::invalid(
                    "YAML aliases and anchors are not allowed",
                ));
            }
            YamlEvent::Scalar(_, _, anchor, tag) => {
                reject_yaml_properties(anchor, tag.is_some())?;
            }
            YamlEvent::SequenceStart(anchor, tag) | YamlEvent::MappingStart(anchor, tag) => {
                reject_yaml_properties(anchor, tag.is_some())?;
                depth += 1;
                if depth > MAX_YAML_DEPTH {
                    return Err(ContractError::invalid(format!(
                        "YAML contract exceeds the {} level depth limit",
                        MAX_YAML_DEPTH
                    )));
                }
            }
            YamlEvent::SequenceEnd | YamlEvent::MappingEnd => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    if documents != 1 {
        return Err(ContractError::invalid(
            "YAML contract must contain exactly one document",
        ));
    }
    Ok(())
}

fn reject_yaml_properties(anchor: usize, tagged: bool) -> Result<(), ContractError> {
    if anchor != 0 {
        return Err(ContractError::invalid(
            "YAML aliases and anchors are not allowed",
        ));
    }
    if tagged {
        return Err(ContractError::invalid("explicit YAML tags are not allowed"));
    }
    Ok(())
}

fn yaml_to_json(
    value: serde_yaml_ng::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<Value, ContractError> {
    *nodes += 1;
    if *nodes > MAX_YAML_EVENTS || depth > MAX_YAML_DEPTH {
        return Err(ContractError::invalid("YAML contract is too complex"));
    }
    match value {
        serde_yaml_ng::Value::Null => Ok(Value::Null),
        serde_yaml_ng::Value::Bool(value) => Ok(Value::Bool(value)),
        serde_yaml_ng::Value::Number(value) => {
            serde_json::to_value(value).map_err(ContractError::Json)
        }
        serde_yaml_ng::Value::String(value) => Ok(Value::String(value)),
        serde_yaml_ng::Value::Sequence(values) => values
            .into_iter()
            .map(|value| yaml_to_json(value, depth + 1, nodes))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        serde_yaml_ng::Value::Mapping(values) => {
            let mut object = Map::new();
            for (key, value) in values {
                let serde_yaml_ng::Value::String(key) = key else {
                    return Err(ContractError::invalid(
                        "YAML contract mapping keys must be strings",
                    ));
                };
                object.insert(key, yaml_to_json(value, depth + 1, nodes)?);
            }
            Ok(Value::Object(object))
        }
        serde_yaml_ng::Value::Tagged(_) => {
            Err(ContractError::invalid("explicit YAML tags are not allowed"))
        }
    }
}

fn reject_unknown_root_fields(object: &Map<String, Value>) -> Result<(), ContractError> {
    for field in object.keys() {
        if !ROOT_FIELDS.contains(&field.as_str()) {
            return Err(ContractError::invalid(format!(
                "unknown root field '{field}'"
            )));
        }
    }
    for field in ROOT_FIELDS {
        if !object.contains_key(*field) {
            return Err(ContractError::invalid(format!(
                "missing required root field '{field}'"
            )));
        }
    }
    Ok(())
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, ContractError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ContractError::invalid(format!("'{field}' must be a non-empty string")))
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields)]
    struct TestSpec {
        value: String,
    }

    struct TestContract;

    impl PlatformContract for TestContract {
        type Spec = TestSpec;
        const KIND: ContractKind = ContractKind::ProjectManifest;
    }

    fn valid_value() -> Value {
        serde_json::json!({
            "apiVersion": CONTRACT_API_VERSION,
            "kind": "ProjectManifest",
            "metadata": {"name": "test"},
            "spec": {"value": "ok"}
        })
    }

    #[test]
    fn canonical_contract_roundtrips() {
        let encoded = encode_contract::<TestContract>(
            ContractMetadata::named("test"),
            TestSpec {
                value: "ok".to_string(),
            },
        )
        .unwrap();
        let decoded = decode_contract::<TestContract>(&encoded).unwrap();
        assert_eq!(decoded.metadata.name, "test");
        assert_eq!(decoded.spec.value, "ok");
    }

    #[test]
    fn canonical_yaml_contract_is_deterministic_and_roundtrips() {
        let encode = || {
            encode_contract_yaml::<TestContract>(
                ContractMetadata::named("test"),
                TestSpec {
                    value: "ok".to_string(),
                },
            )
            .unwrap()
        };
        let first = encode();
        assert_eq!(first, encode());
        let decoded = decode_contract_yaml::<TestContract>(&first).unwrap();
        assert_eq!(decoded.metadata.name, "test");
        assert_eq!(decoded.spec.value, "ok");
    }

    #[test]
    fn strict_yaml_rejects_unsafe_or_ambiguous_features() {
        let anchor = b"apiVersion: zebflow.com/v1\nkind: ProjectManifest\nmetadata:\n  name: test\nspec:\n  value: &shared ok\n";
        assert!(decode_contract_yaml::<TestContract>(anchor).is_err());

        let tag = b"apiVersion: zebflow.com/v1\nkind: ProjectManifest\nmetadata:\n  name: test\nspec:\n  value: !secret ok\n";
        assert!(decode_contract_yaml::<TestContract>(tag).is_err());

        let multiple = b"apiVersion: zebflow.com/v1\nkind: ProjectManifest\nmetadata: {name: test}\nspec: {value: ok}\n---\n{}\n";
        assert!(decode_contract_yaml::<TestContract>(multiple).is_err());

        let non_string_key = b"apiVersion: zebflow.com/v1\nkind: ProjectManifest\nmetadata: {name: test}\nspec:\n  1: ok\n";
        assert!(decode_contract_yaml::<TestContract>(non_string_key).is_err());

        let duplicate = b"apiVersion: zebflow.com/v1\nkind: ProjectManifest\nmetadata: {name: test}\nspec:\n  value: first\n  value: second\n";
        assert!(decode_contract_yaml::<TestContract>(duplicate).is_err());
    }

    #[test]
    fn rejects_missing_future_unknown_and_wrong_kind_contracts() {
        let mut missing = valid_value();
        missing.as_object_mut().unwrap().remove("apiVersion");
        assert_eq!(
            decode_contract_value::<TestContract>(missing)
                .unwrap_err()
                .category(),
            "invalid"
        );

        let mut future = valid_value();
        future["apiVersion"] = Value::String("zebflow.com/v2".to_string());
        assert_eq!(
            decode_contract_value::<TestContract>(future)
                .unwrap_err()
                .category(),
            "unsupported_api_version"
        );

        let mut unknown = valid_value();
        unknown["kind"] = Value::String("MadeUp".to_string());
        assert_eq!(
            decode_contract_value::<TestContract>(unknown)
                .unwrap_err()
                .category(),
            "unknown_kind"
        );

        let mut wrong = valid_value();
        wrong["kind"] = Value::String("Pipeline".to_string());
        assert_eq!(
            decode_contract_value::<TestContract>(wrong)
                .unwrap_err()
                .category(),
            "unexpected_kind"
        );

        let mut extra = valid_value();
        extra["other"] = Value::Bool(true);
        assert_eq!(
            decode_contract_value::<TestContract>(extra)
                .unwrap_err()
                .category(),
            "invalid"
        );
    }
}
