//! The credential tools of `zebflow.command.run` (`app-control.md` § Rules
//! 5 to 8): a door sees the credentials delegated to it and the ones it made,
//! creates only the kinds its fence names, and changes or deletes only its own.

use serde_json::{Map, Value, json};

use super::command::Fence;
use crate::platform::model::{PatchProjectCredentialRequest, UpsertProjectCredentialRequest};
use crate::platform::services::PlatformService;

/// Keys whose value is an address a credential connects to.
const ADDRESS_KEYS: &[&str] = &["host", "hostname", "server", "address", "base_url", "url", "endpoint", "uri", "dsn", "connection_string"];

fn text<'a>(arguments: &'a Value, key: &str) -> Option<&'a str> {
    arguments.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn required<'a>(arguments: &'a Value, key: &str) -> Result<&'a str, String> {
    text(arguments, key).ok_or_else(|| format!("`{key}` is required"))
}

/// The ids of the credentials whose door is this fence.
pub(super) fn managed_ids(platform: &PlatformService, owner: &str, project: &str, fence: &Fence) -> Result<Vec<String>, String> {
    Ok(platform
        .credentials
        .list_project_credentials(owner, project)
        .map_err(|err| err.message)?
        .into_iter()
        .filter(|item| item.managed_by == fence.id)
        .map(|item| item.credential_id)
        .collect())
}

/// Every address in a secret must resolve outside the machine and the
/// private networks: the rule `http.response.fetch` follows.
fn check_addresses(secret: &Value) -> Result<(), String> {
    let Value::Object(map) = secret else { return Ok(()) };
    let port = map.get("port").and_then(|port| port.as_str().map(str::to_string).or_else(|| port.as_u64().map(|n| n.to_string())));
    for (key, value) in map {
        let Some(raw) = value.as_str().map(str::trim).filter(|raw| !raw.is_empty()) else { continue };
        let url = if raw.contains("://") {
            raw.to_string()
        } else if ADDRESS_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
            match &port {
                Some(port) if !raw.contains(':') => format!("http://{raw}:{port}/"),
                _ => format!("http://{raw}/"),
            }
        } else {
            continue;
        };
        // A database URL names its host the same way an http one does; only
        // the host and port are checked.
        let checked = match reqwest::Url::parse(&url) {
            Ok(parsed) if !matches!(parsed.scheme(), "http" | "https") => {
                let host = parsed.host_str().unwrap_or_default();
                match parsed.port() {
                    Some(port) => format!("http://{host}:{port}/"),
                    None => format!("http://{host}/"),
                }
            }
            _ => url,
        };
        crate::pipeline::security::validate_outbound_http_url(&checked, super::command::NODE_KIND)
            .map_err(|err| format!("`{key}` is not an outside address: {}", err.message))?;
    }
    Ok(())
}

/// The fields of a credential that are not secret, by its kind's form.
fn public_fields(kind: &str, secret: &Value) -> Value {
    let types = crate::platform::services::credential::builtin_credential_types();
    let mut out = Map::new();
    if let Some(def) = types.iter().find(|def| def.kind == kind) {
        for field in def.fields.iter().filter(|field| field.field_type != "password") {
            if let Some(value) = secret.get(&field.key) {
                out.insert(field.key.clone(), value.clone());
            }
        }
    }
    Value::Object(out)
}

fn is_managed(platform: &PlatformService, owner: &str, project: &str, fence: &Fence, credential_id: &str) -> Result<crate::platform::model::ProjectCredential, String> {
    let credential = platform
        .credentials
        .get_project_credential(owner, project, credential_id)
        .map_err(|err| err.message)?
        .ok_or_else(|| format!("credential '{credential_id}' is not managed by this door"))?;
    if credential.managed_by != fence.id {
        let why = if fence.delegate.iter().any(|id| id == credential_id) { "delegated to this door, which may only use it" } else { "not managed by this door" };
        return Err(format!("credential '{credential_id}' is {why}"));
    }
    Ok(credential)
}

pub(super) fn list(platform: &PlatformService, owner: &str, project: &str, fence: &Fence) -> Result<Value, String> {
    let mut rows = Vec::new();
    for item in platform.credentials.list_project_credentials(owner, project).map_err(|err| err.message)? {
        let managed = item.managed_by == fence.id;
        if !managed && !fence.delegate.contains(&item.credential_id) {
            continue;
        }
        let secret = platform
            .credentials
            .get_project_credential(owner, project, &item.credential_id)
            .map_err(|err| err.message)?
            .map(|credential| credential.secret)
            .unwrap_or(Value::Null);
        rows.push(json!({
            "credential_id": item.credential_id,
            "kind": item.kind,
            "title": item.title,
            "managed": managed,
            "fields": public_fields(&item.kind, &secret),
        }));
    }
    Ok(json!({ "credentials": rows, "managed": rows.iter().filter(|row| row["managed"] == true).count(), "limit": fence.manage_limit }))
}

pub(super) fn create(platform: &PlatformService, owner: &str, project: &str, fence: &Fence, arguments: &Value) -> Result<Value, String> {
    let credential_id = required(arguments, "credential_id")?;
    if crate::platform::model::slug_segment(credential_id) != credential_id {
        return Err(format!("credential id '{credential_id}' must be lowercase letters, digits, '-' or '_'"));
    }
    let kind = required(arguments, "kind")?;
    if kind == super::command::CREDENTIAL_KIND {
        return Err(format!("a door may never create a {kind} credential"));
    }
    if !fence.manage_kinds.iter().any(|allowed| allowed == kind) {
        return Err(format!("kind '{kind}' is outside what this door manages ({})", fence.manage_kinds.join(", ")));
    }
    let secret = arguments.get("secret").cloned().unwrap_or(Value::Null);
    if !secret.is_object() {
        return Err("`secret` must be an object of the kind's fields".to_string());
    }
    if platform.credentials.get_project_credential(owner, project, credential_id).map_err(|err| err.message)?.is_some() {
        return Err(format!("credential '{credential_id}' already exists; a door creates new credentials only"));
    }
    if let Some(limit) = fence.manage_limit {
        if managed_ids(platform, owner, project, fence)?.len() >= limit as usize {
            return Err(format!("this door already manages {limit} credentials, its limit"));
        }
    }
    check_addresses(&secret)?;
    let saved = platform
        .credentials
        .upsert_managed_credential(
            owner,
            project,
            &fence.id,
            &UpsertProjectCredentialRequest {
                credential_id: credential_id.to_string(),
                title: text(arguments, "title").unwrap_or_default().to_string(),
                kind: kind.to_string(),
                secret,
                notes: text(arguments, "notes").unwrap_or_default().to_string(),
            },
        )
        .map_err(|err| err.message)?;
    Ok(json!({ "ok": true, "credential_id": saved.credential_id, "kind": saved.kind, "fields": public_fields(&saved.kind, &saved.secret) }))
}

pub(super) fn update(platform: &PlatformService, owner: &str, project: &str, fence: &Fence, arguments: &Value) -> Result<Value, String> {
    let credential_id = required(arguments, "credential_id")?;
    let existing = is_managed(platform, owner, project, fence, credential_id)?;
    let patch = arguments.get("secret").cloned();
    if let Some(patch) = &patch {
        if !patch.is_object() {
            return Err("`secret` must be an object: the fields to change".to_string());
        }
        let mut merged = existing.secret.clone();
        crate::platform::services::credential::merge_patch(&mut merged, patch);
        check_addresses(&merged)?;
    }
    let saved = platform
        .credentials
        .patch_project_credential(
            owner,
            project,
            credential_id,
            &PatchProjectCredentialRequest {
                title: text(arguments, "title").map(str::to_string),
                notes: text(arguments, "notes").map(str::to_string),
                secret: patch,
            },
        )
        .map_err(|err| err.message)?;
    Ok(json!({ "ok": true, "credential_id": saved.credential_id, "fields": public_fields(&saved.kind, &saved.secret) }))
}

pub(super) fn delete(platform: &PlatformService, owner: &str, project: &str, fence: &Fence, arguments: &Value) -> Result<Value, String> {
    let credential_id = required(arguments, "credential_id")?;
    is_managed(platform, owner, project, fence, credential_id)?;
    platform.credentials.delete_project_credential(owner, project, credential_id).map_err(|err| err.message)?;
    Ok(json!({ "ok": true, "credential_id": credential_id }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_inside_the_machine_or_a_private_network_is_refused() {
        for secret in [
            json!({ "host": "localhost", "port": "5432" }),
            json!({ "host": "127.0.0.1" }),
            json!({ "host": "10.0.0.5", "port": 5432 }),
            json!({ "base_url": "http://169.254.169.254/latest" }),
            json!({ "dsn": "postgres://127.0.0.1:5432/db" }),
            json!({ "anything": "mysql://10.1.2.3/db" }),
        ] {
            assert!(check_addresses(&secret).is_err(), "{secret}");
        }
        // Only address fields are addresses: a user named like a host is not.
        check_addresses(&json!({ "host": "203.0.113.10", "port": "5432", "user": "localhost" })).expect("an outside address");
    }

    #[test]
    fn only_fields_that_are_not_passwords_are_shown() {
        let hidden = uuid::Uuid::new_v4().to_string();
        let shown = public_fields("postgres", &json!({ "host": "db.example.com", "user": "reader", "password": hidden }));
        assert_eq!(shown, json!({ "host": "db.example.com", "user": "reader" }));
        assert_eq!(public_fields("no-such-kind", &json!({ "password": hidden })), json!({}));
    }
}
