//! Built-in presets: one file per vendor in `presets/`, each an `hmac`
//! credential's settings without the secret. Choosing one fills the form; the
//! saved credential holds its own copy, so editing a preset here never changes
//! a check that is already live.

use serde_json::Value;

const FILES: &[&str] = &[
    include_str!("presets/github.json"),
    include_str!("presets/meta.json"),
    include_str!("presets/shopify.json"),
    include_str!("presets/slack.json"),
    include_str!("presets/zoom.json"),
    include_str!("presets/stripe.json"),
    include_str!("presets/paddle.json"),
    include_str!("presets/square.json"),
    include_str!("presets/standard-webhooks.json"),
    include_str!("presets/svix.json"),
];

/// Every preset as `{ id, title, note, settings }`, in the order above.
pub fn presets() -> Vec<Value> {
    FILES.iter().filter_map(|text| serde_json::from_str(text).ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_is_a_valid_check_once_a_secret_is_added() {
        assert_eq!(presets().len(), FILES.len(), "a preset file does not parse");
        for preset in presets() {
            let id = preset["id"].as_str().unwrap_or("?").to_string();
            assert!(preset["title"].as_str().is_some_and(|t| !t.is_empty()), "{id}: no title");
            let mut settings = preset["settings"].clone();
            let secret = if settings["secret_encoding"] == "base64" { "whsec_c2VjcmV0LWtleS1ieXRlcw==" } else { "secret" };
            settings["secrets"] = serde_json::json!({ "current": secret });
            settings["preset"] = Value::String(id.clone());
            super::super::Settings::from_value(&settings).unwrap_or_else(|err| panic!("{id}: {err}"));
        }
    }
}
