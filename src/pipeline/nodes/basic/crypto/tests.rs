//! One test per behaviour of each `crypto.*` kind. The HMAC key lives in an
//! invented `hmac` credential of a throwaway platform; nothing here is a real
//! secret.

use std::sync::Arc;

use serde_json::{Value, json};

use super::{base64, build, digest, password, random, signature};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};
use crate::platform::model::{PlatformConfig, UpsertProjectCredentialRequest};
use crate::platform::services::PlatformService;

const TEST_KEY: &str = "invented-test-key-not-a-secret";

fn input(payload: Value) -> NodeExecutionInput {
    NodeExecutionInput {
        node_id: "n1".to_string(),
        input_pin: "in".to_string(),
        payload,
        metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
        bus: None,
    }
}

/// Build `kind` from `config` and run it once on `{ "kept": 1 }`.
async fn run(kind: &str, config: Value, platform: Option<&Arc<PlatformService>>) -> Result<NodeExecutionOutput, crate::pipeline::PipelineError> {
    let node = build(kind, &config, platform.map(|p| p.credentials.clone()))?.expect("a crypto kind");
    assert_eq!(node.kind(), kind);
    node.execute_async(input(json!({ "kept": 1 }))).await
}

fn refusal(kind: &str, config: Value) -> &'static str {
    match build(kind, &config, None) {
        Err(err) => err.code,
        Ok(_) => panic!("{kind} accepted {config}"),
    }
}

fn platform() -> (Arc<PlatformService>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut cfg = PlatformConfig::default();
    cfg.data_root = dir.path().to_path_buf();
    let p = Arc::new(PlatformService::from_config(cfg).expect("platform"));
    for (id, kind, key) in [("sig", "hmac", TEST_KEY), ("rfc", "hmac", "Jefe"), ("jwt", "jwt_signing_key", TEST_KEY)] {
        p.credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: id.to_string(),
                    title: id.to_string(),
                    kind: kind.to_string(),
                    secret: json!({ "secret": key, "algorithm": "HS256" }),
                    notes: String::new(),
                },
            )
            .expect("credential");
    }
    (p, dir)
}

#[test]
fn the_family_is_eight_kinds_with_their_signatures() {
    let signatures: Vec<String> = super::definitions().iter().map(crate::pipeline::nodes::node_signature).collect();
    assert_eq!(
        signatures,
        [
            "crypto.password.hash --from TEXT [--algorithm argon2|bcrypt] [--cost N] → password",
            "crypto.password.verify --from TEXT --hash TEXT → password",
            "crypto.digest.create --text TEXT [--algorithm sha256|sha512] → digest",
            "crypto.signature.sign --text TEXT --credential TEXT [--algorithm hmac-sha256] → signature",
            "crypto.signature.verify --text TEXT --credential TEXT [--algorithm hmac-sha256] --signature TEXT → signature",
            "crypto.base64.encode --text TEXT → base64",
            "crypto.base64.decode --from TEXT → base64",
            "crypto.random.generate [--size SIZE] [--encoding hex|base64] → random",
        ]
    );
    assert_eq!(super::KINDS.len(), signatures.len());
    assert!(build("n.crypto", &json!({}), None).expect("not an error").is_none());
}

#[tokio::test]
async fn a_password_hashes_and_verifies_with_both_algorithms() {
    for (algorithm, extra, prefix) in [("argon2", json!(null), "$argon2id$"), ("bcrypt", json!("4"), "$2b$04$")] {
        let hashed = run(password::HASH_KIND, json!({ "from": "correct horse", "algorithm": algorithm, "cost": extra }), None).await.expect("hash");
        assert_eq!(hashed.output_pins, ["out"]);
        assert_eq!(hashed.payload["kept"], 1, "the rest of the payload stays");
        assert_eq!(hashed.payload["password"]["algorithm"], algorithm);
        let hash = hashed.payload["password"]["hash"].as_str().expect("hash").to_string();
        assert!(hash.starts_with(prefix), "{hash}");

        let good = run(password::VERIFY_KIND, json!({ "from": "correct horse", "hash": hash }), None).await.expect("verify");
        assert_eq!((good.output_pins.as_slice(), &good.payload["password"]), (&["true".to_string()][..], &json!({ "valid": true })));
        assert_eq!(good.payload["kept"], 1);

        let wrong = run(password::VERIFY_KIND, json!({ "from": "wrong horse", "hash": hash }), None).await.expect("a wrong password is an answer");
        assert_eq!((wrong.output_pins.as_slice(), &wrong.payload["password"]), (&["false".to_string()][..], &json!({ "valid": false })));
    }
}

#[test]
fn an_empty_or_unknown_hash_and_a_misplaced_cost_are_refused() {
    let empty = password::VERIFY_EMPTY_CODE;
    assert_eq!(refusal(password::VERIFY_KIND, json!({ "from": "pw", "hash": "" })), empty);
    assert_eq!(refusal(password::VERIFY_KIND, json!({ "from": "pw", "hash": null })), empty, "an unknown user's missing hash");
    assert_eq!(refusal(password::VERIFY_KIND, json!({ "from": "", "hash": "$argon2id$x" })), empty);
    assert_eq!(refusal(password::VERIFY_KIND, json!({ "from": "pw", "hash": "5f4dcc3b5aa765d61d8327deb882cf99" })), password::VERIFY_FORMAT_CODE);
    assert_eq!(refusal(password::HASH_KIND, json!({ "from": "pw", "cost": 10 })), password::HASH_CONFIG_CODE, "argon2 takes no cost");
    assert_eq!(refusal(password::HASH_KIND, json!({ "from": "pw", "algorithm": "bcrypt", "cost": 3 })), password::HASH_CONFIG_CODE);
    assert_eq!(refusal(password::HASH_KIND, json!({ "from": "pw", "algorithm": "md5" })), password::HASH_CONFIG_CODE);
    assert_eq!(refusal(password::HASH_KIND, json!({ "from": "" })), password::HASH_EMPTY_CODE);
}

#[tokio::test]
async fn a_malformed_hash_past_its_prefix_is_refused_not_false() {
    for hash in ["$argon2id$garbage", "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ", "$2b$12$short"] {
        let err = run(password::VERIFY_KIND, json!({ "from": "pw", "hash": hash }), None).await.unwrap_err();
        assert_eq!(err.code, password::VERIFY_FORMAT_CODE, "{hash}");
    }
}

#[tokio::test]
async fn digests_match_the_known_vectors() {
    // FIPS 180-2 test vectors for "abc".
    let sha256 = run(digest::NODE_KIND, json!({ "text": "abc" }), None).await.expect("sha256");
    assert_eq!(
        sha256.payload["digest"],
        json!({ "algorithm": "sha256", "value": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" })
    );
    assert_eq!(sha256.payload["kept"], 1);
    let sha512 = run(digest::NODE_KIND, json!({ "text": "abc", "algorithm": "sha512" }), None).await.expect("sha512");
    assert_eq!(
        sha512.payload["digest"]["value"],
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
    assert_eq!(refusal(digest::NODE_KIND, json!({ "text": "" })), digest::EMPTY_CODE);
    assert_eq!(refusal(digest::NODE_KIND, json!({ "text": "abc", "algorithm": "md5" })), digest::CONFIG_CODE);
    assert_eq!(refusal(digest::NODE_KIND, json!({ "text": { "a": 1 } })), digest::CONFIG_CODE, "an object is not text");
}

#[tokio::test]
async fn a_signature_round_trips_with_an_hmac_credential() {
    let (p, _dir) = platform();
    // RFC 4231 test case 2: key "Jefe".
    let rfc = run(signature::SIGN_KIND, json!({ "text": "what do ya want for nothing?", "credential_id": "rfc" }), Some(&p)).await.expect("rfc");
    assert_eq!(rfc.payload["signature"]["value"], "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");

    let signed = run(signature::SIGN_KIND, json!({ "text": "payload", "credential_id": "sig" }), Some(&p)).await.expect("sign");
    let expected = signed.payload["signature"]["value"].as_str().expect("hex").to_string();
    assert_eq!(signed.payload["signature"], json!({ "algorithm": "hmac-sha256", "value": expected }));
    assert_eq!(signed.payload["kept"], 1);

    for (text, given, valid) in [
        ("payload", expected.clone(), true),
        ("payload", expected.to_uppercase(), true),
        ("tampered", expected.clone(), false),
        ("payload", "not hex".to_string(), false),
    ] {
        let out = run(signature::VERIFY_KIND, json!({ "text": text, "credential_id": "sig", "signature": given }), Some(&p))
            .await
            .expect("verify");
        assert_eq!(out.payload["signature"], json!({ "valid": valid }), "{text} {given}");
        assert_eq!(out.output_pins, [if valid { "true" } else { "false" }]);
    }

    let wrong_kind = run(signature::SIGN_KIND, json!({ "text": "payload", "credential_id": "jwt" }), Some(&p)).await.unwrap_err();
    assert_eq!(wrong_kind.code, signature::SIGN_CREDENTIAL_CODE);
    let missing = run(signature::VERIFY_KIND, json!({ "text": "p", "credential_id": "nope", "signature": "00" }), Some(&p)).await.unwrap_err();
    assert_eq!(missing.code, signature::VERIFY_CREDENTIAL_CODE);
    let empty = build(signature::VERIFY_KIND, &json!({ "text": "p", "credential_id": "sig", "signature": "" }), Some(p.credentials.clone()));
    assert_eq!(empty.err().map(|e| e.code), Some(signature::VERIFY_EMPTY_CODE));
    assert_eq!(refusal(signature::SIGN_KIND, json!({ "text": "p", "credential_id": "sig" })), signature::SIGN_CODE, "no credential service");
}

#[tokio::test]
async fn base64_round_trips_and_refuses_what_is_not_base64() {
    let encoded = run(base64::ENCODE_KIND, json!({ "text": "héllo" }), None).await.expect("encode");
    assert_eq!(encoded.payload["base64"], json!({ "value": "aMOpbGxv" }));
    let decoded = run(base64::DECODE_KIND, json!({ "from": "aMOpbGxv" }), None).await.expect("decode");
    assert_eq!(decoded.payload["base64"], json!({ "text": "héllo" }));
    assert_eq!(decoded.payload["kept"], 1);
    for bad in ["not base64!", "/w=="] {
        let err = run(base64::DECODE_KIND, json!({ "from": bad }), None).await.unwrap_err();
        assert_eq!(err.code, base64::DECODE_INVALID_CODE, "{bad}");
    }
    assert_eq!(refusal(base64::ENCODE_KIND, json!({})), base64::ENCODE_EMPTY_CODE);
    assert_eq!(refusal(base64::DECODE_KIND, json!({ "from": "" })), base64::DECODE_EMPTY_CODE);
}

#[tokio::test]
async fn random_takes_a_size_with_a_unit_and_an_encoding() {
    let default = run(random::NODE_KIND, json!({}), None).await.expect("default");
    let value = default.payload["random"]["value"].as_str().expect("value");
    assert_eq!(value.len(), 64, "32 bytes as hex");
    assert!(value.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(default.payload["kept"], 1);

    let b64 = run(random::NODE_KIND, json!({ "size": "16B", "encoding": "base64" }), None).await.expect("base64");
    let value = b64.payload["random"]["value"].as_str().expect("value");
    use ::base64::Engine as _;
    assert_eq!(::base64::engine::general_purpose::STANDARD.decode(value).expect("base64").len(), 16);

    let ceiling = run(random::NODE_KIND, json!({ "size": "1KiB" }), None).await.expect("1KiB");
    assert_eq!(ceiling.payload["random"]["value"].as_str().map(str::len), Some(2048));

    for bad in [json!({ "size": "32" }), json!({ "size": "0B" }), json!({ "size": "2KiB" }), json!({ "encoding": "base32" })] {
        assert_eq!(refusal(random::NODE_KIND, bad.clone()), random::CONFIG_CODE, "{bad}");
    }
}
