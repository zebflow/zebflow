//! Provider profiles (`node-conventions.md` §11).
//!
//! A swappable task is one kind with `--provider`. Its definition holds the
//! shared roles and, per provider, a [`ProviderProfile`]: the
//! provider-specific roles it takes, narrower choice lists, the `--model` ids
//! it accepts, the credential kinds that hold its key, and its own settings
//! as closed, typed `--option key=value` keys.
//!
//! [`check_profile`] holds a node's config to its provider's profile. It is
//! pure, so the node build, the run and (later) the save all call the same
//! rule. A value still holding a `{{ }}` is checked once it resolves, except
//! `--provider`, which is literal.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeDefinition, ProfileOption, ProviderProfile};

pub const PROVIDER_FLAG: &str = "--provider";
pub const OPTION_FLAG: &str = "--option";
pub const MODEL_FLAG: &str = "--model";
pub const CREDENTIAL_FLAG: &str = "--credential";

/// `--provider`, closed to `providers` and literal (§3).
pub fn provider_flag(providers: &[&str], description: &str) -> DslFlag {
    DslFlag {
        flag: PROVIDER_FLAG.to_string(),
        config_key: "provider".to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: true,
        value: "text".to_string(),
        choices: providers.iter().map(|p| p.to_string()).collect(),
        ..Default::default()
    }
}

/// `--option key=value`: a provider's own settings, closed and typed by its
/// profile.
pub fn option_flag() -> DslFlag {
    DslFlag {
        flag: OPTION_FLAG.to_string(),
        config_key: "option".to_string(),
        description: "A setting of the chosen provider, key=value, repeated. The keys and their types are the provider's profile; an unknown key is refused.".to_string(),
        kind: DslFlagKind::KeyValuePairs,
        value: "text".to_string(),
        ..Default::default()
    }
}

fn flag<'a>(def: &'a NodeDefinition, name: &str) -> Option<&'a DslFlag> {
    def.dsl_flags.iter().find(|f| f.flag == name)
}

fn is_expression(value: &Value) -> bool {
    value.as_str().is_some_and(|s| s.contains("{{"))
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(items) => items.iter().all(is_empty),
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

/// A list flag's values, a list inside the list flattened, empties dropped.
fn listed(value: &Value) -> Vec<&Value> {
    match value {
        Value::Array(items) => items.iter().flat_map(listed).collect(),
        other if is_empty(other) => Vec::new(),
        other => vec![other],
    }
}

fn word(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_string(),
        other => other.to_string(),
    }
}

/// The flags some profile names in its roles: the provider-specific ones.
/// Every other flag of the kind is shared by every provider.
pub fn gated_roles(def: &NodeDefinition) -> BTreeSet<&str> {
    def.profiles.iter().flat_map(|p| p.roles.keys().map(String::as_str)).collect()
}

fn providers(def: &NodeDefinition) -> String {
    def.profiles.iter().map(|p| p.provider.as_str()).collect::<Vec<_>>().join(", ")
}

/// The profile a config's `--provider` names. `--provider` is literal, and
/// a word without a profile is refused naming the ones that have one.
pub fn chosen_profile<'a>(def: &'a NodeDefinition, config: &Value) -> Result<&'a ProviderProfile, String> {
    let key = flag(def, PROVIDER_FLAG).map(|f| f.config_key.as_str()).unwrap_or("provider");
    let value = config.get(key).unwrap_or(&Value::Null);
    if is_expression(value) {
        return Err(format!("--provider is literal, never {{{{ }}}}; write one of {}", providers(def)));
    }
    let name = word(value);
    if is_empty(value) {
        return Err(format!("{} needs --provider: one of {}", def.kind, providers(def)));
    }
    def.profiles
        .iter()
        .find(|p| p.provider == name)
        .ok_or_else(|| format!("--provider '{name}' is not one of {}", providers(def)))
}

/// What a provider takes beyond the shared roles, for a refusal to name.
fn takes(profile: &ProviderProfile) -> String {
    if profile.roles.is_empty() {
        "none of the provider-specific roles".to_string()
    } else {
        profile.roles.keys().cloned().collect::<Vec<_>>().join(", ")
    }
}

fn option_keys(profile: &ProviderProfile) -> String {
    if profile.options.is_empty() {
        "it takes no --option".to_string()
    } else {
        format!("it takes {}", profile.options.iter().map(|o| o.key.as_str()).collect::<Vec<_>>().join(", "))
    }
}

/// A node's config against its provider's profile (§11): the provider is a
/// literal with a profile; a provider-specific role the provider does not
/// take is refused, one it needs is required and its repeats capped; a choice
/// takes only the provider's words; `--model` only its models; `--option`
/// only its keys, each value of the key's type. A kind with no profiles
/// passes.
pub fn check_profile(def: &NodeDefinition, config: &Value) -> Result<(), String> {
    if def.profiles.is_empty() {
        return Ok(());
    }
    let profile = chosen_profile(def, config)?;
    let provider = profile.provider.as_str();
    let value_of = |name: &str| flag(def, name).and_then(|f| config.get(&f.config_key)).unwrap_or(&Value::Null);

    for name in gated_roles(def) {
        let value = value_of(name);
        match profile.roles.get(name) {
            None if !is_empty(value) => {
                return Err(format!("--provider {provider} does not take {name}; it takes {}", takes(profile)));
            }
            None => {}
            Some(role) => {
                if role.required && is_empty(value) {
                    return Err(format!("--provider {provider} needs {name}"));
                }
                let values = listed(value);
                if let Some(max) = role.max_repeat
                    && !values.iter().any(|v| is_expression(v))
                    && values.len() > max as usize
                {
                    return Err(format!("--provider {provider} takes at most {max} {name}; {} given", values.len()));
                }
            }
        }
    }

    for (name, words) in &profile.choices {
        for given in listed(value_of(name)).into_iter().filter(|v| !is_expression(v)) {
            let given = word(given);
            if !words.contains(&given) {
                return Err(format!("--provider {provider} takes {name} {}; not '{given}'", words.join("|")));
            }
        }
    }

    if !profile.models.is_empty() {
        let model = value_of(MODEL_FLAG);
        if !is_empty(model) && !is_expression(model) && !profile.models.contains(&word(model)) {
            return Err(format!(
                "--provider {provider} takes --model {}; not '{}'",
                profile.models.join("|"),
                word(model)
            ));
        }
    }

    match value_of(OPTION_FLAG) {
        Value::Object(map) => {
            for (key, value) in map {
                let option = profile
                    .options
                    .iter()
                    .find(|o| &o.key == key)
                    .ok_or_else(|| format!("--option '{key}' is not a setting of --provider {provider}; {}", option_keys(profile)))?;
                check_option_value(provider, option, value)?;
            }
        }
        value if is_empty(value) || is_expression(value) => {}
        other => return Err(format!("--option takes key=value pairs, not {other}")),
    }
    Ok(())
}

/// One `--option` value against its declared type and words.
fn check_option_value(provider: &str, option: &ProfileOption, value: &Value) -> Result<(), String> {
    if is_expression(value) {
        return Ok(());
    }
    let key = option.key.as_str();
    let text = word(value);
    if text.is_empty() {
        return Err(format!("--option {key} is empty; it needs a value"));
    }
    if !option.choices.is_empty() && !option.choices.contains(&text) {
        return Err(format!("--provider {provider} takes --option {key}={}; not '{text}'", option.choices.join("|")));
    }
    // The shared unit parser reads durations and sizes; its own code is
    // never raised from here, only its message.
    const CODE: &str = "FW_NODE_PACKAGE_CONFIG";
    let flag = format!("--option {key}");
    match option.value.as_str() {
        "number" if !text.parse::<f64>().is_ok_and(f64::is_finite) => {
            Err(format!("--option {key}='{text}' is not a number"))
        }
        "duration" => crate::pipeline::nodes::shared::units::duration(&text, &flag, CODE).map(|_| ()).map_err(|e| e.message),
        "size" => crate::pipeline::nodes::shared::units::size(&text, &flag, CODE).map(|_| ()).map_err(|e| e.message),
        _ => Ok(()),
    }
}

/// A credential of `kind` against the provider's profile: its kind must be
/// one the provider's key lives in.
pub fn check_credential(def: &NodeDefinition, provider: &str, credential_id: &str, kind: &str) -> Result<(), String> {
    let Some(profile) = def.profiles.iter().find(|p| p.provider == provider) else {
        return Err(format!("--provider '{provider}' is not one of {}", providers(def)));
    };
    if profile.credential_kinds.is_empty() || profile.credential_kinds.iter().any(|k| k == kind) {
        return Ok(());
    }
    Err(format!(
        "credential '{credential_id}' is kind '{kind}'; --provider {provider} takes a credential of kind {}",
        profile.credential_kinds.join(" or ")
    ))
}

/// One `--option` value of the config, by key, as text.
pub fn option_text(config_option: &Value, key: &str) -> Option<String> {
    config_option.get(key).map(word).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::pipeline::model::ProfileRole;

    /// A video-shaped kind: two providers, one taking `--image` (at most 2)
    /// and a narrower `--format`, the other a typed option.
    fn def() -> NodeDefinition {
        let words = |w: &[&str]| w.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        NodeDefinition {
            kind: "ai.video.generate".to_string(),
            dsl_flags: vec![
                provider_flag(&["alpha", "beta"], "Who generates it."),
                DslFlag { flag: "--credential".into(), config_key: "credential_id".into(), value: "text".into(), required: true, ..Default::default() },
                DslFlag { flag: "--prompt".into(), config_key: "prompt".into(), value: "text".into(), required: true, ..Default::default() },
                DslFlag { flag: "--model".into(), config_key: "model".into(), value: "text".into(), ..Default::default() },
                DslFlag { flag: "--image".into(), config_key: "image".into(), kind: DslFlagKind::RepeatedList, value: "file:image".into(), max_repeat: Some(4), ..Default::default() },
                DslFlag { flag: "--format".into(), config_key: "format".into(), choices: words(&["mp4", "webm"]), ..Default::default() },
                option_flag(),
            ],
            profiles: vec![
                ProviderProfile {
                    provider: "alpha".into(),
                    credential_kinds: words(&["alpha"]),
                    models: words(&["a-1", "a-2"]),
                    roles: BTreeMap::from([("--image".to_string(), ProfileRole { required: false, max_repeat: Some(2) })]),
                    choices: BTreeMap::from([("--format".to_string(), words(&["mp4"]))]),
                    ..Default::default()
                },
                ProviderProfile {
                    provider: "beta".into(),
                    credential_kinds: words(&["beta", "beta_team"]),
                    options: vec![
                        ProfileOption { key: "seed".into(), value: "number".into(), description: "Seed.".into(), ..Default::default() },
                        ProfileOption { key: "motion".into(), value: "text".into(), choices: words(&["low", "high"]), description: "Motion.".into() },
                    ],
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    fn refused(config: Value) -> String {
        check_profile(&def(), &config).expect_err("refused")
    }

    #[test]
    fn the_provider_is_a_literal_with_a_profile() {
        assert!(check_profile(&def(), &json!({ "provider": "alpha" })).is_ok());
        assert!(refused(json!({})).contains("needs --provider: one of alpha, beta"));
        assert!(refused(json!({ "provider": "gamma" })).contains("'gamma' is not one of alpha, beta"));
        assert!(refused(json!({ "provider": "{{ input.who }}" })).contains("literal"));
    }

    #[test]
    fn a_role_the_provider_does_not_take_is_refused_naming_what_it_takes() {
        let message = refused(json!({ "provider": "beta", "image": ["a.png"] }));
        assert!(message.contains("beta does not take --image"), "{message}");
        assert!(message.contains("none of the provider-specific roles"), "{message}");
        assert!(check_profile(&def(), &json!({ "provider": "alpha", "image": ["a.png", "b.png"] })).is_ok());
        let message = refused(json!({ "provider": "alpha", "image": ["a", "b", "c"] }));
        assert!(message.contains("at most 2 --image"), "{message}");
        // A list still to resolve is counted once it has resolved.
        assert!(check_profile(&def(), &json!({ "provider": "alpha", "image": ["{{ input.images }}"] })).is_ok());
    }

    #[test]
    fn a_choice_and_a_model_take_only_the_providers_words() {
        assert!(refused(json!({ "provider": "alpha", "format": "webm" })).contains("takes --format mp4; not 'webm'"));
        assert!(check_profile(&def(), &json!({ "provider": "beta", "format": "webm" })).is_ok());
        assert!(refused(json!({ "provider": "alpha", "model": "b-9" })).contains("--model a-1|a-2"));
        assert!(check_profile(&def(), &json!({ "provider": "beta", "model": "anything" })).is_ok());
    }

    #[test]
    fn an_option_is_closed_and_typed() {
        assert!(check_profile(&def(), &json!({ "provider": "beta", "option": { "seed": "42", "motion": "low" } })).is_ok());
        let message = refused(json!({ "provider": "beta", "option": { "speed": "2" } }));
        assert!(message.contains("'speed' is not a setting of --provider beta; it takes seed, motion"), "{message}");
        assert!(refused(json!({ "provider": "beta", "option": { "seed": "many" } })).contains("not a number"));
        assert!(refused(json!({ "provider": "beta", "option": { "motion": "wild" } })).contains("motion=low|high"));
        assert!(refused(json!({ "provider": "alpha", "option": { "seed": "1" } })).contains("it takes no --option"));
        // A resolved expression is checked by type, a pending one waits.
        assert!(check_profile(&def(), &json!({ "provider": "beta", "option": { "seed": 7 } })).is_ok());
        assert!(check_profile(&def(), &json!({ "provider": "beta", "option": "{{ input.options }}" })).is_ok());
    }

    #[test]
    fn a_credential_of_another_providers_kind_is_refused() {
        assert!(check_credential(&def(), "beta", "team", "beta_team").is_ok());
        let message = check_credential(&def(), "alpha", "main", "beta").expect_err("refused");
        assert!(message.contains("credential 'main' is kind 'beta'; --provider alpha takes a credential of kind alpha"), "{message}");
    }
}
