use super::*;
use base64::Engine as _;
use serde_json::json;

const NOW: i64 = 1_790_000_000;

/// A key made for this run, never a literal in the source.
fn made_key(tag: &str) -> String {
    format!("{tag}-{}", uuid::Uuid::new_v4().simple())
}

fn sign(secret: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = <Hmac<sha2::Sha256> as hmac::digest::KeyInit>::new_from_slice(secret).unwrap();
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn check(settings: Value, headers: &[(&str, &str)], body: &[u8]) -> Result<(), Refusal> {
    check_at(settings, headers, body, "https://hooks.example/in?x=1")
}

fn check_at(settings: Value, headers: &[(&str, &str)], body: &[u8], url: &str) -> Result<(), Refusal> {
    let settings = Settings::from_value(&settings).expect("valid settings");
    let lookup = |name: &str| {
        headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.to_string())
    };
    settings.verify(&Request { header: &lookup, body, method: "POST", path: "/in", url, now: NOW })
}

fn preset(id: &str, current: &str) -> Value {
    let mut settings = presets().into_iter().find(|p| p["id"] == id).expect("preset")["settings"].clone();
    settings["secrets"] = json!({ "current": current });
    settings
}

fn code(result: Result<(), Refusal>) -> &'static str {
    result.err().map_or("OK", |r| r.code)
}

#[test]
fn githubs_published_example_passes_the_github_preset() {
    // docs.github.com, "Validating webhook deliveries": the test values.
    let header = "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";
    let settings = preset("github", "It's a Secret to Everybody");
    assert_eq!(code(check(settings.clone(), &[("X-Hub-Signature-256", header)], b"Hello, World!")), "OK");
    assert_eq!(code(check(settings, &[("X-Hub-Signature-256", header)], b"Hello, World?")), "HMAC_SIGNATURE_MISMATCH");
}

/// A timestamp in its own header, `{timestamp}.{body}` signed, `v1=` before
/// the hex: the format no fixed provider covered.
fn timestamped() -> Value {
    json!({
        "secrets": { "current": "vendor-secret" },
        "signed": "{timestamp}.{body}",
        "signature": { "from": "header:X-Vendor-Signature", "prefix": "v1=" },
        "timestamp": { "from": "header:X-Vendor-Timestamp" }
    })
}

#[test]
fn a_timestamp_header_and_a_template_check_a_vendor_no_preset_knows() {
    let body = br#"{"event":"login"}"#;
    let ts = NOW.to_string();
    let good = format!("v1={}", hex::encode(sign(b"vendor-secret", format!("{ts}.").as_bytes().iter().chain(body).copied().collect::<Vec<_>>().as_slice())));
    let headers = [("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", good.as_str())];
    assert_eq!(code(check(timestamped(), &headers, body)), "OK");

    let old = (NOW - 301).to_string();
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", old.as_str()), ("X-Vendor-Signature", good.as_str())], body)), "HMAC_TIMESTAMP_STALE");
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Signature", good.as_str())], body)), "HMAC_TIMESTAMP_MISSING");
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", "soon"), ("X-Vendor-Signature", good.as_str())], body)), "HMAC_TIMESTAMP_INVALID");
    let other_prefix = good.replacen("v1=", "sha256=", 1);
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", other_prefix.as_str())], body)), "HMAC_PREFIX_MISSING");
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", "v1=zz")], body)), "HMAC_SIGNATURE_UNREADABLE");
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", ts.as_str())], body)), "HMAC_HEADER_MISSING");
    // Another timestamp under the same signature: the timestamp is signed too.
    let next = (NOW + 1).to_string();
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", next.as_str()), ("X-Vendor-Signature", good.as_str())], body)), "HMAC_SIGNATURE_MISMATCH");
}

#[test]
fn a_refusal_names_the_header_but_never_a_value() {
    let refusal = check(timestamped(), &[("X-Vendor-Timestamp", "1"), ("X-Vendor-Signature", "v1=abcd")], b"x").unwrap_err();
    assert!(refusal.message.contains("300"), "{}", refusal.message);
    let refusal = check(timestamped(), &[("X-Vendor-Timestamp", &NOW.to_string()), ("X-Vendor-Signature", "v1=abcd")], b"x").unwrap_err();
    assert!(!refusal.message.contains("abcd") && !refusal.message.contains("vendor-secret"), "{}", refusal.message);
}

#[test]
fn stripe_accepts_any_v1_in_the_header() {
    let body = b"{\"id\":\"evt_1\"}";
    let ts = NOW.to_string();
    let message: Vec<u8> = format!("{ts}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let right = hex::encode(sign(b"whsec_stripe", &message));
    let wrong = hex::encode(sign(b"another", &message));
    let both = format!("t={ts},v1={wrong},v1={right},v0=00");
    assert_eq!(code(check(preset("stripe", "whsec_stripe"), &[("Stripe-Signature", both.as_str())], body)), "OK");
    let only_wrong = format!("t={ts},v1={wrong}");
    assert_eq!(code(check(preset("stripe", "whsec_stripe"), &[("Stripe-Signature", only_wrong.as_str())], body)), "HMAC_SIGNATURE_MISMATCH");
    let no_t = format!("v1={right}");
    assert_eq!(code(check(preset("stripe", "whsec_stripe"), &[("Stripe-Signature", no_t.as_str())], body)), "HMAC_TIMESTAMP_MISSING");
}

#[test]
fn the_previous_secret_still_passes_while_a_secret_is_replaced() {
    let key_old = b"old-key-bytes-000";
    let key_new = b"new-key-bytes-111";
    let body = b"payload";
    let ts = NOW.to_string();
    let message: Vec<u8> = format!("msg_1.{ts}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let signed_with_old = format!("v1a,ignored v1,{}", b64(&sign(key_old, &message)));
    let mut settings = preset("standard-webhooks", &format!("whsec_{}", b64(key_new)));
    let headers = [("webhook-id", "msg_1"), ("webhook-timestamp", ts.as_str()), ("webhook-signature", signed_with_old.as_str())];
    assert_eq!(code(check(settings.clone(), &headers, body)), "HMAC_SIGNATURE_MISMATCH");
    settings["secrets"]["previous"] = json!(format!("whsec_{}", b64(key_old)));
    assert_eq!(code(check(settings.clone(), &headers, body)), "OK");
    // The id is part of what is signed.
    let other_id = [("webhook-id", "msg_2"), ("webhook-timestamp", ts.as_str()), ("webhook-signature", signed_with_old.as_str())];
    assert_eq!(code(check(settings, &other_id, body)), "HMAC_SIGNATURE_MISMATCH");
}

#[test]
fn a_text_secret_is_the_key_byte_for_byte() {
    let body = b"x";
    let ts = NOW.to_string();
    let message: Vec<u8> = format!("{ts}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let spaced = " key with spaces ";
    let sig = format!("v1={}", hex::encode(sign(spaced.as_bytes(), &message)));
    let mut settings = timestamped();
    settings["secrets"]["current"] = json!(spaced);
    assert_eq!(code(check(settings, &[("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", sig.as_str())], body)), "OK");
}

#[test]
fn the_body_is_signed_as_its_raw_bytes_even_when_it_is_not_text() {
    let body: &[u8] = &[0xff, 0xfe, 0x00, 0x41];
    let ts = NOW.to_string();
    let raw: Vec<u8> = format!("{ts}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let good = format!("v1={}", hex::encode(sign(b"vendor-secret", &raw)));
    let as_if_empty = format!("v1={}", hex::encode(sign(b"vendor-secret", format!("{ts}.").as_bytes())));
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", good.as_str())], body)), "OK");
    assert_eq!(code(check(timestamped(), &[("X-Vendor-Timestamp", ts.as_str()), ("X-Vendor-Signature", as_if_empty.as_str())], body)), "HMAC_SIGNATURE_MISMATCH");
}

#[test]
fn paddle_square_and_millisecond_timestamps() {
    let body = b"{}";
    let ts = NOW.to_string();
    let paddle_msg: Vec<u8> = format!("{ts}:").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let paddle = format!("ts={ts};h1={}", hex::encode(sign(b"pdl_key", &paddle_msg)));
    assert_eq!(code(check(preset("paddle", "pdl_key"), &[("Paddle-Signature", paddle.as_str())], body)), "OK");

    let url = "https://shop.example/hooks/square";
    let square_msg: Vec<u8> = url.as_bytes().iter().chain(body.iter()).copied().collect();
    let square = b64(&sign(b"sq_key", &square_msg));
    assert_eq!(code(check_at(preset("square", "sq_key"), &[("x-square-hmacsha256-signature", square.as_str())], body, url)), "OK");
    assert_eq!(code(check_at(preset("square", "sq_key"), &[("x-square-hmacsha256-signature", square.as_str())], body, "http://shop.example/hooks/square")), "HMAC_SIGNATURE_MISMATCH");

    let mut ms = timestamped();
    ms["timestamp"]["unit"] = json!("milliseconds");
    let ts_ms = (NOW * 1000 + 250).to_string();
    let msg: Vec<u8> = format!("{ts_ms}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let sig = format!("v1={}", hex::encode(sign(b"vendor-secret", &msg)));
    assert_eq!(code(check(ms, &[("X-Vendor-Timestamp", ts_ms.as_str()), ("X-Vendor-Signature", sig.as_str())], body)), "OK");
}

#[test]
fn a_credential_may_hold_only_the_key_for_signing_nodes() {
    let key_only = json!({ "secrets": { "current": "k" } });
    assert!(validate(&key_only).is_ok(), "a key alone is a valid hmac credential");
    assert_eq!(key(&key_only).unwrap(), b"k");
    let encoded = json!({ "secrets": { "current": "whsec_aGk=" }, "secret_encoding": "base64", "secret_prefix": "whsec_" });
    assert_eq!(key(&encoded).unwrap(), b"hi");
    // At the webhook's door it says what is missing.
    assert!(Settings::from_value(&key_only).unwrap_err().contains("no signature check"));
    // The old single secret is refused by name, for signing too.
    assert!(key(&json!({ "secret": made_key("old") })).unwrap_err().contains("before 0.11.28"));
    // Any check setting makes the whole check required.
    assert!(validate(&json!({ "secrets": { "current": "k" }, "signature": { "from": "header:X-Sig" } })).is_err());
}

#[test]
fn settings_are_refused_when_saved_not_when_a_request_arrives() {
    let refused = |value: Value| Settings::from_value(&value).unwrap_err();
    assert!(refused(json!({ "provider": "github", "secret": made_key("old") })).contains("before 0.11.28"));
    let mut unknown = timestamped();
    unknown["colour"] = json!("red");
    assert!(refused(unknown).contains("colour"));
    let mut no_ts = timestamped();
    no_ts.as_object_mut().unwrap().remove("timestamp");
    assert!(refused(no_ts).contains("{timestamp}"));
    let mut part_without_parts = timestamped();
    part_without_parts["signature"]["from"] = json!("part:v1");
    assert!(refused(part_without_parts).contains("parts"));
    let mut zero = timestamped();
    zero["timestamp"]["tolerance"] = json!(0);
    assert!(refused(zero).contains("tolerance"));
    let mut empty = timestamped();
    empty["secrets"] = json!({ "current": "" });
    assert!(refused(empty).contains("current"));
    let mut bad_b64 = timestamped();
    bad_b64["secret_encoding"] = json!("base64");
    bad_b64["secrets"] = json!({ "current": "not base64!" });
    assert!(refused(bad_b64).contains("base64"));
    let mut slot = timestamped();
    slot["secrets"]["next"] = json!("x");
    assert!(refused(slot).contains("next"));
    // A timestamp the signature does not cover could be replaced.
    let mut unsigned_time = timestamped();
    unsigned_time["signed"] = json!("{body}");
    assert!(refused(unsigned_time).contains("not signed"));
    // ... unless the same header is signed under its own name.
    let mut by_header = timestamped();
    by_header["signed"] = json!("{header:X-Vendor-Timestamp}.{body}");
    assert!(Settings::from_value(&by_header).is_ok());
    // A mistyped setting is refused, never read as its default.
    for (field, value) in [("algorithm", json!(512)), ("secret_encoding", json!(true)), ("preset", json!(1))] {
        let mut wrong = timestamped();
        wrong[field] = value;
        assert!(refused(wrong).contains(field), "{field}");
    }
    let mut prefix = timestamped();
    prefix["signature"]["prefix"] = json!(1);
    assert!(refused(prefix).contains("signature.prefix"));
    let mut previous = timestamped();
    previous["secrets"]["previous"] = json!(123);
    assert!(refused(previous).contains("secrets.previous"));
    let mut unit = timestamped();
    unit["timestamp"]["unit"] = json!(5);
    assert!(refused(unit).contains("timestamp.unit"));
}

#[test]
fn a_credential_from_before_0_11_28_converts_to_the_same_check() {
    assert!(from_old_shape(&timestamped()).is_none(), "the new shape is left alone");

    // generic / Meta: sha256= over the raw body.
    let app_key = made_key("app");
    let old = json!({ "provider": "generic", "secret": app_key, "signature_header": "X-Hub-Signature-256", "signature_prefix": "sha256=", "signature_encoding": "hex", "algorithm": "sha256", "replay_tolerance": 0 });
    let converted = from_old_shape(&old).expect("old shape");
    let body = b"{\"entry\":[]}";
    let sig = format!("sha256={}", hex::encode(sign(app_key.as_bytes(), body)));
    assert_eq!(code(check(converted, &[("X-Hub-Signature-256", sig.as_str())], body)), "OK");

    // stripe with a window, and without one.
    let ts = NOW.to_string();
    let message: Vec<u8> = format!("{ts}.").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let stripe_key = made_key("whsec");
    let header = format!("t={ts},v1={}", hex::encode(sign(stripe_key.as_bytes(), &message)));
    for tolerance in [300, 0] {
        let old = json!({ "provider": "stripe", "secret": stripe_key, "signature_header": "Stripe-Signature", "replay_tolerance": tolerance });
        let converted = from_old_shape(&old).expect("old stripe");
        assert_eq!(code(check(converted, &[("Stripe-Signature", header.as_str())], body)), "OK", "tolerance {tolerance}");
    }

    // slack.
    let slack_msg: Vec<u8> = format!("v0:{ts}:").into_bytes().into_iter().chain(body.iter().copied()).collect();
    let slack_key = made_key("slack");
    let slack_sig = format!("v0={}", hex::encode(sign(slack_key.as_bytes(), &slack_msg)));
    let old = json!({ "provider": "slack", "secret": slack_key, "signature_header": "X-Slack-Signature", "signature_prefix": "v0=", "replay_tolerance": 300 });
    let converted = from_old_shape(&old).expect("old slack");
    assert_eq!(code(check(converted, &[("X-Slack-Signature", slack_sig.as_str()), ("X-Slack-Request-Timestamp", ts.as_str())], body)), "OK");

    // A signing-only credential: the key moves to secrets.current.
    let signing_key = made_key("sign");
    let converted = from_old_shape(&json!({ "secret": signing_key })).expect("old key");
    assert_eq!(key(&converted).unwrap(), signing_key.as_bytes());
}
