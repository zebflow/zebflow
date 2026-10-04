//! One node's config against its definition, as a save sees it.
//!
//! Only what the definition declares is judged, and only a value already
//! written down: a value still holding `{{ }}` is checked when it resolves,
//! at run (`node-conventions.md` §3), except a literal-only flag, which
//! refuses `{{ }}` outright.

use serde_json::Value;

use super::Problem;
use crate::pipeline::NodeDefinition;
use crate::pipeline::model::{DslFlag, DslFlagKind, NODE_TIMEOUT_KEY, engine_common_dsl_flags, node_timeout};
use crate::pipeline::nodes::shared::{profile, units};

/// Flags that must stay literal (§3): `{{ }}` is refused at save.
pub const LITERAL_FLAGS: &[&str] = &["--provider", "--return", "--write"];

/// Config keys every kind carries and no definition declares: the canvas's
/// own (`ui`), and the nested object the preview flags write.
const PRESENTATION_KEYS: &[&str] = &["ui", "preview"];

/// The code the shared unit parser is called with here; only its message
/// is ever shown.
const UNITS_CODE: &str = "FW_PIPELINE_CHECK";

fn is_expression(value: &Value) -> bool {
    match value {
        Value::String(s) => s.contains("{{"),
        Value::Array(items) => items.iter().any(is_expression),
        Value::Object(map) => map.values().any(is_expression),
        _ => false,
    }
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

fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_string(),
        other => other.to_string(),
    }
}

/// The values a flag holds: each item of a repeated one, else the one.
fn items(flag: &DslFlag, value: &Value) -> Vec<Value> {
    match (&flag.kind, value) {
        (DslFlagKind::RepeatedList, Value::Array(list)) => list.clone(),
        (_, other) => vec![other.clone()],
    }
}

/// Every flag the kind takes: its own and the engine's, its own first.
fn flags(def: &NodeDefinition) -> Vec<DslFlag> {
    let mut all = def.dsl_flags.clone();
    for common in engine_common_dsl_flags() {
        if !all.iter().any(|f| f.flag == common.flag) {
            all.push(common);
        }
    }
    all
}

/// What is wrong with `config` for `def`, every problem at once. The
/// problems carry no node id; [`super::check_graph_nodes`] adds it.
pub fn check_node_config(def: &NodeDefinition, config: &Value) -> Vec<Problem> {
    let kind = def.kind.as_str();
    let mut problems = Vec::new();
    let mut push = |flag: Option<&str>, message: String| {
        problems.push(Problem::node("", kind, flag, message));
    };
    let flags = flags(def);
    let empty = serde_json::Map::new();
    let map = match config {
        Value::Object(map) => map,
        Value::Null => &empty,
        other => {
            push(None, format!("{kind} config must be an object, not {other}"));
            return problems;
        }
    };

    // A key the kind does not declare: a typo, a retired flag, a flag of
    // another kind. The UI form's own field names are the kind's too.
    for key in map.keys() {
        let declared = flags.iter().any(|f| &f.config_key == key || f.config_key.split('.').next() == Some(key.as_str()))
            || PRESENTATION_KEYS.contains(&key.as_str())
            || def.fields.iter().any(|f| &f.name == key);
        if !declared {
            let names: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
            let wanted = format!("--{}", key.replace('_', "-"));
            if let Some(flag) = def.dsl_flags.iter().find(|f| f.flag == wanted) {
                push(Some(&wanted), format!("{kind} keeps {wanted} under the config key `{}`, not `{key}`", flag.config_key));
                continue;
            }
            let hint = super::suggest::did_you_mean(&wanted, &names).map(|n| format!("; did you mean {n}?")).unwrap_or_default();
            let takes = if names.is_empty() { "no flags".to_string() } else { names.join(" ") };
            push(Some(&wanted), format!("{kind} takes no {wanted} (config key `{key}`){hint} — it takes {takes}"));
        }
    }

    for flag in &flags {
        let name = flag.flag.as_str();
        let value = map.get(&flag.config_key).unwrap_or(&Value::Null);
        if flag.required && is_empty(value) {
            push(Some(name), format!("{kind} needs {name}"));
            continue;
        }
        if is_empty(value) {
            continue;
        }
        if LITERAL_FLAGS.contains(&name) && is_expression(value) {
            let words = if flag.choices.is_empty() { String::new() } else { format!("; write one of {}", flag.choices.join(", ")) };
            push(Some(name), format!("{kind} {name} is literal, never {{{{ }}}}{words}"));
            continue;
        }
        if flag.config_key == NODE_TIMEOUT_KEY {
            if !is_expression(value)
                && let Err(err) = node_timeout(config)
            {
                push(Some(name), format!("{kind} {}", err.message));
            }
            continue;
        }
        if !matches!(flag.kind, DslFlagKind::Scalar | DslFlagKind::RepeatedList) {
            continue;
        }
        let values = items(flag, value);
        if let Some(max) = flag.max_repeat
            && !values.iter().any(is_expression)
            && values.len() > max as usize
        {
            push(Some(name), format!("{kind} {name} is given {} times; it takes at most {max}", values.len()));
        }
        for item in values.iter().filter(|v| !is_empty(v) && !is_expression(v)) {
            let given = text(item);
            if !flag.choices.is_empty() {
                if !flag.choices.iter().any(|c| c.eq_ignore_ascii_case(&given)) {
                    push(Some(name), format!("{kind} {name} '{given}' must be one of {}", flag.choices.join(", ")));
                }
                continue;
            }
            let wrong = match flag.value.as_str() {
                "number" => (!item.is_number() && !given.parse::<f64>().is_ok_and(f64::is_finite))
                    .then(|| format!("{kind} {name} '{given}' is not a number")),
                "duration" => units::duration(&given, name, UNITS_CODE).err().map(|e| format!("{kind} {}", e.message)),
                "size" => units::size(&given, name, UNITS_CODE).err().map(|e| format!("{kind} {}", e.message)),
                _ => None,
            };
            if let Some(message) = wrong {
                push(Some(name), message);
            }
        }
    }

    // A provider's own rules, once the shared ones hold for `--provider`.
    let provider_wrong = problems.iter().any(|p| p.flag.as_deref() == Some(profile::PROVIDER_FLAG));
    if !def.profiles.is_empty() && !provider_wrong {
        if let Err(message) = profile::check_profile(def, config) {
            problems.push(Problem::node("", kind, None, format!("{kind} {message}")));
        }
    }
    problems
}

/// The credential a provider kind's config names, with the provider it is
/// for: `(provider, credential id)`, when both are literal.
pub fn provider_credential(def: &NodeDefinition, config: &Value) -> Option<(String, String)> {
    if def.profiles.is_empty() {
        return None;
    }
    let provider = profile::chosen_profile(def, config).ok()?.provider.clone();
    let key = def.dsl_flags.iter().find(|f| f.flag == profile::CREDENTIAL_FLAG)?.config_key.as_str();
    let id = config.get(key)?.as_str()?.trim();
    (!id.is_empty() && !id.contains("{{")).then(|| (provider, id.to_string()))
}
