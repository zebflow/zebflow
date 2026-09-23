use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use base64::Engine as _;
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::infra::cluster::config::ClusterRole;
use zebflow::infra::cluster::security::{
    ControllerSigningKey, JoinToken, OfficeVouch, verify_registration_proof,
};
use zebflow::platform::model::{
    CollectionAttribute, CreateHubTokenRequest, CreateSimpleTableRequest,
    ZebflowJsonDistributionHub,
};
use zebflow::platform::sekejap;
use zebflow::platform::{
    CreateProjectRequest, CreateUserRequest, PlatformConfig, PlatformService, ProjectAccessSubject,
    ProjectCapability, build_router,
};

fn temp_test_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}"))
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    serde_json::from_slice(&body).expect("json body")
}

async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    String::from_utf8(body.to_vec()).expect("utf8 response body")
}

async fn login_cookie(app: axum::Router, identifier: &str, password: &str) -> String {
    let login = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "identifier={identifier}&password={password}"
                )))
                .expect("request"),
        )
        .await
        .expect("login response");
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    login
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string()
}

#[test]
fn first_bootstrap_generates_password_and_restart_preserves_it() {
    let data_root = temp_test_dir("generated-superadmin-password");
    let mut config = PlatformConfig::default();
    config.data_root = data_root.clone();

    let platform = PlatformService::from_config(config).expect("first platform bootstrap");
    let password_path = data_root.join(".bootstrap/superadmin-password");
    let generated = fs::read_to_string(&password_path)
        .expect("generated superadmin password")
        .trim()
        .to_string();
    assert_eq!(generated.len(), 48);
    assert!(
        platform
            .auth
            .login("superadmin", &generated)
            .expect("authenticate generated password")
            .is_some()
    );
    drop(platform);

    let mut restarted_config = PlatformConfig::default();
    restarted_config.data_root = data_root.clone();
    restarted_config.default_password = "must-not-replace-existing-password".to_string();
    let restarted =
        PlatformService::from_config(restarted_config).expect("restarted platform bootstrap");
    assert!(
        restarted
            .auth
            .login("superadmin", &generated)
            .expect("authenticate preserved password")
            .is_some()
    );
    assert!(
        !restarted
            .auth
            .login("superadmin", "must-not-replace-existing-password")
            .expect("reject replacement password")
            .is_some()
    );
    drop(restarted);

    let _ = fs::remove_dir_all(data_root);
}

#[tokio::test]
async fn office_refuses_to_start_without_cluster_configuration_and_starts_with_it() {
    let unconfigured_root = temp_test_dir("office-without-cluster-config");
    let mut unconfigured = PlatformConfig::default();
    unconfigured.data_root = unconfigured_root.clone();
    unconfigured.cluster.role = ClusterRole::Worker;
    // The binary always fills advertise_url from its own listen address, so the
    // realistic deployment failure is the two variables only an operator supplies.
    unconfigured.cluster.advertise_url = Some("http://office-a:10610".to_string());

    let refused = build_router(unconfigured)
        .await
        .err()
        .expect("office without a controller URL must refuse to start");
    assert_eq!(refused.code, "CLUSTER_CONFIG_INCOMPLETE");
    assert!(
        refused.message.contains("ZEBFLOW_CLUSTER_MASTER_URL")
            && refused.message.contains("ZEBFLOW_CLUSTER_JOIN_TOKEN"),
        "refusal must name every missing variable at once: {}",
        refused.message
    );
    assert!(
        !unconfigured_root.exists(),
        "a refused office must not create its data root"
    );

    let configured_root = temp_test_dir("office-with-cluster-config");
    let mut configured = PlatformConfig::default();
    configured.data_root = configured_root.clone();
    configured.cluster.role = ClusterRole::Worker;
    configured.cluster.advertise_url = Some("http://office-a:10610".to_string());
    // Unreachable on purpose: registration retries in the background and must not
    // stop the office from serving.
    configured.cluster.master_url = Some("http://127.0.0.1:1".to_string());
    // A real minted token, because `offices.md` §8's token is per-office and
    // self-describing; the shared environment secret this replaced would now
    // refuse to parse.
    configured.cluster.join_token = Some(unissued_token("office-a"));

    let app = build_router(configured)
        .await
        .expect("office with complete cluster configuration starts");
    let health = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(response_json(health).await["status"], json!("ok"));

    // `offices.md` §4 keeps institutions on both sides of a join: "every office
    // seeds its own blessed shelf and data root, joined or not; identical bytes
    // for the same release". A standalone instance of the same binary is the
    // yardstick — the office must hold exactly the same coordinates.
    let standalone_root = temp_test_dir("standalone-beside-an-office");
    let mut standalone = PlatformConfig::default();
    standalone.data_root = standalone_root.clone();
    let _standalone = build_router(standalone)
        .await
        .expect("standalone instance starts");

    let office_shelf = blessed_shelf_coordinates(&configured_root);
    assert_eq!(
        office_shelf,
        blessed_shelf_coordinates(&standalone_root),
        "a joined office's blessed shelf must match a standalone instance's"
    );
    assert!(
        !office_shelf.is_empty(),
        "the release seeds a non-empty blessed shelf"
    );

    // §6 re-enables an account that is "disabled, not merely unknown", and §7
    // makes a detached office "a complete instance the moment it leaves".
    // Neither is possible on an office that never created a local account, so
    // the first-boot defaults run for an office exactly as they do standalone.
    assert!(
        configured_root
            .join("users")
            .join(PlatformConfig::default().default_owner)
            .join(PlatformConfig::default().default_project)
            .is_dir(),
        "an office creates its own default owner and project"
    );

    let _ = fs::remove_dir_all(configured_root);
    let _ = fs::remove_dir_all(standalone_root);
}

/// `offices.md` §8: the token is per-office, issued by the controller, and
/// revocable for one office alone, with proof in both directions.
///
/// The property the whole mechanism exists for is the revocation one — one
/// office stops, every other office keeps working — so it is proven explicitly
/// rather than inferred from a refusal count.
#[tokio::test]
async fn a_join_token_is_per_office_revocable_and_proves_both_directions() {
    let controller_root = temp_test_dir("controller-join-tokens");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    // A controller holds no token of its own; it mints one per office.
    let app = build_router(controller).await.expect("controller starts");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // --- minting records the holder before it records the secret -----------
    let minted = mint_join_token(&app, &cookie, "office-a", false).await;
    assert_eq!(minted.0, StatusCode::CREATED);
    let token_a = minted.1["token"].as_str().expect("token").to_string();
    assert!(token_a.starts_with("zfjoin2:office-a:"), "{token_a}");
    assert_eq!(minted.1["office"]["office_id"], json!("office-a"));
    assert_eq!(minted.1["office"]["status"], json!("planned"));
    assert_eq!(minted.1["record"]["status"], json!("active"));
    // The digest the controller stores never travels. It used to come back in
    // this very response and in the listing below, which put an office's stored
    // material into browser memory and every proxy log on the way.
    assert!(
        minted.1["record"].get("secret_digest").is_none(),
        "the mint response must not carry the stored digest: {}",
        minted.1["record"]
    );
    // The token carries the controller's *public* key, which is what lets the
    // office verify a controller it has never met.
    let parsed_a = JoinToken::parse(&token_a).expect("parse");
    assert!(!parsed_a.controller_verify_key.is_empty());
    let digest_a = parsed_a.secret_digest();
    assert!(
        !token_a.contains(&digest_a),
        "the stored record must be the digest, not the token"
    );

    // The listing never carries a secret, and no longer carries the digest.
    let listed = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/api/platform/cluster/join-tokens")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("list response"),
    )
    .await;
    let listed = serde_json::to_string(&listed["tokens"]).expect("tokens json");
    assert!(
        listed.contains("office-a"),
        "the listing still names the office"
    );
    assert!(
        !listed.contains(&digest_a),
        "a listing must not carry the digest either: {listed}"
    );
    assert!(
        !listed.contains(token_a.rsplit(':').next().expect("secret")),
        "a listing must never carry the secret"
    );

    // Minting again for the same office refuses rather than silently rotating.
    let again = mint_join_token(&app, &cookie, "office-a", false).await;
    assert_eq!(again.0, StatusCode::CONFLICT);
    assert_eq!(again.1["error"]["code"], json!("CLUSTER_JOIN_TOKEN_EXISTS"));

    let token_b = mint_join_token(&app, &cookie, "office-b", false).await.1["token"]
        .as_str()
        .expect("token")
        .to_string();

    // --- an office with its own token registers and heartbeats -------------
    let nonce = "0123456789abcdef";
    let registered = register_office(&app, &token_a, "office-a", nonce).await;
    assert_eq!(registered.0, StatusCode::OK);
    assert_eq!(registered.1["ok"], json!(true));
    let proof = registered.1["proof"].as_str().expect("proof").to_string();

    // The controller half of the mutual proof, verified exactly as the office
    // verifies it: with the public key its token carried, and nothing else.
    let identity_a = JoinToken::parse(&token_a).expect("parse");
    assert!(
        verify_registration_proof(
            &identity_a.controller_verify_key,
            "office-a",
            &identity_a.fingerprint(),
            nonce,
            &proof
        ),
        "the office must accept its own controller's answer"
    );
    // A host that read every byte the controller stores about this office —
    // the digest, and therefore the fingerprint — still cannot answer. Before
    // this release the digest *was* the key, so this is the whole change.
    for stolen in [
        identity_a.secret_digest(),
        identity_a.fingerprint(),
        String::new(),
    ] {
        assert!(
            !verify_registration_proof(
                &identity_a.controller_verify_key,
                "office-a",
                &identity_a.fingerprint(),
                nonce,
                &stolen
            ),
            "stored material must not answer a nonce"
        );
    }
    let (_, impostor) = ControllerSigningKey::generate().expect("key");
    assert!(
        !verify_registration_proof(
            impostor.verify_key(),
            "office-a",
            &identity_a.fingerprint(),
            nonce,
            &proof
        ),
        "one controller's answer must not verify under another's key"
    );

    assert_eq!(
        heartbeat_office(&app, &token_a, "office-a").await.0,
        StatusCode::OK
    );
    // A valid token that has not registered yet passes the door and is turned
    // away by the registry, not by the token check.
    let unregistered = heartbeat_office(&app, &token_b, "office-b").await;
    assert_ne!(unregistered.0, StatusCode::OK);
    assert_eq!(
        unregistered.1["error"]["code"],
        json!("CLUSTER_WORKER_UNKNOWN")
    );
    assert_eq!(
        register_office(&app, &token_b, "office-b", nonce).await.0,
        StatusCode::OK
    );
    assert_eq!(
        heartbeat_office(&app, &token_b, "office-b").await.0,
        StatusCode::OK
    );

    // --- office A's token presented by office B is refused -----------------
    let crossed = register_office(&app, &token_a, "office-b", nonce).await;
    assert_eq!(crossed.0, StatusCode::FORBIDDEN);
    assert_eq!(
        crossed.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_OFFICE_MISMATCH")
    );

    // --- a wrong or malformed token is refused, actionably -----------------
    let legacy = register_office(&app, "join-token", "office-a", nonce).await;
    assert_eq!(legacy.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        legacy.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_MALFORMED")
    );
    let message = legacy.1["error"]["message"].as_str().expect("message");
    assert!(
        message.contains("Mint one on the controller") && message.contains("join-tokens"),
        "refusal must name the fix: {message}"
    );

    let forged = unissued_token("office-a");
    let forged = register_office(&app, &forged, "office-a", nonce).await;
    assert_eq!(forged.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        forged.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_INVALID")
    );

    let unknown = unissued_token("office-z");
    let unknown = register_office(&app, &unknown, "office-z", nonce).await;
    assert_eq!(unknown.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        unknown.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_UNKNOWN")
    );

    // --- revoking one office stops that office and no other ----------------
    let revoked = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/cluster/join-tokens/office-a/revoke")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("revoke response");
    assert_eq!(revoked.status(), StatusCode::OK);
    assert_eq!(
        response_json(revoked).await["record"]["status"],
        json!("revoked")
    );

    let stopped = heartbeat_office(&app, &token_a, "office-a").await;
    assert_eq!(stopped.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        stopped.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_REVOKED")
    );
    // An already-registered office must not re-register its way back in.
    let rejoin = register_office(&app, &token_a, "office-a", nonce).await;
    assert_eq!(rejoin.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        rejoin.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_REVOKED")
    );

    // This is the whole point of the step: office B is untouched.
    assert_eq!(
        heartbeat_office(&app, &token_b, "office-b").await.0,
        StatusCode::OK,
        "revoking one office must not disturb another"
    );

    // Rotation re-issues for the revoked office alone.
    let rotated = mint_join_token(&app, &cookie, "office-a", true).await;
    assert_eq!(rotated.0, StatusCode::CREATED);
    let rotated_token = rotated.1["token"].as_str().expect("token").to_string();
    assert_ne!(rotated_token, token_a);
    assert_eq!(
        register_office(&app, &rotated_token, "office-a", nonce)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        register_office(&app, &token_a, "office-a", nonce).await.0,
        StatusCode::UNAUTHORIZED,
        "the rotated-away token must not work"
    );

    let _ = fs::remove_dir_all(controller_root);
}

/// `offices.md` §2's third verb, §4's login term, and §8's identity-write rule
/// in one pass: an operator authenticated on the controller reaches an office
/// without knowing any password there, exactly once per vouch, and the office's
/// own operator can read what was written into its accounts.
///
/// The boundary this loop deliberately does *not* cross is asserted too. §4
/// says a joined office's local accounts are disabled for login, and building
/// that before this door existed would have made a joined office unreachable by
/// anybody. Local login on the office is therefore proven still live, so the
/// next loop's change is visible as a change.
#[tokio::test]
async fn a_controller_vouch_opens_an_office_once_and_is_logged_there() {
    let controller_root = temp_test_dir("controller-vouch");
    let office_a_root = temp_test_dir("office-a-vouch");
    let office_b_root = temp_test_dir("office-b-vouch");

    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let controller_app = build_router(controller).await.expect("controller starts");
    let controller_cookie = login_cookie(controller_app.clone(), "superadmin", "test-pass").await;

    let token_a = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-a",
        "http://office-a.example:10610",
    )
    .await;
    let token_b = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-b",
        "http://office-b.example:10610",
    )
    .await;

    let office_a = build_office(&office_a_root, &token_a).await;
    let office_b = build_office(&office_b_root, &token_b).await;

    // (h) The boundary, before anything else: this office is joined, so §4's
    // login term has closed its local door. Asserted here rather than assumed,
    // because everything below is about the door that replaces it — a vouch
    // that worked only on an office you could also log into locally would
    // prove nothing about the term.
    let closed = attempt_login(&office_a, "superadmin", "test-pass").await;
    assert_eq!(closed.0, StatusCode::FORBIDDEN);
    assert!(closed.1.contains("zeb admin break-glass"), "{}", closed.1);

    // --- (a) a controller session reaches an office with no password there --
    let (status, minted) = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    assert_eq!(status, StatusCode::OK);
    let vouch = minted["vouch"]["vouch"]
        .as_str()
        .expect("vouch")
        .to_string();
    assert!(
        vouch.starts_with("zfjoin2v:office-a:superadmin:"),
        "{vouch}"
    );
    assert_eq!(minted["vouch"]["ttl_seconds"], json!(120));
    // The controller hands over a place to go, not only a credential.
    assert!(
        minted["vouch"]["redeem_url"]
            .as_str()
            .expect("redeem url")
            .starts_with("http://office-a.example:10610/office/vouch?v="),
        "{minted}"
    );

    let redeemed = redeem_vouch(&office_a, &vouch).await;
    assert_eq!(redeemed.0, StatusCode::OK, "{:?}", redeemed.1);
    assert_eq!(redeemed.1["owner"], json!("superadmin"));
    // The office already holds this owner, so nothing is created and its role
    // is left exactly as the office set it.
    assert_eq!(redeemed.1["action"], json!("linked"));
    // The session this office issued for its own `superadmin`. It is the
    // office's own session — the same cookie `POST /login` issued before §4
    // closed that door — and from here on it is what reads the office's own
    // logs, with the controller uninvolved.
    let office_session = redeemed.2.expect("the office issues its own session");

    // The session is an *ordinary* one: an unmodified downstream handler
    // accepts it, which is the whole reason a vouch ends in a session rather
    // than in a parallel authentication path.
    let profile = office_a
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/profile")
                .header(header::COOKIE, &office_session)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("profile response");
    assert_eq!(profile.status(), StatusCode::OK);
    assert_eq!(
        response_json(profile).await["user"]["owner"],
        json!("superadmin")
    );

    // --- (b) the same vouch a second time is refused ----------------------
    let replayed = redeem_vouch(&office_a, &vouch).await;
    assert_eq!(replayed.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        replayed.1["error"]["code"],
        json!("CLUSTER_VOUCH_ALREADY_REDEEMED")
    );
    assert!(replayed.2.is_none(), "a refused vouch issues no session");

    // --- a vouch may name an identity this office has never held ----------
    // §5's owner-mapping rule governs *joining* a non-empty office. This is a
    // different moment, and §8 presumes the write by requiring it be logged.
    create_user(&controller_app, &controller_cookie, "remote-admin").await;
    let remote_cookie = login_cookie(controller_app.clone(), "remote-admin", "remote-pass").await;
    let (status, minted) = mint_vouch(&controller_app, &remote_cookie, "office-a").await;
    assert_eq!(status, StatusCode::OK);
    // The vouch names the session owner, never a value the caller supplied.
    assert_eq!(minted["vouch"]["identity"], json!("remote-admin"));
    let created = redeem_vouch(&office_a, minted["vouch"]["vouch"].as_str().expect("vouch")).await;
    assert_eq!(created.0, StatusCode::OK, "{:?}", created.1);
    assert_eq!(created.1["action"], json!("created"));
    assert_eq!(created.1["identity_write"]["role"], json!("superadmin"));

    // The created account is reachable by vouch and by nothing else: no
    // password was chosen for it, so it must not be a local back door. On a
    // joined office the local door is shut for everybody, so the interesting
    // half of this — that it stays shut for *this* account once §6 reopens the
    // door for the rest — is proven in
    // `detach_keeps_everything_and_a_vouched_account_never_becomes_a_local_door`.
    let refused_login = attempt_login(&office_a, "remote-admin", "remote-pass").await;
    assert_eq!(refused_login.0, StatusCode::FORBIDDEN);

    // --- (c) an expired vouch is refused -----------------------------------
    // Minted with the controller's own signing key, read from its data root,
    // because the route will never mint a stale one and the property under test
    // is the office's refusal, not the controller's arithmetic. Nothing short
    // of the private key would do: that is the release.
    let signing = controller_signing_key(&controller_root);
    let fingerprint_a = JoinToken::parse(&token_a).expect("parse").fingerprint();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    let expired = OfficeVouch::mint(
        &signing,
        "office-a",
        "superadmin",
        now - 1,
        "expired-nonce",
        &fingerprint_a,
    )
    .render();
    let refused = redeem_vouch(&office_a, &expired).await;
    assert_eq!(refused.0, StatusCode::UNAUTHORIZED);
    assert_eq!(refused.1["error"]["code"], json!("CLUSTER_VOUCH_EXPIRED"));

    // --- (d) a vouch for office A presented to office B is refused ---------
    let (_, for_a) = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    let for_a = for_a["vouch"]["vouch"].as_str().expect("vouch").to_string();
    let crossed = redeem_vouch(&office_b, &for_a).await;
    assert_eq!(crossed.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        crossed.1["error"]["code"],
        json!("CLUSTER_VOUCH_OFFICE_MISMATCH")
    );
    // Office A still accepts it, so the refusal was about the office and not
    // about the vouch having been spoiled.
    assert_eq!(redeem_vouch(&office_a, &for_a).await.0, StatusCode::OK);
    let _ = token_b;

    // --- (e) a tampered proof is refused -----------------------------------
    let (_, honest) = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    let honest = honest["vouch"]["vouch"]
        .as_str()
        .expect("vouch")
        .to_string();
    let tampered = flip_last_hex_digit(&honest);
    assert_ne!(tampered, honest);
    let forged = redeem_vouch(&office_a, &tampered).await;
    assert_eq!(forged.0, StatusCode::UNAUTHORIZED);
    assert_eq!(forged.1["error"]["code"], json!("CLUSTER_VOUCH_INVALID"));
    // Editing the identity is the same forgery: every field is signed.
    let renamed = honest.replace(":superadmin:", ":root:");
    assert_eq!(
        redeem_vouch(&office_a, &renamed).await.1["error"]["code"],
        json!("CLUSTER_VOUCH_INVALID")
    );

    // --- (g) the identity write is readable by the office's own operator ---
    // Read with the office's *local* superadmin session, with the controller
    // uninvolved. §8's term is about who can read it, not that it exists.
    let log = response_json(
        office_a
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/platform/office/identity-writes")
                    .header(header::COOKIE, &office_session)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("identity write log"),
    )
    .await;
    let writes = log["writes"].as_array().expect("writes").clone();
    let created_row = writes
        .iter()
        .find(|row| row["owner"] == json!("remote-admin"))
        .expect("the created account is in the office's own log");
    assert_eq!(created_row["action"], json!("created"));
    assert_eq!(created_row["office_id"], json!("office-a"));
    assert_eq!(created_row["source"], json!("controller-vouch"));
    assert!(
        created_row["detail"]
            .as_str()
            .expect("detail")
            .contains("never by local login"),
        "the log must read as prose an operator can act on: {created_row}"
    );
    assert!(
        writes
            .iter()
            .any(|row| row["owner"] == json!("superadmin") && row["action"] == json!("linked")),
        "an arrival that created nothing is still an arrival"
    );
    // A refused vouch reaches no account, so it writes no row.
    assert!(
        !writes.iter().any(|row| row["owner"] == json!("root")),
        "a forged vouch must not appear in the accounts log"
    );

    // --- (f) revoking the office's token ends vouching for it --------------
    // Nothing separate had to be built: minting a vouch reads the same record
    // and refuses the same status that stops a heartbeat.
    let revoked = controller_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/cluster/join-tokens/office-a/revoke")
                .method("POST")
                .header(header::COOKIE, &controller_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("revoke response");
    assert_eq!(revoked.status(), StatusCode::OK);

    let after_revoke = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    assert_eq!(after_revoke.0, StatusCode::FORBIDDEN);
    assert_eq!(
        after_revoke.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_REVOKED")
    );

    // Rotation goes further: the office's stored secret no longer derives the
    // digest the controller holds, so a vouch minted after a rotation is
    // refused by an office still running on the old token.
    let rotated = mint_join_token(&controller_app, &controller_cookie, "office-a", true).await;
    assert_eq!(rotated.0, StatusCode::CREATED);
    let (_, after_rotation) = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    let after_rotation = after_rotation["vouch"]["vouch"]
        .as_str()
        .expect("vouch")
        .to_string();
    assert_eq!(
        redeem_vouch(&office_a, &after_rotation).await.1["error"]["code"],
        json!("CLUSTER_VOUCH_INVALID"),
        "a rotated token invalidates vouching cryptographically, not by policy"
    );

    // --- (h) the boundary again, at the end --------------------------------
    // Everything above happened — vouches minted, spent, forged, refused, and
    // a token revoked and rotated — and none of it opened the local door. Only
    // §6's host command does that, which its own tests prove.
    let still_closed = attempt_login(&office_a, "superadmin", "test-pass").await;
    assert_eq!(still_closed.0, StatusCode::FORBIDDEN);

    // --- the door must open on a *fresh* office, §5's ordinary case --------
    // An office whose local account is still on its generated password would
    // otherwise land a vouched operator on the change-password screen, asking
    // for a credential they have no way to know. The forced-change gate exists
    // to stop a generated credential being *used*; a vouch uses none.
    let office_c_root = temp_test_dir("office-c-vouch");
    let token_c = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-c",
        "http://office-c.example:10610",
    )
    .await;
    let mut office_c_config = PlatformConfig::default();
    office_c_config.data_root = office_c_root.clone();
    office_c_config.cluster.role = ClusterRole::Worker;
    office_c_config.cluster.advertise_url = Some("http://office-c.example:10610".to_string());
    office_c_config.cluster.master_url = Some("http://127.0.0.1:1".to_string());
    office_c_config.cluster.join_token = Some(token_c);
    // No default password: this office generated one and nobody has chosen one.
    let office_c = build_router(office_c_config)
        .await
        .expect("fresh office starts");
    assert!(
        office_c_root
            .join(".bootstrap/superadmin-password")
            .exists(),
        "the fresh office's credential really is a generated one"
    );

    let (_, fresh) = mint_vouch(&controller_app, &controller_cookie, "office-c").await;
    let fresh_vouch = fresh["vouch"]["vouch"].as_str().expect("vouch").to_string();
    let landed = office_c
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/office/vouch?v={}",
                    fresh_vouch.replace(':', "%3A")
                ))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("browser redemption");
    assert_eq!(landed.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        landed
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/home"),
        "a vouched operator lands in the office, not on a password screen"
    );
    let fresh_session = landed
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .expect("session cookie")
        .to_string();
    let home = office_c
        .clone()
        .oneshot(
            Request::builder()
                .uri("/home")
                .header(header::COOKIE, &fresh_session)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("home response");
    assert_eq!(
        home.status(),
        StatusCode::OK,
        "the change-password gate must not bounce a session that used no password"
    );

    // A standalone instance has no controller, so nobody vouches there.
    let standalone_root = temp_test_dir("standalone-vouch");
    let mut standalone = PlatformConfig::default();
    standalone.data_root = standalone_root.clone();
    standalone.default_password = "test-pass".to_string();
    let standalone_app = build_router(standalone).await.expect("standalone starts");
    let nowhere = redeem_vouch(&standalone_app, &vouch).await;
    assert_eq!(nowhere.0, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(
        nowhere.1["error"]["code"],
        json!("CLUSTER_VOUCH_NOT_AN_OFFICE")
    );

    let _ = fs::remove_dir_all(controller_root);
    let _ = fs::remove_dir_all(office_a_root);
    let _ = fs::remove_dir_all(office_b_root);
    let _ = fs::remove_dir_all(office_c_root);
    let _ = fs::remove_dir_all(standalone_root);
}

/// A well-formed token that no controller ever issued.
///
/// Shaped correctly, under a key generated here and held by nobody, so it
/// exercises the *record* check rather than the parser.
fn unissued_token(office_id: &str) -> String {
    let (_, key) = ControllerSigningKey::generate().expect("key");
    JoinToken::mint(office_id, key.verify_key()).render()
}

/// The private key one controller keeps in its own data root.
///
/// Read from disk rather than reconstructed, because the point of the release
/// is that it *cannot* be reconstructed from anything the controller publishes.
/// A test that wants to mint what the route will not mint has to be the
/// controller.
fn controller_signing_key(data_root: &Path) -> ControllerSigningKey {
    let document = fs::read(data_root.join("platform").join("cluster-signing-key"))
        .expect("the controller's signing key");
    ControllerSigningKey::from_pkcs8(&document).expect("signing key")
}

async fn build_office(data_root: &Path, token: &str) -> axum::Router {
    let mut config = PlatformConfig::default();
    config.data_root = data_root.to_path_buf();
    config.cluster.role = ClusterRole::Worker;
    config.cluster.advertise_url = Some("http://office.example:10610".to_string());
    // Unreachable on purpose. A vouch is self-authenticating, so redemption
    // must work with the controller gone — §3's "the controller's death costs
    // logins, never execution" would be false if the door needed it alive.
    config.cluster.master_url = Some("http://127.0.0.1:1".to_string());
    config.cluster.join_token = Some(token.to_string());
    config.default_password = "test-pass".to_string();
    build_router(config).await.expect("office starts")
}

async fn mint_join_token_at(
    app: &axum::Router,
    cookie: &str,
    office_id: &str,
    base_url: &str,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/cluster/join-tokens")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "office_id": office_id, "base_url": base_url }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("mint response");
    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["token"]
        .as_str()
        .expect("token")
        .to_string()
}

async fn mint_vouch(app: &axum::Router, cookie: &str, office_id: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/platform/cluster/offices/{office_id}/vouch"))
                .method("POST")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("vouch response");
    let status = response.status();
    (status, response_json(response).await)
}

/// Redeem one vouch, returning the refusal or the session the office issued.
async fn redeem_vouch(app: &axum::Router, vouch: &str) -> (StatusCode, Value, Option<String>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/office/vouch")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "vouch": vouch }).to_string()))
                .expect("request"),
        )
        .await
        .expect("redeem response");
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    (status, response_json(response).await, cookie)
}

async fn create_user(app: &axum::Router, cookie: &str, owner: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/users")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "owner": owner,
                        "password": "remote-pass",
                        "role": "superadmin",
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create user response");
    assert!(
        response.status().is_success(),
        "creating the controller-side operator: {}",
        response.status()
    );
}

fn flip_last_hex_digit(vouch: &str) -> String {
    let mut chars: Vec<char> = vouch.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == '0' { '1' } else { '0' };
    chars.into_iter().collect()
}

async fn mint_join_token(
    app: &axum::Router,
    cookie: &str,
    office_id: &str,
    rotate: bool,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/cluster/join-tokens")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "office_id": office_id,
                        // An office needs an address to be minted at all
                        // (`kinds/office-topology/README.md`, Rejections).
                        "base_url": format!("https://{office_id}.example.test"),
                        "rotate": rotate,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("mint response");
    let status = response.status();
    (status, response_json(response).await)
}

async fn register_office(
    app: &axum::Router,
    token: &str,
    node_id: &str,
    nonce: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/workers/register")
                .method("POST")
                .header("x-zebflow-cluster-token", token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "node_id": node_id,
                        "label": node_id,
                        "base_url": format!("http://{node_id}:10610"),
                        "nonce": nonce,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("register response");
    let status = response.status();
    (status, response_json(response).await)
}

async fn heartbeat_office(app: &axum::Router, token: &str, node_id: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/workers/heartbeat")
                .method("POST")
                .header("x-zebflow-cluster-token", token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "node_id": node_id, "status": "online" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("heartbeat response");
    let status = response.status();
    (status, response_json(response).await)
}

/// Every `{package}@{version}` coordinate on one instance's blessed shelf.
fn blessed_shelf_coordinates(data_root: &Path) -> Vec<String> {
    let packages_dir = data_root
        .join("services")
        .join("hub-local")
        .join("packages");
    let mut coordinates = Vec::new();
    let Ok(packages) = fs::read_dir(&packages_dir) else {
        return coordinates;
    };
    for package in packages.flatten() {
        let package_id = package.file_name().to_string_lossy().to_string();
        let Ok(versions) = fs::read_dir(package.path().join("versions")) else {
            continue;
        };
        for version in versions.flatten() {
            coordinates.push(format!(
                "{package_id}@{}",
                version.file_name().to_string_lossy()
            ));
        }
    }
    coordinates.sort();
    coordinates
}

/// Multipart body with text fields plus one file field, for platform import.
fn multipart_import_body(owner: &str, project: &str, archive: &[u8]) -> (String, Vec<u8>) {
    let boundary = format!(
        "zebflow-boundary-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut body = Vec::new();
    for (name, value) in [("owner", owner), ("project", project)] {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"archive\"; filename=\"project.full.tar\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/x-tar\r\n\r\n");
    body.extend_from_slice(archive);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    (boundary, body)
}

fn multipart_body(field_name: &str, file_name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    multipart_body_with_type(field_name, file_name, "application/x-tar", bytes)
}

fn multipart_body_with_type(
    field_name: &str,
    file_name: &str,
    content_type: &str,
    bytes: &[u8],
) -> (String, Vec<u8>) {
    let boundary = format!(
        "zebflow-boundary-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{file_name}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    (boundary, body)
}

#[tokio::test]
async fn platform_bootstrap_and_login_flow_works() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("login-flow");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=superadmin&password=test-pass"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(login.status(), axum::http::StatusCode::SEE_OTHER);

    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string();

    let home = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/home")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(home.status(), axum::http::StatusCode::OK);
    let body = to_bytes(home.into_body(), usize::MAX)
        .await
        .expect("home body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Projects for"));
    assert!(html.contains("superadmin"));
    assert!(html.contains("default"));

    let project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default")
                .method("GET")
                .header(header::COOKIE, cookie.clone())
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(project.status(), axum::http::StatusCode::OK);
    let body = to_bytes(project.into_body(), usize::MAX)
        .await
        .expect("project body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Dashboard"));
    assert!(html.contains("Pipelines"));
    assert!(html.contains("Credentials"));
    assert!(html.contains("Files"));
    assert!(html.contains("Settings"));
    // `data/` splits into the four tiers `project-directory.md` §3 draws:
    // store (durable), cache (disposable, rebuilds from repo/), recovery and
    // logs (disposable, bounded). A fresh project gets all four up front.
    let project_root = data_root.join("users").join("superadmin").join("default");
    assert!(project_root.join("data").exists());
    assert!(project_root.join("data").join("store").exists());
    assert!(project_root.join("data").join("cache").exists());
    assert!(
        project_root
            .join("data")
            .join("cache")
            .join("pipelines")
            .exists()
    );
    assert!(
        project_root
            .join("data")
            .join("cache")
            .join("agent_docs")
            .exists()
    );
    assert!(project_root.join("data").join("recovery").exists());
    assert!(project_root.join("data").join("logs").exists());
    assert!(project_root.join("files").exists());
    // files/public and files/private are not scaffolded: ZebFS has no physical
    // public/private split, only ZebFsAclManifest per-path visibility
    // (project-directory.md section 4).
    assert!(project_root.join("repo").exists());
    assert!(project_root.join("repo").join(".git").exists());
    // No folder is scaffolded any more: the layout entries name where a kind is
    // looked for, not directories the platform makes.
    assert!(!project_root.join("repo").join("pipelines").exists());

    let settings = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/settings")
                .method("GET")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(settings.status(), axum::http::StatusCode::OK);
    let body = to_bytes(settings.into_body(), usize::MAX)
        .await
        .expect("settings body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Settings"));
    assert!(html.contains("Project Export"));
    assert!(html.contains("Portability"));
}

#[tokio::test]
async fn public_hub_requires_service_and_hides_project_internals() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("public-hub-boundary");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;

    let disabled = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("disabled response");
    assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "default", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "default",
            "calc-studio",
            "Calc Studio",
            "https://publishers.example/calc-studio",
            "private@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            true,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher");
    platform
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "default",
            "pipelines/internal-calc.zf.json",
            "Internal Calculator",
            "Internal project path should not leak",
            "manual",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"internal-calc"},
  "spec":{
  "id":"internal-calc",
  "entry_nodes":[],
  "nodes":[],
  "edges":[]}
}"#,
        )
        .expect("pipeline");
    let files_dir = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files")
        .join("images");
    std::fs::create_dir_all(&files_dir).expect("image dir");
    let mut cover_png = Vec::new();
    image::DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(32, 18, |_x, _y| {
        image::Rgba([32, 96, 160, 255])
    }))
    .write_to(&mut Cursor::new(&mut cover_png), image::ImageFormat::Png)
    .expect("png");
    std::fs::write(files_dir.join("cover.png"), cover_png).expect("write cover");
    platform
        .hub
        .publish_asset(
            "superadmin",
            "default",
            "superadmin",
            "calc-studio",
            "",
            "",
            "",
            "superadmin",
            "default",
            "pipeline_with_dependencies",
            "pipelines/internal-calc.zf.json",
            "calc-tools",
            "1.0.0",
            "Calculator Tools",
            "Reusable calculator pipeline.",
            "",
            "public",
            Default::default(),
            vec!["math".to_string()],
        )
        .expect("publish");

    platform
        .zebflow_cfg
        .set_hub_distribution(
            "superadmin",
            "default",
            ZebflowJsonDistributionHub {
                producer_enabled: true,
                ..Default::default()
            },
        )
        .expect("enable producer mode");
    let (_token, publisher_token) = platform
        .hub
        .create_token(
            "superadmin",
            "default",
            &CreateHubTokenRequest {
                publisher_id: "calc-studio".to_string(),
                title: "Publish token".to_string(),
                scopes: vec!["hub:publish".to_string()],
                expires_at: None,
            },
        )
        .expect("publisher token");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let no_token_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-no-token",
                        "version": "1.0.0",
                        "publisher_id": "calc-studio",
                        "title": "No Token",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("no token publish response");
    assert_eq!(no_token_publish.status(), StatusCode::UNAUTHORIZED);
    let token_publish_review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/assets/publish-review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-token",
                        "version": "1.0.0",
                        "publisher_token": publisher_token,
                        "title": "Token Publish",
                        "image_file_path": "images/cover.png",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("token publish review response");
    let token_publish_review_status = token_publish_review.status();
    let token_publish_review_body = response_text(token_publish_review).await;
    assert_eq!(
        token_publish_review_status,
        StatusCode::OK,
        "{token_publish_review_body}"
    );
    let token_publish_review: Value =
        serde_json::from_str(&token_publish_review_body).expect("review json");
    assert_eq!(
        token_publish_review["review"]["package_id"],
        json!("calc-studio.calc-tools-token")
    );
    assert_eq!(
        token_publish_review["review"]["media"][0]["name"],
        json!("cover.webp")
    );
    assert_eq!(
        token_publish_review["review"]["media"][0]["content_type"],
        json!("image/webp")
    );
    let token_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-token",
                        "version": "1.0.0",
                        "publisher_id": "spoofed-publisher",
                        "publisher_token": publisher_token,
                        "title": "Token Publish",
                        "image_file_path": "images/cover.png",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("token publish response");
    assert_eq!(token_publish.status(), StatusCode::OK);
    let token_publish = response_json(token_publish).await;
    assert_eq!(
        token_publish["package"]["publisher_id"],
        json!("calc-studio")
    );
    assert_eq!(
        token_publish["package"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    // Presentation is the mutable half: a typo is corrected by writing the
    // package row, and the cover it does not mention stays where it was.
    let presentation_patch = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/assets/calc-tools-token/presentation")
                .method("PATCH")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "publisher_token": publisher_token,
                        "summary": "Calculators, batteries included.",
                        "description_md": "## Calc Tools\nA long description."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("presentation patch response");
    assert_eq!(presentation_patch.status(), StatusCode::OK);
    let presentation_patch = response_json(presentation_patch).await;
    assert_eq!(
        presentation_patch["presentation"]["summary"],
        json!("Calculators, batteries included.")
    );
    assert_eq!(
        presentation_patch["presentation"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    // A scope the system cannot use is refused where it is given, rather than
    // stored as nothing and failing later at a different endpoint.
    let bad_scope_token = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/tokens")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "publisher_id": "calc-studio",
                        "title": "Bad scope token",
                        "scopes": ["read", "publish"],
                        "expires_at": null
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("bad scope token response");
    assert_eq!(bad_scope_token.status(), StatusCode::BAD_REQUEST);
    let bad_scope_token = response_json(bad_scope_token).await;
    assert_eq!(
        bad_scope_token["error"]["code"],
        json!("HUB_TOKEN_SCOPE_INVALID")
    );
    let public_remote_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "calc-tools-remote",
                        "version": "1.0.0",
                        "title": "Remote Calculator Tools",
                        "description": "Published through the platform Hub API.",
                        // Presentation rides beside the release, never inside it.
                        "summary": "Quick calculator pack.",
                        "description_md": "## Remote Calculator Tools\nReusable calculator workflow.",
                        "media": [{
                            "name": "cover.png",
                            "role": "cover",
                            "content_type": "image/png",
                            "size_bytes": 3,
                            "sha256": "b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72",
                            "encoding": "base64",
                            "content": "aW1n"
                        }],
                        "gallery": {
                            "cover": {
                                "kind": "image",
                                "media_name": "cover.png",
                                "alt": "Calculator cover"
                            },
                            "items": [
                                {
                                    "kind": "image",
                                    "media_name": "cover.png",
                                    "alt": "Calculator screenshot"
                                },
                                {
                                    "kind": "youtube",
                                    "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                                    "title": "Demo video"
                                }
                            ]
                        },
                        "visibility": "public",
                        "tags": ["math", "remote"],
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/remote-calc.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.calc-tools-remote",
                                "version": "1.0.0"
                            },
                            "spec": {
                                "asset_kind": "pipeline_bundle",
                                "title": "Remote Calculator Tools",
                                "description": "Published through the platform Hub API.",
                                "files": [{
                                    "rel_path": "pipelines/remote-calc.zf.json",
                                    "kind": "pipeline",
                                    "size_bytes": 2,
                                    "reason": "primary",
                                    "encoding": "text",
                                    "content": "{}"
                                }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("public remote publish response");
    assert_eq!(public_remote_publish.status(), StatusCode::OK);
    let public_remote_publish = response_json(public_remote_publish).await;
    assert_eq!(
        public_remote_publish["package"]["package_id"],
        json!("calc-studio.calc-tools-remote")
    );
    assert_eq!(
        public_remote_publish["package"]["publisher_id"],
        json!("calc-studio")
    );
    assert_eq!(
        public_remote_publish["package"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-remote/media/cover.png")
    );
    let invalid_kind_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "bad-kind-pack",
                        "version": "1.0.0",
                        "title": "Bad Kind",
                        "visibility": "public",
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/bad.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.bad-kind-pack",
                                "version": "1.0.0"
                            },
                            "spec": {
                            "asset_kind": "made_up_kind",
                            "title": "Bad Kind",
                            "description": "Invalid hub asset kind.",
                            "files": [{
                                "rel_path": "pipelines/bad.zf.json",
                                "kind": "pipeline",
                                "size_bytes": 2,
                                "reason": "primary",
                                "encoding": "text",
                                "content": "{}"
                            }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("invalid kind response");
    assert_eq!(invalid_kind_publish.status(), StatusCode::BAD_REQUEST);
    let invalid_kind_publish = response_json(invalid_kind_publish).await;
    assert_eq!(
        invalid_kind_publish["error"]["code"],
        json!("HUB_ASSET_KIND_INVALID")
    );
    let invalid_gallery_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "bad-gallery-pack",
                        "version": "1.0.0",
                        "title": "Bad Gallery",
                        "visibility": "public",
                        "gallery": {
                            "items": [{
                                "kind": "youtube",
                                "url": "https://example.com/video",
                                "title": "Not YouTube"
                            }]
                        },
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/bad-gallery.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.bad-gallery-pack",
                                "version": "1.0.0"
                            },
                            "spec": {
                            "asset_kind": "pipeline_bundle",
                            "title": "Bad Gallery",
                            "description": "Invalid gallery.",
                            "files": [{
                                "rel_path": "pipelines/bad-gallery.zf.json",
                                "kind": "pipeline",
                                "size_bytes": 2,
                                "reason": "primary",
                                "encoding": "text",
                                "content": "{}"
                            }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("invalid gallery response");
    assert_eq!(invalid_gallery_publish.status(), StatusCode::BAD_REQUEST);
    let invalid_gallery_publish = response_json(invalid_gallery_publish).await;
    assert_eq!(
        invalid_gallery_publish["error"]["code"],
        json!("HUB_GALLERY_INVALID")
    );

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let list_body = response_text(listed).await;
    assert!(list_body.contains("calc-studio.calc-tools"));
    assert!(list_body.contains("Quick calculator pack."));
    assert!(list_body.contains("youtube"));
    assert!(
        list_body.contains("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    assert!(
        list_body.contains("/api/hub/remote/assets/calc-studio.calc-tools-remote/media/cover.png")
    );
    assert!(!list_body.contains("publisher_owner"));
    assert!(!list_body.contains("authority_owner"));
    assert!(!list_body.contains("source_owner"));
    assert!(!list_body.contains("superadmin"));
    assert!(!list_body.contains("private@example.com"));

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools/1.0.0")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = response_text(detail).await;
    assert!(detail_body.contains("calc-studio.calc-tools"));
    assert!(!detail_body.contains("publisher_owner"));
    assert!(!detail_body.contains("authority_owner"));
    assert!(!detail_body.contains("source_owner"));
    assert!(!detail_body.contains("source_project"));
    assert!(!detail_body.contains("source_ref"));
    assert!(!detail_body.contains("\"files\""));
    assert!(!detail_body.contains("superadmin"));
    assert!(!detail_body.contains("private@example.com"));
    assert!(!detail_body.contains("pipelines/internal-calc.zf.json"));

    let remote_detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-remote/1.0.0")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("remote detail response");
    assert_eq!(remote_detail.status(), StatusCode::OK);
    let remote_detail = response_json(remote_detail).await;
    assert_eq!(
        remote_detail["artifact"]["apiVersion"],
        json!("zebflow.com/v1")
    );
    assert_eq!(remote_detail["artifact"]["kind"], json!("HubPackage"));
    // The release carries only what installing it needs. Everything a human
    // reads while choosing comes from the mutable row beside it.
    for field in [
        "description_md",
        "summary",
        "image_url",
        "gallery",
        "media",
        "publisher_id",
        "publisher_display_name",
        "publisher_url",
        "publisher_email",
        "source_type",
        "source_owner",
        "source_project",
        "source_ref",
    ] {
        assert!(
            remote_detail["artifact"]["spec"].get(field).is_none(),
            "spec.{field} must not be carried by the release"
        );
    }
    assert_eq!(
        remote_detail["presentation"]["description_md"],
        json!("## Remote Calculator Tools\nReusable calculator workflow.")
    );
    assert_eq!(
        remote_detail["presentation"]["summary"],
        json!("Quick calculator pack.")
    );
    assert_eq!(
        remote_detail["presentation"]["gallery"]["items"][1]["kind"],
        json!("youtube")
    );
    assert_eq!(
        remote_detail["presentation"]["media"][0]["name"],
        json!("cover.png")
    );
    // A cover is served from the artifact store, so its bytes never appear in
    // any document a client parses to find files.
    assert!(
        remote_detail["presentation"]["media"][0]
            .get("content")
            .is_none()
    );
    assert!(
        remote_detail["presentation"]["media"][0]
            .get("artifact_sha256")
            .is_none()
    );

    let media = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("media response");
    assert_eq!(media.status(), StatusCode::OK);
    assert_eq!(
        media
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/webp")
    );

    let artifact = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools/1.0.0/artifact")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("artifact response");
    assert_eq!(artifact.status(), StatusCode::OK);
    let artifact_body = response_text(artifact).await;
    assert!(artifact_body.contains("\"artifact_sha256\""));
    assert!(artifact_body.contains("\"artifact_size_bytes\""));
    assert!(artifact_body.contains("\"files\""));
    assert!(artifact_body.contains("pipelines/internal-calc.zf.json"));
    // Token publishes land in the Public Hub store; the blessed shelf keeps
    // its own catalog beside it and is never written by a publish.
    assert!(
        data_root
            .join("services")
            .join("hub-public")
            .join("hub.db")
            .is_file()
    );
    assert!(
        data_root
            .join("services")
            .join("hub-local")
            .join("hub.db")
            .is_file()
    );
    assert!(
        data_root
            .join("services")
            .join("hub-public")
            .join("packages")
            .join("calc-studio.calc-tools")
            .join("versions")
            .join("1.0.0")
            .join("artifact.json")
            .is_file()
    );
    assert!(
        !data_root
            .join("services")
            .join("hub-local")
            .join("packages")
            .join("calc-studio.calc-tools")
            .exists(),
        "a publisher token cannot land a release in the blessed shelf"
    );
    let delete_remote = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-remote")
                .method("DELETE")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete response");
    assert_eq!(delete_remote.status(), StatusCode::OK);
    let delete_remote = response_json(delete_remote).await;
    assert_eq!(delete_remote["retracted_versions"], json!(1));
    // DELETE retracts: the coordinate stays listed and marked, so a project
    // that pinned it reads what happened instead of finding nothing there.
    let listed_after_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list after retract response");
    assert_eq!(listed_after_delete.status(), StatusCode::OK);
    let listed_after_delete = response_json(listed_after_delete).await;
    let retracted_item = listed_after_delete["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["package_id"] == json!("calc-studio.calc-tools-remote"))
        .expect("a retracted package stays listed");
    assert!(retracted_item["retracted"]["retracted_at"].is_i64());
    assert_eq!(retracted_item["latest_version"], json!(""));
    assert!(
        !data_root
            .join("services")
            .join("hub-public")
            .join("packages")
            .join("calc-studio.calc-tools-remote")
            .join("versions")
            .join("1.0.0")
            .join("artifact.json")
            .exists()
    );
    assert!(
        !data_root
            .join("platform")
            .join("hub")
            .join("assets")
            .exists()
    );
}

#[tokio::test]
async fn hub_scoped_tokens_split_prosumer_and_consumer_projects() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("hub-prosumer-consumer");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "prosumer-app".to_string(),
                title: Some("Prosumer App".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("prosumer project");
    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "consumer-app".to_string(),
                title: Some("Consumer App".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("consumer project");
    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "prosumer-app", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "prosumer-app",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            false,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher");
    platform
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "prosumer-app",
            "pipelines/prosumer-demo.zf.json",
            "Prosumer Demo",
            "Published by a scoped publisher token",
            "manual",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"prosumer-demo"},
  "spec":{
  "id":"prosumer-demo",
  "entry_nodes":[],
  "nodes":[],
  "edges":[]}
}"#,
        )
        .expect("prosumer pipeline");
    let (_prosumer_row, prosumer_token) = platform
        .hub
        .create_token(
            "superadmin",
            "prosumer-app",
            &CreateHubTokenRequest {
                publisher_id: "zebflow-labs".to_string(),
                title: "Prosumer publish".to_string(),
                scopes: vec!["hub:read".to_string(), "hub:publish".to_string()],
                expires_at: None,
            },
        )
        .expect("prosumer token");
    let (_consumer_row, consumer_token) = platform
        .hub
        .create_token(
            "superadmin",
            "consumer-app",
            &CreateHubTokenRequest {
                publisher_id: "zebflow-labs".to_string(),
                title: "Consumer read".to_string(),
                scopes: vec!["hub:read".to_string()],
                expires_at: None,
            },
        )
        .expect("consumer token");

    let prosumer_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/prosumer-app/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/prosumer-demo.zf.json",
                        "package_id": "prosumer-demo-pack",
                        "version": "1.0.0",
                        "publisher_token": prosumer_token,
                        "title": "Prosumer Demo Pack",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("prosumer publish response");
    assert_eq!(prosumer_publish.status(), StatusCode::OK);
    let prosumer_publish = response_json(prosumer_publish).await;
    assert_eq!(
        prosumer_publish["package"]["publisher_id"],
        json!("zebflow-labs")
    );

    let consumer_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/consumer-app/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/prosumer-demo.zf.json",
                        "package_id": "consumer-should-not-publish",
                        "version": "1.0.0",
                        "publisher_token": consumer_token,
                        "title": "Consumer Cannot Publish",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("consumer publish response");
    assert_eq!(consumer_publish.status(), StatusCode::UNAUTHORIZED);

    let consumer_install = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-app/hub/assets/zebflow-labs.prosumer-demo-pack/1.0.0/add",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"target_folder": "/hub/zebflow-labs.prosumer-demo-pack"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("consumer install response");
    assert_eq!(consumer_install.status(), StatusCode::OK);
    let consumer_install = response_json(consumer_install).await;
    assert_eq!(consumer_install["ok"], json!(true));

    let installed = platform
        .projects
        .read_pipeline_source(
            "superadmin",
            "consumer-app",
            "hub/zebflow-labs.prosumer-demo-pack/pipelines/prosumer-demo.zf.json",
        )
        .expect("installed pipeline");
    assert!(installed.contains("prosumer-demo"));
}

#[tokio::test]
async fn project_bundle_installs_spatial_blog_with_sekejap_schema_across_two_instances() {
    unsafe {
        std::env::set_var("ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE", "1");
    }

    let mut publisher_config = PlatformConfig::default();
    publisher_config.data_root = temp_test_dir("hub-project-bundle-publisher");
    publisher_config.default_password = "test-pass".to_string();
    let publisher = Arc::new(PlatformService::from_config(publisher_config).expect("publisher"));
    let publisher_app = zebflow::platform::web::router(publisher.clone()).await;
    let publisher_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("publisher listener");
    let publisher_addr = publisher_listener.local_addr().expect("publisher addr");
    let publisher_server = tokio::spawn(async move {
        axum::serve(publisher_listener, publisher_app)
            .await
            .expect("publisher server");
    });

    publisher
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "spatial-blogging-source".to_string(),
                title: Some("Spatial Blogging".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("source project");
    sekejap::create_table(
        &publisher.config.data_root,
        "superadmin",
        "spatial-blogging-source",
        &CreateSimpleTableRequest {
            table: "posts".to_string(),
            attributes: vec![
                CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: vec!["hash".to_string(), "fulltext".to_string()],
                },
                CollectionAttribute {
                    name: "body".to_string(),
                    kind: "text".to_string(),
                    index_types: vec!["fulltext".to_string()],
                },
                CollectionAttribute {
                    name: "geometry".to_string(),
                    kind: "geo".to_string(),
                    index_types: vec!["spatial".to_string()],
                },
                CollectionAttribute {
                    name: "embedding".to_string(),
                    // A vector attribute carries its dimension: sekejap 0.17
                    // types the column as VECTOR(n).
                    kind: "vector(3)".to_string(),
                    index_types: vec!["vector".to_string()],
                },
            ],
            hash_indexed_fields: Vec::new(),
            range_indexed_fields: Vec::new(),
        },
    )
    .expect("source sekejap schema");
    let source_layout = publisher
        .projects
        .project_layout("superadmin", "spatial-blogging-source")
        .expect("source layout");
    let seed_path = source_layout
        .repo_dir
        .join("initial-data")
        .join("sekejap")
        .join("posts.sql");
    std::fs::create_dir_all(seed_path.parent().unwrap()).expect("seed dir");
    std::fs::write(
        &seed_path,
        "INSERT INTO posts (_key, title, body) VALUES ('hello-melbourne', 'Hello Melbourne', 'Seeded spatial starter post');\n",
    )
    .expect("seed file");
    publisher
        .projects
        .write_repo_file(
            "superadmin",
            "spatial-blogging-source",
            "pages/spatial-blog.tsx",
            r#"
export default function SpatialBlogPage({ input }) {
  const count = Array.isArray(input?.rows) ? input.rows.length : 0;
  return (
    <main className="min-h-screen bg-slate-950 px-8 py-10 text-white">
      <section className="mx-auto max-w-4xl">
        <p className="text-sm uppercase tracking-wide text-emerald-300">Spatial blogging</p>
        <h1 className="mt-3 text-4xl font-semibold">Spatial Blogging Starter</h1>
        <p className="mt-4 text-slate-300">A project bundle with a live Sekejap schema for text, geometry, and embedding fields.</p>
        <div className="mt-8 grid gap-3 sm:grid-cols-3">
          <div className="border border-emerald-500/40 bg-emerald-500/10 p-4">
            <p className="text-xs text-emerald-200">Rows</p>
            <p className="mt-2 text-2xl font-semibold">{count}</p>
          </div>
          <div className="border border-sky-500/40 bg-sky-500/10 p-4">Geo field ready</div>
          <div className="border border-fuchsia-500/40 bg-fuchsia-500/10 p-4">Vector field ready</div>
        </div>
      </section>
    </main>
  );
}
"#
                .trim(),
        )
        .expect("template");
    publisher
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "spatial-blogging-source",
            "pipelines/pages/spatial-blog.zf.json",
            "Spatial Blog",
            "Render spatial blog posts from Sekejap.",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"spatial-blog"},
  "spec":{
  "id":"spatial-blog",
  "entry_nodes":["trigger"],
  "nodes":[
    {"id":"trigger","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/blog","method":"GET"}},
    {"id":"query","kind":"n.sekejap.query","input_pins":["in"],"output_pins":["out"],"config":{"query":"SELECT * FROM posts LIMIT 20","limit":20,"read_only":true}},
    {"id":"response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/spatial-blog.tsx"}}
  ],
  "edges":[
    {"from_node":"trigger","from_pin":"out","to_node":"query","to_pin":"in"},
    {"from_node":"query","from_pin":"out","to_node":"response","to_pin":"in"}
  ]}
}"#,
        )
        .expect("pipeline");
    publisher
        .projects
        .activate_pipeline_definition(
            "superadmin",
            "spatial-blogging-source",
            "pipelines/pages/spatial-blog.zf.json",
        )
        .expect("activate source pipeline");
    publisher
        .hub
        .ensure_default_service_instance(
            "standalone",
            &format!("http://{publisher_addr}/api"),
            true,
        )
        .expect("enable publisher hub");
    publisher
        .hub
        .upsert_publisher(
            "superadmin",
            "spatial-blogging-source",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            false,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher identity");
    publisher
        .hub
        .publish_asset(
            "superadmin",
            "spatial-blogging-source",
            "superadmin",
            "zebflow-labs",
            "",
            "",
            "",
            "superadmin",
            "spatial-blogging-source",
            "project_files",
            ".",
            "spatial-blogging",
            "1.0.0",
            "Spatial Blogging Starter",
            "Cloneable project bundle with Sekejap spatial and vector schema.",
            "",
            "public",
            zebflow::platform::services::hub::HubProjectBundlePublishOptions {
                include_sekejap_schema: true,
                include_sqlite_schema: false,
                include_libraries: Vec::new(),
                include_initial_data: true,
                initial_data_paths: vec!["initial-data/sekejap/posts.sql".to_string()],
            },
            vec!["spatial".to_string(), "blog".to_string()],
        )
        .expect("publish project bundle");

    let mut consumer_config = PlatformConfig::default();
    consumer_config.data_root = temp_test_dir("hub-project-bundle-consumer");
    consumer_config.default_password = "test-pass".to_string();
    let consumer = Arc::new(PlatformService::from_config(consumer_config).expect("consumer"));
    let consumer_app = zebflow::platform::web::router(consumer.clone()).await;
    let cookie = login_cookie(consumer_app.clone(), "superadmin", "test-pass").await;
    consumer
        .hub
        .upsert_platform_repository(
            "superadmin",
            "local-publisher",
            "Local Publisher",
            &format!("http://{publisher_addr}/api"),
            "",
            "",
            "",
            "api",
            None,
            "public",
            true,
        )
        .expect("consumer hub source");

    // The review comes first and must describe this install without performing
    // any part of it.
    let review = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/install/review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("review request"),
        )
        .await
        .expect("review response");
    let review_status = review.status();
    let review = response_json(review).await;
    assert_eq!(review_status, StatusCode::OK, "{review}");
    let review = review["review"].clone();
    println!(
        "install review: {}",
        serde_json::to_string_pretty(&review).expect("review json")
    );
    assert_eq!(review["installable"], json!(true), "{review}");
    assert!(
        !review["database_initialization"]
            .as_array()
            .expect("database_initialization array")
            .is_empty(),
        "the review says what the seed SQL does before it runs"
    );
    let reviewed_project = review["project"]
        .as_str()
        .expect("reviewed project slug")
        .to_string();
    assert!(
        consumer
            .projects
            .get_project("superadmin", &reviewed_project)
            .expect("project lookup")
            .is_none(),
        "the review created no project"
    );
    assert!(
        !consumer
            .config
            .data_root
            .join(format!("users/superadmin/{reviewed_project}"))
            .exists(),
        "the review wrote no files and no store"
    );

    // The account-scoped route is the same review under a different auth rule,
    // and a review can be repeated because it changes nothing.
    let account_review = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/hub/install/review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("account review request"),
        )
        .await
        .expect("account review response");
    assert_eq!(account_review.status(), StatusCode::OK);
    assert_eq!(response_json(account_review).await["review"], review);

    let install = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/install")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("install request"),
        )
        .await
        .expect("install response");
    let install_status = install.status();
    let install = response_json(install).await;
    assert_eq!(install_status, StatusCode::OK, "{install}");
    let installed_project = install["project"]["project"]
        .as_str()
        .expect("installed project slug")
        .to_string();

    // What the review promised is what the install reports having done.
    assert_eq!(installed_project, reviewed_project);
    for field in [
        "files_written",
        "skipped_files",
        "pipelines_registered",
        "pipelines_activated",
        "pipelines_not_activated",
        "unexecuted_initial_data",
        "database_initialization",
        "schema_executed",
    ] {
        assert_eq!(
            install["install"][field], review[field],
            "the install's {field} is what the review showed"
        );
    }

    let cloned_tables =
        sekejap::list_tables(&consumer.config.data_root, "superadmin", &installed_project)
            .expect("cloned tables");
    assert_eq!(cloned_tables.len(), 1);
    assert_eq!(cloned_tables[0].table, "posts");
    assert!(
        cloned_tables[0]
            .spatial_fields
            .contains(&"geometry".to_string())
    );
    assert!(
        cloned_tables[0]
            .vector_fields
            .contains(&"embedding".to_string())
    );
    let seeded = sekejap::execute_sql(
        &consumer.config.data_root,
        "superadmin",
        &installed_project,
        "SELECT * FROM posts",
        &[],
        20,
        true,
    )
    .expect("seeded rows");
    assert_eq!(seeded.rows.len(), 1);

    let rendered = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/wh/superadmin/{installed_project}/blog"))
                .method("GET")
                .body(Body::empty())
                .expect("render request"),
        )
        .await
        .expect("render response");
    assert_eq!(rendered.status(), StatusCode::OK);
    let html = response_text(rendered).await;
    assert!(html.contains("Spatial Blogging Starter"));
    assert!(html.contains("Vector field ready"));

    publisher_server.abort();
}

#[tokio::test]
async fn hub_add_reviews_risks_and_respects_target_folders() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("hub-add-review");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "producer-review".to_string(),
                title: Some("Producer Review".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("producer project");
    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "consumer-review".to_string(),
                title: Some("Consumer Review".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("consumer project");
    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "producer-review", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "producer-review",
            "review-lab",
            "Review Lab",
            "https://publishers.example/review-lab",
            "review@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            false,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher");
    platform
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "producer-review",
            "pipelines/safety-demo.zf.json",
            "Safety Demo",
            "Pipeline with reviewable effects",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"safety-demo"},
  "spec":{
  "id":"safety-demo",
  "entry_nodes":["wh"],
  "nodes":[
    {"id":"wh","kind":"n.trigger.webhook","config":{"path":"/unsafe-public-hook"}},
    {"id":"http","kind":"n.http.request","config":{"url":"https://api.example.com/v1/items","credential":"secure-egress"}},
    {"id":"pg","kind":"n.pg.query","config":{"credential":"pg-main"}},
    {"id":"fs","kind":"n.fs.put","config":{"path":"exports/out.json"}}
  ],
  "edges":[]}
}"#,
        )
        .expect("producer pipeline");
    platform
        .hub
        .publish_asset(
            "superadmin",
            "producer-review",
            "superadmin",
            "review-lab",
            "",
            "",
            "",
            "superadmin",
            "producer-review",
            "pipeline_with_dependencies",
            "pipelines/safety-demo.zf.json",
            "safety-demo",
            "1.0.0",
            "Safety Demo",
            "Reviewable risky pipeline.",
            "",
            "public",
            Default::default(),
            vec!["review".to_string()],
        )
        .expect("publish");

    let review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"target_folder": "/safety-demo"}).to_string()))
                .expect("request"),
        )
        .await
        .expect("review response");
    assert_eq!(review.status(), StatusCode::OK);
    let review = response_json(review).await;
    assert_eq!(review["review"]["risk_level"], json!("high"));
    assert_eq!(
        review["review"]["files_added"][0],
        json!("safety-demo/pipelines/safety-demo.zf.json")
    );
    assert!(
        review["review"]["nodes_used"]
            .as_array()
            .expect("nodes")
            .contains(&json!("n.http.request"))
    );
    assert!(
        review["review"]["credentials_required"]
            .as_array()
            .expect("credentials")
            .contains(&json!("secure-egress"))
    );
    assert!(
        review["review"]["external_urls"]
            .as_array()
            .expect("urls")
            .contains(&json!("https://api.example.com/v1/items"))
    );
    assert!(
        review["review"]["public_endpoints"]
            .as_array()
            .expect("endpoints")
            .contains(&json!("/unsafe-public-hook"))
    );
    assert!(
        review["review"]["database_effects"]
            .as_array()
            .expect("database effects")
            .contains(&json!("n.pg.query"))
    );
    assert!(
        review["review"]["filesystem_effects"]
            .as_array()
            .expect("filesystem effects")
            .contains(&json!("n.fs.put"))
    );

    let nested_review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"target_folder": "/functions/blogging/safety-demo"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("nested review response");
    assert_eq!(nested_review.status(), StatusCode::OK);
    let nested_review = response_json(nested_review).await;
    assert_eq!(
        nested_review["review"]["files_added"][0],
        json!("functions/blogging/safety-demo/pipelines/safety-demo.zf.json")
    );

    let add = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/add",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"target_folder": "/safety-demo"}).to_string()))
                .expect("request"),
        )
        .await
        .expect("add response");
    assert_eq!(add.status(), StatusCode::OK);
    let added = platform
        .projects
        .read_pipeline_source(
            "superadmin",
            "consumer-review",
            "safety-demo/pipelines/safety-demo.zf.json",
        )
        .expect("added pipeline");
    assert!(added.contains("unsafe-public-hook"));

    // A target folder used to be pulled into the source root only when the
    // caller typed a leading slash, so `billing` installed outside it, was
    // reviewed as risk-free with every finding list empty, and registered
    // nothing. All three spellings now review and register the same pipeline.
    // One webhook path may only be claimed once, so each spelling is installed
    // on its own and removed again.
    let mut previous_install = Some("safety-demo/pipelines/safety-demo.zf.json".to_string());
    for (target_folder, expected_root) in [
        // The source root is the repository, so an install root is the target
        // folder itself with no prefix in front of it.
        ("", "hub/review-lab.safety-demo"),
        ("billing", "billing"),
        ("/billing", "billing"),
    ] {
        if let Some(previous) = previous_install.take() {
            platform
                .projects
                .delete_pipeline("superadmin", "consumer-review", &previous)
                .expect("remove previous install");
        }
        let review = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(
                        "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                    )
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "target_folder": target_folder }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("target folder review response");
        assert_eq!(review.status(), StatusCode::OK);
        let review = response_json(review).await;
        assert_eq!(
            review["review"]["install_root"],
            json!(expected_root),
            "install root for target_folder {target_folder:?}"
        );
        assert_eq!(
            review["review"]["risk_level"],
            json!("high"),
            "risk level for target_folder {target_folder:?}"
        );
        assert!(
            review["review"]["public_endpoints"]
                .as_array()
                .expect("endpoints")
                .contains(&json!("/unsafe-public-hook")),
            "public endpoints for target_folder {target_folder:?}"
        );

        let add = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(
                        "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/add",
                    )
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "target_folder": target_folder }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("target folder add response");
        assert_eq!(add.status(), StatusCode::OK);
        let add = response_json(add).await;
        let registered = add["result"]["pipelines_registered"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            registered,
            review["review"]["pipelines_registered"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            "review and install disagree for target_folder {target_folder:?}"
        );
        assert_eq!(
            registered.len(),
            1,
            "nothing registered for target_folder {target_folder:?}: {add}"
        );
        let identity = registered[0].as_str().expect("identity");
        assert!(
            platform
                .projects
                .get_pipeline_meta_by_file_id("superadmin", "consumer-review", identity)
                .expect("meta lookup")
                .is_some(),
            "no catalog row for target_folder {target_folder:?}"
        );
        previous_install = Some(identity.to_string());
    }
}

#[tokio::test]
async fn project_hub_ui_is_project_surface_with_explicit_grant() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("project-hub-surface");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let create_project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/projects")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "project": "market-test",
                        "title": "Market Test"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create project response");
    assert_eq!(create_project.status(), StatusCode::OK);

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/market-test/hub")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("hub page response");
    let page_status = page.status();
    let page_body = response_text(page).await;
    assert_eq!(page_status, StatusCode::OK, "hub page: {page_body}");
    assert!(page_body.contains("Browse"));
    assert!(page_body.contains("Published"));
    assert!(page_body.contains("Publish"));
    assert!(!page_body.contains("Create Hub Token"));
    assert!(!page_body.contains("Publisher Registry"));
    assert!(!page_body.contains("Pack Repositories"));
    assert!(!page_body.contains("Project Publishing Access"));

    let repos = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/market-test/hub/repositories")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("repository response");
    assert_eq!(repos.status(), StatusCode::OK);
    let repos = response_json(repos).await;
    let items = repos["items"].as_array().expect("repository items");
    assert!(
        items.is_empty(),
        "project should not inherit Hub access without an explicit grant"
    );

    let source = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/repositories")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "zebflow-com",
                        "title": "Zebflow Hub",
                        "base_url": "https://93.184.216.34/api",
                        "remote_owner": "",
                        "remote_project": "",
                        "visibility": "public",
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("platform source response");
    let source_status = source.status();
    let source_body = response_text(source).await;
    assert_eq!(
        source_status,
        StatusCode::OK,
        "platform source: {source_body}"
    );

    let grant = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/grants")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "zebflow-com",
                        "grant_scope": "all_projects",
                        "can_read": true,
                        "can_publish": false,
                        "can_manage": false,
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("grant response");
    let grant_status = grant.status();
    let grant_body = response_text(grant).await;
    assert_eq!(grant_status, StatusCode::OK, "grant: {grant_body}");
    let grant: Value = serde_json::from_str(&grant_body).expect("grant json");
    assert_eq!(grant["grant"]["grant_scope"], json!("all_projects"));

    let repos = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/market-test/hub/access")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("effective access response");
    assert_eq!(repos.status(), StatusCode::OK);
    let repos = response_json(repos).await;
    let items = repos["repositories"].as_array().expect("repository items");
    assert!(items.iter().any(|item| {
        item["repository_id"] == json!("zebflow-com")
            && item["base_url"] == json!("https://93.184.216.34/api")
            && item["enabled"] == json!(true)
    }));
}

#[tokio::test]
async fn private_project_files_require_project_capability() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("private-file-authz");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let project_files = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files");
    fs::create_dir_all(project_files.join("public")).expect("public dir");
    fs::create_dir_all(project_files.join("private")).expect("private dir");
    fs::write(project_files.join("public").join("hello.txt"), "hello").expect("public file");
    fs::write(project_files.join("private").join("secret.txt"), "secret").expect("private file");

    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/public/hello.txt")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("public response");
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(response_text(public).await, "hello");

    let anonymous_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("anonymous private response");
    assert_eq!(anonymous_private.status(), StatusCode::UNAUTHORIZED);

    let forged_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("forged private response");
    assert_eq!(forged_private.status(), StatusCode::UNAUTHORIZED);

    let superadmin_cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let create_user = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/users")
                .method("POST")
                .header(header::COOKIE, &superadmin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "owner": "bob",
                        "password": "bob-pass",
                        "role": "member"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create user response");
    assert_eq!(create_user.status(), StatusCode::OK);

    let bob_cookie = login_cookie(app.clone(), "bob", "bob-pass").await;
    let other_user_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, bob_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("other user private response");
    assert_eq!(other_user_private.status(), StatusCode::FORBIDDEN);

    let owner_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, &superadmin_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("owner private response");
    assert_eq!(owner_private.status(), StatusCode::OK);
    assert_eq!(response_text(owner_private).await, "secret");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", project_files.join("private").join("escape"))
            .expect("symlink");
        let symlink_escape = app
            .oneshot(
                Request::builder()
                    .uri("/files/superadmin/default/private/escape")
                    .method("GET")
                    .header(header::COOKIE, superadmin_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("symlink response");
        assert_eq!(symlink_escape.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[ignore = "manual local TTS smoke using Narrator Piper assets"]
async fn platform_tts_upload_credential_and_execute_smoke() {
    let model_src = Path::new("/path/to/piper/narrator.onnx");
    let config_src = Path::new("/path/to/piper/narrator.onnx.json");
    if !(model_src.is_file() && config_src.is_file()) {
        eprintln!("local TTS smoke assets are not present; skipping");
        return;
    }

    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("tts-api-smoke");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let app = build_router(config).await.expect("platform router");

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=superadmin&password=test-pass"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string();

    let upload_specs = vec![
        (
            "private/voices/narrator".to_string(),
            "narrator.onnx".to_string(),
            fs::read(model_src).expect("read model"),
            "application/octet-stream".to_string(),
        ),
        (
            "private/voices/narrator".to_string(),
            "narrator.onnx.json".to_string(),
            fs::read(config_src).expect("read config"),
            "application/json".to_string(),
        ),
    ];
    for (path, file_name, bytes, content_type) in upload_specs {
        let (boundary, body) = multipart_body_with_type("file", &file_name, &content_type, &bytes);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/projects/superadmin/default/files/upload?path={path}"
                    ))
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("upload response");
        let status = response.status();
        let body_text = response_text(response).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "upload failed for {path}/{file_name}: {body_text}"
        );
    }

    let cred_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/credentials")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "credential_id": "narrator-tts",
                        "title": "Narrator TTS",
                        "kind": "tts",
                        "notes": "",
                        "secret": {
                            "provider": "piper",
                            "model_file": "voices/narrator/narrator.onnx",
                            "config_file": "voices/narrator/narrator.onnx.json"
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("credential response");
    assert_eq!(cred_response.status(), StatusCode::OK);

    let dsl = r#"register pipelines/tests/tts-api
[a] trigger.manual
[b] ai.tts --provider piper --credential narrator-tts --text-expr "$input.text" --output-path-expr "'audio/' + $input.slug + '.wav'" --return both
[a] -> [b]
&& activate pipelines/tests/tts-api.zf.json"#;
    let dsl_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/dsl")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "dsl": dsl }).to_string()))
                .expect("request"),
        )
        .await
        .expect("dsl response");
    assert_eq!(dsl_response.status(), StatusCode::OK);
    let dsl_json = response_json(dsl_response).await;
    assert_eq!(dsl_json["ok"], json!(true), "dsl output: {dsl_json}");

    let exec_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/execute")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "file_rel_path": "pipelines/tests/tts-api.zf.json",
                        "trigger": "manual",
                        "input": {
                            "text": "Halo, ini Narrator dari API smoke n.ai.tts.",
                            "slug": "narrator-api-smoke"
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("execute response");
    let exec_status = exec_response.status();
    let exec_body_text = response_text(exec_response).await;
    assert_eq!(
        exec_status,
        StatusCode::OK,
        "execute failed: {exec_body_text}"
    );
    let exec_json: Value = serde_json::from_str(&exec_body_text).expect("execute json");
    assert_eq!(exec_json["ok"], json!(true), "execute output: {exec_json}");
    let output = &exec_json["output"];
    let file_path = output["audio"]["path"]
        .as_str()
        .expect("audio.path should be present");
    let file_url = output["audio"]["url"]
        .as_str()
        .expect("audio.url should be present");
    let blob = output["audio_blob_base64"]
        .as_str()
        .expect("audio_blob_base64 should be present");
    let blob_bytes = base64::engine::general_purpose::STANDARD
        .decode(blob)
        .expect("decode blob");
    assert!(
        blob_bytes.starts_with(b"RIFF"),
        "expected WAV blob to start with RIFF"
    );
    assert_eq!(file_path, "private/audio/narrator-api-smoke.wav");
    assert!(file_url.ends_with("/private/audio/narrator-api-smoke.wav"));

    let file_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(file_url)
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("file response");
    assert_eq!(file_response.status(), StatusCode::OK);
    let file_bytes = to_bytes(file_response.into_body(), usize::MAX)
        .await
        .expect("file bytes");
    assert!(file_bytes.starts_with(b"RIFF"));

    let disk_path = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files")
        .join(file_path);
    assert!(
        disk_path.is_file(),
        "expected file on disk: {}",
        disk_path.display()
    );
}

#[tokio::test]
async fn a_markdown_file_lives_anywhere_and_the_registry_renders_its_folder() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("docs-nested-registry");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("router starts");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // A folder and a `.md` inside it, at a path the layout never named. The
    // road this replaces could only reach `repo/docs/`.
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/file?path=guides/archive/intro.md")
                .method("PUT")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("# Intro\n"))
                .expect("request"),
        )
        .await
        .expect("write response");
    assert_eq!(created.status(), StatusCode::OK);

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/move")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"from_path":"guides/archive/intro.md","to_path":"guides/intro.md"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("move response");
    assert_eq!(moved.status(), StatusCode::OK);

    let tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("tree response");
    assert_eq!(tree.status(), StatusCode::OK);
    let body = response_text(tree).await;
    assert!(body.contains("guides/intro.md"), "{body}");

    // The registry renders that folder like any other, with no `/docs` overlay.
    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/guides")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("registry response");
    assert_eq!(page.status(), StatusCode::OK);
    let html = response_text(page).await;
    assert!(html.contains("intro.md"), "the folder lists its file");
}

#[tokio::test]
async fn platform_sidebar_active_classes_have_tailwind_utilities_on_section_pages() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("sidebar-tailwind");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let studio = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(studio.status(), axum::http::StatusCode::OK);
    let body = to_bytes(studio.into_body(), usize::MAX)
        .await
        .expect("studio body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("<style data-rwe-tw>"));
    assert!(html.contains("Editor"));
    assert!(html.contains("Root"));
    assert!(html.contains("Registry"));
}

#[tokio::test]
async fn platform_templates_workspace_renders_seeded_tree_and_editor_bootstrap() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("templates-workspace");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("templates body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Root"));
    assert!(html.contains("Templates"));
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));
    assert!(html.contains("Add+"));
}

#[tokio::test]
async fn platform_serves_local_codemirror_library_asset() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("templates-library-asset");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/javascript; charset=utf-8")
    );
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("library asset body");
    let js = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(js.contains("EditorView"));
    assert!(js.contains("basicSetup"));
}

#[tokio::test]
async fn a_declared_source_root_moves_templates_pipelines_and_assets() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("declared-source-root");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "declared".to_string(),
                title: Some("Declared".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("project");
    platform
        .zebflow_cfg
        .update("superadmin", "declared", |cfg| {
            cfg.configs.layout.source = Some("src".to_string());
        })
        .expect("declare layout");

    let layout = platform
        .projects
        .project_layout("superadmin", "declared")
        .expect("layout");
    assert_eq!(layout.repo_source_dir(), layout.repo_dir.join("src"));
    assert_eq!(layout.repo_static_dir(), layout.repo_dir.join("src/static"));

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/repo/file?path=src/components/panel.tsx")
                .method("PUT")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(
                    "export default function Panel() { return <div />; }\n",
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(create.status(), StatusCode::OK);
    assert!(layout.repo_dir.join("src/components/panel.tsx").is_file());
    assert!(
        !layout
            .repo_dir
            .join("pipelines/components/panel.tsx")
            .exists()
    );

    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "file_rel_path": "blog/feed",
                        "title": "Feed",
                        "description": "",
                        "trigger_kind": "webhook",
                        "source": r#"{"apiVersion":"zebflow.com/v1","kind":"Pipeline","metadata":{"name":"feed"},"spec":{"id":"feed","entry_nodes":["wh"],"nodes":[{"id":"wh","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/feed","method":"GET"}}],"edges":[]}}"#
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("register response");
    assert_eq!(register.status(), StatusCode::OK);
    let register = response_json(register).await;
    // Identity has no root segment; the root lives in zebflow.yaml.
    assert_eq!(
        register["meta"]["file_rel_path"],
        json!("blog/feed.zf.json")
    );
    assert!(layout.repo_dir.join("src/blog/feed.zf.json").is_file());

    let registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/pipelines/registry?scope=project")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("registry response");
    assert_eq!(registry.status(), StatusCode::OK);
    let registry = response_json(registry).await;
    let listed = registry["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        listed.len(),
        1,
        "registry did not discover the pipeline: {registry}"
    );
    assert_eq!(listed[0]["file_rel_path"], json!("blog/feed.zf.json"));

    std::fs::create_dir_all(layout.repo_static_dir()).expect("assets dir");
    std::fs::write(layout.repo_static_dir().join("logo.txt"), b"declared-asset")
        .expect("asset file");
    let asset = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/superadmin/declared/logo.txt")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("asset response");
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(response_text(asset).await, "declared-asset");
}

#[tokio::test]
async fn the_repository_api_creates_saves_moves_and_deletes_a_file() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("repo-file-api");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("router starts");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let write = |path: &str, body: &'static str| {
        let app = app.clone();
        let cookie = cookie.clone();
        let uri = format!("/api/projects/superadmin/default/repo/file?path={path}");
        async move {
            app.oneshot(
                Request::builder()
                    .uri(uri)
                    .method("PUT")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("write response")
        }
    };

    assert_eq!(
        write(
            "components/editor-panel.tsx",
            "export default function P() { return <div />; }\n"
        )
        .await
        .status(),
        StatusCode::OK
    );

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/move")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"from_path":"components/editor-panel.tsx","to_path":"pages/editor-panel.tsx"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("move response");
    assert_eq!(moved.status(), StatusCode::OK);

    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/file?path=pages/editor-panel.tsx")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("read response");
    assert_eq!(read.status(), StatusCode::OK);

    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/file?path=pages/editor-panel.tsx")
                .method("DELETE")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete response");
    assert_eq!(deleted.status(), StatusCode::OK);
}

/// `kinds/project-bundle/README.md`: "Import verifies function targets
/// resolve in the carried repo and reports misses through the dependency
/// report as its fifth family — report, not refuse."
#[tokio::test]
async fn import_reports_unresolved_function_targets_without_refusing() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("import-function-report");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // A pipeline that calls a function pipeline which is not in the project.
    // Registering it through the pipelines API is the only way a pipeline is
    // created; nothing writes into repo/ behind the service's back.
    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "file_rel_path": "pipelines/caller.zf.json",
                        "trigger_kind": "webhook",
                        "source": json!({
                            "apiVersion": "zebflow.com/v1",
                            "kind": "Pipeline",
                            "metadata": {"name": "caller"},
                            "spec": {
                                "id": "caller",
                                "entry_nodes": ["t"],
                                "nodes": [
                                    {"id": "t", "kind": "n.trigger.webhook",
                                     "output_pins": ["out"]},
                                    {"id": "a", "kind": "n.function.call",
                                     "input_pins": ["in"], "output_pins": ["out", "error"],
                                     "config": {"function": "absent-fn"}}
                                ],
                                "edges": [
                                    {"from_node": "t", "from_pin": "out",
                                     "to_node": "a", "to_pin": "in"}
                                ]
                            }
                        })
                        .to_string(),
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("register response");
    if register.status() != StatusCode::OK {
        let status = register.status();
        let body = response_text(register).await;
        panic!("pipeline registration failed with {status}: {body}");
    }

    let export = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("export response");
    assert_eq!(export.status(), StatusCode::OK);
    let export = response_json(export).await;
    let op = export["operation"]["operation_id"]
        .as_str()
        .expect("operation id");
    let archive = data_root
        .join("platform")
        .join("project-operations")
        .join(op)
        .join("project.bundle.tar");
    let bytes = fs::read(&archive).expect("archive bytes");

    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bytes);
    let import = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    // Report, never refuse.
    assert_eq!(import.status(), StatusCode::OK);
    let import = response_json(import).await;
    assert_eq!(import["ok"], true);

    let items = import["dependencies"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("import must carry a dependency report: {import}"));
    let misses = items
        .iter()
        .filter(|item| item["family"] == "function_pipeline")
        .map(|item| item["name"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(misses, vec!["absent-fn"], "{import}");
    assert_eq!(import["dependencies"]["ok"], false, "{import}");

    let _ = fs::remove_dir_all(&data_root);
}

#[tokio::test]
async fn project_transfer_export_import_roundtrip_restores_repo_and_files() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("transfer-roundtrip");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let project_root = data_root.join("users").join("superadmin").join("default");
    fs::create_dir_all(project_root.join("files").join("public")).expect("public dir");
    fs::write(
        project_root.join("files").join("public").join("hello.txt"),
        "hello static export\n",
    )
    .expect("seed public file");

    let export_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    if export_bundle.status() != StatusCode::OK {
        let status = export_bundle.status();
        let body = response_text(export_bundle).await;
        panic!("bundle export failed with {status}: {body}");
    }
    let export_bundle = response_json(export_bundle).await;
    let bundle_op = export_bundle["operation"]["operation_id"]
        .as_str()
        .expect("bundle operation id");

    let export_files = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/files")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("files export response");
    assert_eq!(export_files.status(), StatusCode::OK);
    let export_files = response_json(export_files).await;
    let files_op = export_files["operation"]["operation_id"]
        .as_str()
        .expect("files operation id");

    let bundle_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(bundle_op)
        .join("project.bundle.tar");
    let files_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(files_op)
        .join("project.files.tar");
    assert!(bundle_archive.exists());
    assert!(files_archive.exists());

    let zebflow_yaml = project_root.join("repo").join("zebflow.yaml");
    let mutated = fs::read_to_string(&zebflow_yaml)
        .expect("zebflow.yaml")
        .replace("title: Default", "title: Mutated Before Import");
    fs::write(&zebflow_yaml, mutated).expect("mutate zebflow.yaml");
    fs::write(
        project_root.join("files").join("public").join("hello.txt"),
        "mutated file before import\n",
    )
    .expect("mutate hello.txt");

    let bundle_bytes = fs::read(&bundle_archive).expect("bundle archive bytes");
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bundle_bytes);
    let import_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("bundle import response");
    assert_eq!(import_bundle.status(), StatusCode::OK);

    let files_bytes = fs::read(&files_archive).expect("files archive bytes");
    let (boundary, body) = multipart_body("archive", "project.files.tar", &files_bytes);
    let import_files = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/import/files")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("files import response");
    assert_eq!(import_files.status(), StatusCode::OK);

    let restored = fs::read_to_string(&zebflow_yaml).expect("restored zebflow.yaml");
    assert!(restored.contains("title: Default"));
    assert_eq!(
        fs::read_to_string(project_root.join("files").join("public").join("hello.txt"))
            .expect("restored hello.txt"),
        "hello static export\n"
    );

    let operations = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/operations")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operations response");
    assert_eq!(operations.status(), StatusCode::OK);
    let operations = response_json(operations).await;
    let items = operations["items"].as_array().expect("operation items");
    assert!(items.iter().any(|item| item["kind"] == "export_bundle"));
    assert!(items.iter().any(|item| item["kind"] == "export_files"));
    assert!(items.iter().any(|item| item["kind"] == "import_bundle"));
    assert!(items.iter().any(|item| item["kind"] == "import_files"));
}

#[tokio::test]
async fn platform_import_creates_project_and_auto_initiates_repo_only_store() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("platform-import");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // Fixture state on the default project: declared initial data in repo/
    // (the layout's `initial-data/sekejap` prefix) plus one live-only store
    // row the initial data does not know about — the discriminator between
    // "steps replayed" and "snapshot restored".
    let seed_dir = data_root.join("users/superadmin/default/repo/initial-data/sekejap");
    fs::create_dir_all(&seed_dir).expect("seed dir");
    fs::write(
        seed_dir.join("seed.sql"),
        "CREATE TABLE seeded (_key TEXT PRIMARY KEY, id TEXT, title TEXT);\nINSERT INTO seeded (_key, id, title) VALUES ('s1', 's1', 'from-initial-data');\n",
    )
    .expect("seed file");
    zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "default",
        "CREATE TABLE seeded (_key TEXT PRIMARY KEY, id TEXT, title TEXT)",
        &[],
        0,
        false,
    )
    .expect("create live table");
    zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "default",
        "INSERT INTO seeded (_key, id, title) VALUES ('live1', 'live1', 'live-only-row')",
        &[],
        0,
        false,
    )
    .expect("insert live row");

    let export_full = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/full")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("full export response");
    assert_eq!(export_full.status(), StatusCode::OK);
    let export_full = response_json(export_full).await;
    let full_archive = data_root.join("platform").join("project-operations").join(
        export_full["operation"]["artifact_rel_path"]
            .as_str()
            .expect("artifact path"),
    );
    let full_bytes = fs::read(&full_archive).expect("full archive bytes");

    // A repo + store import restores the snapshot and replays nothing.
    let (boundary, body) = multipart_import_body("superadmin", "full-clone", &full_bytes);
    let full_import = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("full import response");
    assert_eq!(full_import.status(), StatusCode::OK);
    let full_import = response_json(full_import).await;
    assert_eq!(full_import["store_auto_initiated"], json!(false));
    assert_eq!(full_import["initial_data_replayed"], json!([]));
    let cloned = zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "full-clone",
        "SELECT id FROM seeded",
        &[],
        0,
        true,
    )
    .expect("query full clone");
    let mut cloned_ids = cloned
        .rows
        .iter()
        .map(|row| row[0].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    cloned_ids.sort();
    // The source never replayed its declared seed, so the snapshot holds only
    // the live row — and the clone restores exactly that, replaying nothing.
    assert_eq!(cloned_ids, vec!["live1"], "snapshot wins whole");

    // A repo-only import auto-initiates: declared steps replay into a fresh
    // store, so the seeded row exists and the live-only row does not.
    let unpack = data_root.join("tmp-repo-only");
    fs::create_dir_all(&unpack).expect("unpack dir");
    let extract = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&full_archive)
        .arg("-C")
        .arg(&unpack)
        .status()
        .expect("tar extract");
    assert!(extract.success());
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(unpack.join("manifest.json")).expect("manifest"))
            .expect("manifest json");
    manifest["spec"]["classes"] = json!(["repo"]);
    let repo_digest = manifest["spec"]["class_digests"]["repo"].clone();
    manifest["spec"]["class_digests"] = json!({ "repo": repo_digest });
    let repo_count = manifest["spec"]["counts"]["repo"].clone();
    let total = manifest["spec"]["counts"]["total_bytes"].clone();
    manifest["spec"]["counts"] = json!({ "repo": repo_count, "total_bytes": total });
    fs::write(
        unpack.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("manifest bytes"),
    )
    .expect("manifest write");
    fs::remove_dir_all(unpack.join("store")).expect("drop store class");
    fs::remove_dir_all(unpack.join("files")).expect("drop files class");
    let repo_only_archive = data_root.join("repo-only.tar");
    let pack = std::process::Command::new("tar")
        .arg("-cf")
        .arg(&repo_only_archive)
        .arg("-C")
        .arg(&unpack)
        .arg(".")
        .status()
        .expect("tar create");
    assert!(pack.success());

    let repo_only_bytes = fs::read(&repo_only_archive).expect("repo-only bytes");
    let (boundary, body) = multipart_import_body("superadmin", "init-clone", &repo_only_bytes);
    let repo_only_import = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("repo-only import response");
    assert_eq!(repo_only_import.status(), StatusCode::OK);
    let repo_only_import = response_json(repo_only_import).await;
    assert_eq!(repo_only_import["store_auto_initiated"], json!(true));
    assert_eq!(
        repo_only_import["initial_data_replayed"],
        json!(["initial-data/sekejap/seed.sql"])
    );
    let fresh = zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "init-clone",
        "SELECT id FROM seeded",
        &[],
        0,
        true,
    )
    .expect("query fresh clone");
    let fresh_ids = fresh
        .rows
        .iter()
        .map(|row| row[0].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(fresh_ids, vec!["s1"], "fresh store has only declared data");

    // A second platform import at the same name refuses: it creates, never
    // replaces — replacing is the project-scope import's job.
    let (boundary, body) = multipart_import_body("superadmin", "init-clone", &repo_only_bytes);
    let duplicate = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("duplicate import response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn project_transfer_import_records_provenance_and_failures() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("transfer-failure");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let export_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    if export_bundle.status() != StatusCode::OK {
        let status = export_bundle.status();
        let body = response_text(export_bundle).await;
        panic!("bundle export failed with {status}: {body}");
    }
    let export_bundle = response_json(export_bundle).await;
    let bundle_op = export_bundle["operation"]["operation_id"]
        .as_str()
        .expect("bundle operation id");
    let bundle_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(bundle_op)
        .join("project.bundle.tar");

    let create_project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/projects")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "project": "other-project",
                        "title": "Other Project",
                        "runtime": {}
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create project response");
    assert_eq!(create_project.status(), StatusCode::OK);

    // Identity is provenance, never a gate (`kinds/project-bundle/README.md`):
    // importing superadmin/default's archive into other-project succeeds, the
    // provenance is recorded, and the displaced classes have recovery copies.
    let bundle_bytes = fs::read(&bundle_archive).expect("bundle archive bytes");
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bundle_bytes);
    let import_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    if import_bundle.status() != StatusCode::OK {
        let status = import_bundle.status();
        let body = response_text(import_bundle).await;
        panic!("foreign-provenance import failed with {status}: {body}");
    }
    let import_bundle = response_json(import_bundle).await;
    assert_eq!(import_bundle["import"]["provenance"], "superadmin/default");
    let recovery = import_bundle["import"]["recovery"]
        .as_array()
        .expect("recovery swaps");
    assert_eq!(recovery.len(), 2);
    for swap in recovery {
        let recovery_dir = data_root
            .join("users/superadmin/other-project/data/recovery")
            .join(swap["recovery_dir"].as_str().expect("recovery dir"));
        assert!(recovery_dir.is_dir(), "missing {}", recovery_dir.display());
    }

    // A genuinely broken archive fails, and the operation record says so.
    // (Operation ids carry second resolution; step past the completed one.)
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", b"not a tar archive");
    let import_garbage = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    assert_eq!(import_garbage.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let operations = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/operations")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operations response");
    assert_eq!(operations.status(), StatusCode::OK);
    let operations = response_json(operations).await;
    let items = operations["items"].as_array().expect("operation items");
    let failed = items
        .iter()
        .find(|item| item["kind"] == "import_bundle" && item["status"] == "failed")
        .expect("failed import record");
    assert_eq!(failed["current_step"], "import failed");
    let completed = items
        .iter()
        .find(|item| item["kind"] == "import_bundle" && item["status"] == "completed")
        .expect("completed import record");
    assert!(
        completed["current_step"]
            .as_str()
            .expect("completed step")
            .contains("import completed from 'superadmin/default'")
    );
}

#[test]
fn platform_project_authorization_is_policy_based_and_shared() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("project-authz");
    config.default_password = "test-pass".to_string();

    let platform = PlatformService::from_config(config).expect("platform service");

    let owner_subject = ProjectAccessSubject::user("superadmin");
    let owner_caps = platform
        .authz
        .resolve_project_capabilities(&owner_subject, "superadmin", "default")
        .expect("owner capabilities");
    assert!(owner_caps.contains(&ProjectCapability::TemplatesWrite));
    assert!(owner_caps.contains(&ProjectCapability::SettingsWrite));
    assert!(owner_caps.contains(&ProjectCapability::McpSessionCreate));

    let policies = platform
        .data
        .list_project_policies("superadmin", "default")
        .expect("project policies");
    assert!(policies.iter().any(|policy| policy.policy_id == "owner"));
    assert!(policies.iter().any(|policy| policy.policy_id == "viewer"));
    assert!(
        policies
            .iter()
            .any(|policy| policy.policy_id == "agent.templates")
    );

    let bindings = platform
        .data
        .list_project_policy_bindings("superadmin", "default")
        .expect("project policy bindings");
    assert!(
        bindings
            .iter()
            .any(|binding| { binding.subject_id == "superadmin" && binding.policy_id == "owner" })
    );

    platform
        .users
        .create_or_update_user(&CreateUserRequest {
            owner: "alice".to_string(),
            password: "alice-pass".to_string(),
            role: "member".to_string(),
            git_name: String::new(),
            git_email: String::new(),
        })
        .expect("create alice");
    let alice_subject = ProjectAccessSubject::user("alice");
    let alice_caps = platform
        .authz
        .resolve_project_capabilities(&alice_subject, "superadmin", "default")
        .expect("alice capabilities");
    assert!(alice_caps.is_empty());

    let err = platform
        .authz
        .ensure_project_capability(
            &alice_subject,
            "superadmin",
            "default",
            ProjectCapability::TemplatesRead,
        )
        .expect_err("alice must be denied");
    assert_eq!(err.code, "PLATFORM_AUTHZ_FORBIDDEN");
}

#[tokio::test]
async fn platform_template_diagnostics_reports_compile_errors() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("template-diagnostics");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/diagnostics")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"rel_path":"pages/home.tsx","content":"export default function Page(input) { return (<Page><main><div></main></Page>); }"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("diagnostics body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("\"ok\":false"));
    assert!(json.contains("\"severity\":\"error\""));
}

#[tokio::test]
async fn platform_registry_is_hierarchical_from_virtual_path() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("registry-tree");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let root_registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(root_registry.status(), axum::http::StatusCode::OK);
    let body = to_bytes(root_registry.into_body(), usize::MAX)
        .await
        .expect("root registry body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));

    let blog_registry = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/contents/blog")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(blog_registry.status(), axum::http::StatusCode::OK);
    let body = to_bytes(blog_registry.into_body(), usize::MAX)
        .await
        .expect("blog registry body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));
}

/// A static repository is a directory over HTTP, and it reaches the same gates.
///
/// `distribution.md` §3 says every channel runs one review. This proves it for
/// the channel that has no server behind it: the package is served as two plain
/// files, and the install refuses it for exactly the reasons a hub-served one
/// would be refused, through the same `ProjectBundleInstallPlan`.
///
/// It also proves the answer to the mutability question the same section asks.
/// The index pins the release document's digest, so replacing that document
/// under its path fails closed rather than installing whatever is there now.
#[tokio::test]
async fn a_static_repository_installs_through_the_same_review_and_pins_its_release() {
    unsafe {
        std::env::set_var("ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE", "1");
        // The two official sources are seeded into every instance and this test
        // must not reach the internet to find that out. Pointed at the discard
        // port they refuse instantly, which is also the state this test wants
        // them in: an unreachable source is reported, not dropped.
        std::env::set_var("ZEBFLOW_HUB_DEFAULT_BASE_URL", "http://127.0.0.1:9/api");
        std::env::set_var("ZEBFLOW_HUB_DEFAULT_STATIC_BASE_URL", "http://127.0.0.1:9");
    }

    // --- a publisher, only so the fixture is a real project bundle ---------
    let mut publisher_config = PlatformConfig::default();
    publisher_config.data_root = temp_test_dir("static-repo-publisher");
    publisher_config.default_password = "test-pass".to_string();
    let publisher = Arc::new(PlatformService::from_config(publisher_config).expect("publisher"));
    publisher
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "static-source".to_string(),
                title: Some("Static Source".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("source project");
    publisher
        .projects
        .write_repo_file(
            "superadmin",
            "static-source",
            "pages/home.tsx",
            "export default function Home() { return <main>Static</main>; }\n",
        )
        .expect("template");
    publisher
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "static-source",
            "pipelines/pages/home.zf.json",
            "Home",
            "Serve the home page.",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"home"},
  "spec":{
  "id":"home",
  "entry_nodes":["trigger"],
  "nodes":[
    {"id":"trigger","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/","method":"GET"}},
    {"id":"response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home.tsx"}}
  ],
  "edges":[
    {"from_node":"trigger","from_pin":"out","to_node":"response","to_pin":"in"}
  ]}
}"#,
        )
        .expect("pipeline");
    publisher
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1:1/api", true)
        .expect("enable publisher hub");
    publisher
        .hub
        .upsert_publisher(
            "superadmin",
            "static-source",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            false,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher identity");
    publisher
        .hub
        .publish_asset(
            "superadmin",
            "static-source",
            "superadmin",
            "zebflow-labs",
            "",
            "",
            "",
            "superadmin",
            "static-source",
            "project_files",
            ".",
            "static-tool",
            "1.0.0",
            "Static Tool",
            "A project bundle served by a plain directory.",
            "",
            "public",
            zebflow::platform::services::hub::HubProjectBundlePublishOptions::default(),
            vec!["demo".to_string()],
        )
        .expect("publish project bundle");
    let (_, document) = publisher
        .hub
        .get_asset_version_artifact("zebflow-labs.static-tool", "1.0.0")
        .expect("artifact document");

    // --- the repository: an index and one document, and nothing else -------
    let static_root = temp_test_dir("static-repo-files");
    let release_rel = "packages/zebflow-labs.static-tool/1.0.0/package.json";
    let release_path = static_root.join(release_rel);
    fs::create_dir_all(release_path.parent().expect("release dir")).expect("release dir");
    let release_bytes = serde_json::to_vec_pretty(&document).expect("release bytes");
    fs::write(&release_path, &release_bytes).expect("release file");
    let write_index = |sha256: &str, size_bytes: usize| {
        fs::write(
            static_root.join("zebflow-repository.json"),
            serde_json::to_vec_pretty(&json!({
                "apiVersion": "zebflow.com/v1",
                "kind": "HubRepositoryIndex",
                "metadata": {"name": "test-repository"},
                "spec": {"packages": [{
                    "package_id": "zebflow-labs.static-tool",
                    "asset_kind": "project_bundle",
                    "title": "Static Tool",
                    "description": "A project bundle served by a plain directory.",
                    "latest_version": "1.0.0",
                    "releases": [{
                        "version": "1.0.0",
                        "path": release_rel,
                        "sha256": sha256,
                        "size_bytes": size_bytes,
                    }],
                }]},
            }))
            .expect("index bytes"),
        )
        .expect("index file");
    };
    write_index(&sha256_hex(&release_bytes), release_bytes.len());

    // A plain file server. No Zebflow, no API, no logic: the whole point of the
    // channel is that this is all a repository has to be.
    let served_root = static_root.clone();
    let files = axum::Router::new().route(
        "/{*path}",
        axum::routing::get(
            move |axum::extract::Path(path): axum::extract::Path<String>| {
                let root = served_root.clone();
                async move {
                    use axum::response::IntoResponse as _;
                    match fs::read(root.join(path)) {
                        Ok(bytes) => (StatusCode::OK, bytes).into_response(),
                        Err(_) => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            },
        ),
    );
    let files_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("file listener");
    let files_addr = files_listener.local_addr().expect("file addr");
    let files_server = tokio::spawn(async move {
        axum::serve(files_listener, files)
            .await
            .expect("file server");
    });

    // --- a consumer that has that directory as its second source -----------
    let mut consumer_config = PlatformConfig::default();
    consumer_config.data_root = temp_test_dir("static-repo-consumer");
    consumer_config.default_password = "test-pass".to_string();
    let consumer = Arc::new(PlatformService::from_config(consumer_config).expect("consumer"));
    let consumer_app = zebflow::platform::web::router(consumer.clone()).await;
    let cookie = login_cookie(consumer_app.clone(), "superadmin", "test-pass").await;
    let repository = consumer
        .hub
        .upsert_platform_repository(
            "superadmin",
            "test-static",
            "Test Static Repository",
            &format!("http://{files_addr}"),
            "",
            "",
            "",
            "static",
            Some(20),
            "public",
            true,
        )
        .expect("static source");
    assert_eq!(repository.kind, "static");
    assert_eq!(repository.priority, 20);

    let body = json!({
        "repository_id": "test-static",
        "package_id": "zebflow-labs.static-tool",
        "version": "1.0.0",
    })
    .to_string();
    let post = |uri: &'static str, body: String, cookie: String, app: axum::Router| async move {
        app.oneshot(
            Request::builder()
                .uri(uri)
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response")
    };

    // The listing walks the configured sources in order and names each one.
    let listing = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/assets")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("listing request"),
        )
        .await
        .expect("listing response");
    let listing = response_json(listing).await;
    let sources = listing["sources"].as_array().expect("sources array");
    // Priority ascending, then repository id: the official static source at
    // 10 first, then this test's static source and the official API hub at
    // 20, id-ordered (`distribution.md` §2, decided 2026-08-27).
    assert_eq!(
        sources
            .iter()
            .map(|item| item["repository_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["zebflow-hub", "test-static", "zebflow-com"],
        "sources are returned in resolution order: {listing}"
    );
    assert_eq!(sources[1]["kind"], json!("static"), "{listing}");
    assert_eq!(sources[1]["ok"], json!(true), "{listing}");
    // A source that did not answer is named as such rather than dropped, which
    // is what lets one "not found" error tell the two cases apart.
    assert_eq!(sources[2]["ok"], json!(false), "{listing}");
    assert!(
        !sources[2]["error"].as_str().unwrap_or_default().is_empty(),
        "an unreachable source says why: {listing}"
    );
    assert_eq!(
        listing["items"]
            .as_array()
            .expect("items")
            .iter()
            .filter(|item| item["repository_id"] == json!("test-static"))
            .count(),
        1,
        "the static repository's package is listed: {listing}"
    );

    let review = post(
        "/api/platform/hub/install/review",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let review_status = review.status();
    let review = response_json(review).await;
    assert_eq!(review_status, StatusCode::OK, "{review}");
    assert_eq!(review["review"]["installable"], json!(true), "{review}");

    let install = post(
        "/api/platform/hub/install",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let install_status = install.status();
    let install = response_json(install).await;
    assert_eq!(install_status, StatusCode::OK, "{install}");
    let installed = install["install"]["project"]
        .as_str()
        .expect("installed project")
        .to_string();
    assert!(
        consumer
            .projects
            .get_project("superadmin", &installed)
            .expect("project lookup")
            .is_some(),
        "a static repository materialises a project like any other channel"
    );

    // --- the release is pinned: replacing it under its path fails closed ---
    let mut tampered: Value = serde_json::from_slice(&release_bytes).expect("document json");
    tampered["spec"]["description"] = json!("Replaced after the index was published.");
    fs::write(
        &release_path,
        serde_json::to_vec_pretty(&tampered).expect("tampered bytes"),
    )
    .expect("tampered file");

    let refused = post(
        "/api/platform/hub/install",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let refused = response_json(refused).await;
    assert_eq!(
        refused["error"]["code"],
        json!("HUB_REMOTE_HASH_MISMATCH"),
        "a document swapped under its path is refused: {refused}"
    );

    // --- and the review's refusals are the install's refusals --------------
    // A pipeline whose bytes the review cannot read is a violation, and it is a
    // violation on this channel too, reached through the same plan.
    let mut hostile: Value = serde_json::from_slice(&release_bytes).expect("document json");
    for file in hostile["spec"]["files"]
        .as_array_mut()
        .expect("files array")
        .iter_mut()
    {
        if file["rel_path"]
            .as_str()
            .is_some_and(|path| path.ends_with(".zf.json"))
        {
            let raw = vec![0x00_u8, 0x01, 0x02, 0xff, 0xfe].repeat(20);
            file["encoding"] = json!("base64");
            file["content"] = json!(base64::engine::general_purpose::STANDARD.encode(&raw));
            file["size_bytes"] = json!(raw.len());
        }
    }
    let hostile_bytes = serde_json::to_vec_pretty(&hostile).expect("hostile bytes");
    fs::write(&release_path, &hostile_bytes).expect("hostile file");
    write_index(&sha256_hex(&hostile_bytes), hostile_bytes.len());

    let refused = post(
        "/api/platform/hub/install",
        body,
        cookie,
        consumer_app.clone(),
    )
    .await;
    let refused = response_json(refused).await;
    assert_eq!(
        refused["error"]["code"],
        json!("HUB_REMOTE_INSTALL_REFUSED"),
        "the one review refuses a static repository's package too: {refused}"
    );

    files_server.abort();
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// ---------------------------------------------------------------------------
// `offices.md` §4 login, §6 break-glass, §7 leaving.
// ---------------------------------------------------------------------------

/// Attempt a local password login and report what came back.
async fn attempt_login(
    app: &axum::Router,
    identifier: &str,
    password: &str,
) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "identifier={identifier}&password={password}"
                )))
                .expect("request"),
        )
        .await
        .expect("login response");
    let status = response.status();
    (status, response_text(response).await)
}

/// The office's own local-authority service, opened against a stopped instance.
///
/// This is exactly what `zeb admin break-glass` and `zeb admin detach` do once
/// their probe has established that nothing is serving the data root: open the
/// catalog directly and act. Doing it this way in a test rather than shelling
/// out keeps the assertion on the behaviour instead of on argv handling, and it
/// is the same code path.
fn host_authority(data_root: &Path) -> zebflow::platform::services::OfficeLocalAuthorityService {
    let data = zebflow::platform::adapters::data::build_data_adapter(
        zebflow::platform::model::DataAdapterKind::Sqlite,
        data_root,
    )
    .expect("catalog adapter");
    zebflow::platform::services::OfficeLocalAuthorityService::new(data, data_root.to_path_buf())
}

async fn get_json(app: &axum::Router, uri: &str, cookie: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    (status, response_json(response).await)
}

/// `offices.md` §4: "Local accounts are disabled for login while joined; the
/// controller's identity is the normal door."
///
/// Both halves of that sentence are one test, because either alone is a bug:
/// a closed local door with no controller door is an office nobody can reach,
/// and an open local door is the term unimplemented. A standalone instance is
/// built beside them as the control — nothing here may leak onto an instance
/// that joined nobody.
#[tokio::test]
async fn a_joined_office_refuses_local_login_and_opens_for_its_controller() {
    let controller_root = temp_test_dir("login-term-controller");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let controller_app = build_router(controller).await.expect("controller starts");
    let controller_cookie = login_cookie(controller_app.clone(), "superadmin", "test-pass").await;

    let office_root = temp_test_dir("login-term-office");
    let token = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-a",
        "http://office-a.example:10610",
    )
    .await;
    let office_app = build_office(&office_root, &token).await;

    // --- (h) the control: a standalone instance is untouched ---------------
    let standalone_root = temp_test_dir("login-term-standalone");
    let mut standalone = PlatformConfig::default();
    standalone.data_root = standalone_root.clone();
    standalone.default_password = "test-pass".to_string();
    let standalone_app = build_router(standalone).await.expect("standalone starts");
    assert_eq!(
        attempt_login(&standalone_app, "superadmin", "test-pass")
            .await
            .0,
        StatusCode::SEE_OTHER,
        "an instance that joined nobody logs in exactly as before"
    );

    // --- (a) the joined office refuses, actionably --------------------------
    let (status, body) = attempt_login(&office_app, "superadmin", "test-pass").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    for expected in [
        "joined a controller",
        "office-a",
        "vouch",
        "zeb admin break-glass",
        "zeb admin detach",
    ] {
        assert!(
            body.contains(expected),
            "the refusal must say '{expected}': {body}"
        );
    }

    // The refusal is the same for a good password, a bad one, and a name this
    // office has never held. Nothing about the credential leaks through a door
    // that is closed for a different reason.
    let good = attempt_login(&office_app, "superadmin", "test-pass").await;
    let bad = attempt_login(&office_app, "superadmin", "wrong-pass").await;
    let unknown = attempt_login(&office_app, "nobody-here", "wrong-pass").await;
    assert_eq!(good, bad);
    assert_eq!(good, unknown);
    assert!(
        !body.contains("invalid credentials"),
        "a refused mechanism must not be dressed as a refused credential: {body}"
    );

    // --- (b) the controller's door still opens the same office --------------
    let (status, vouch) = mint_vouch(&controller_app, &controller_cookie, "office-a").await;
    assert_eq!(status, StatusCode::OK);
    let vouch = vouch["vouch"]["vouch"].as_str().expect("vouch").to_string();
    let (status, _body, cookie) = redeem_vouch(&office_app, &vouch).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the operator must not be locked out of a joined office"
    );
    let cookie = cookie.expect("the office issues its own session");

    // And that session is a real one: it reads the office's own state.
    let (status, state) = get_json(&office_app, "/api/platform/office/local-authority", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state["joined"], json!(true));
    assert_eq!(state["office_id"], json!("office-a"));
    assert_eq!(state["local_login_allowed"], json!(false));

    let _ = fs::remove_dir_all(controller_root);
    let _ = fs::remove_dir_all(office_root);
    let _ = fs::remove_dir_all(standalone_root);
}

/// `offices.md` §6: break-glass re-enables local authority, needs no quorum and
/// no controller, and its use is recorded locally and reported on reconnect.
///
/// The controller in this test is a *different* router from the one the office
/// was configured to reach: the office's `master_url` is `127.0.0.1:1`, which
/// answers nothing. Everything up to the report therefore happens with the
/// controller genuinely unreachable, which is the property §6 exists for.
#[tokio::test]
async fn break_glass_re_enables_local_login_without_a_controller_and_is_reported_later() {
    let controller_root = temp_test_dir("break-glass-controller");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let controller_app = build_router(controller).await.expect("controller starts");
    let controller_cookie = login_cookie(controller_app.clone(), "superadmin", "test-pass").await;
    let token = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-a",
        "http://office-a.example:10610",
    )
    .await;

    let office_root = temp_test_dir("break-glass-office");
    let office_app = build_office(&office_root, &token).await;
    assert_eq!(
        attempt_login(&office_app, "superadmin", "test-pass")
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    drop(office_app);

    // --- (c) the act, on the host, with nothing to talk to ------------------
    let authority = host_authority(&office_root);
    let event = authority
        .break_glass("superadmin", "local authority re-enabled from the host")
        .expect("break-glass needs no controller and no quorum");
    assert_eq!(event.office_id, "office-a");
    assert_eq!(event.reported_at, 0);
    // §6 re-enables local authority. §7 is what leaves. The office is still an
    // office.
    assert!(
        office_root.join("platform/office-join-token").is_file(),
        "break-glass must not detach"
    );

    let office_app = build_office(&office_root, &token).await;
    let (status, _body) = attempt_login(&office_app, "superadmin", "test-pass").await;
    assert_eq!(
        status,
        StatusCode::SEE_OTHER,
        "the operator gets in after breaking the glass"
    );
    // A wrong password is a wrong password again, not a closed mechanism.
    assert_eq!(
        attempt_login(&office_app, "superadmin", "wrong-pass")
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );

    // --- (d) recorded locally, read by this office's own operator -----------
    let office_cookie = login_cookie(office_app.clone(), "superadmin", "test-pass").await;
    let (status, state) =
        get_json(&office_app, "/api/platform/office/local-authority", &office_cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state["joined"], json!(true));
    assert_eq!(state["local_login_allowed"], json!(true));
    assert_eq!(
        state["break_glass_in_force"]["event_id"],
        json!(event.event_id)
    );
    assert_eq!(state["events"][0]["event"], json!("break-glass"));
    assert_eq!(state["events"][0]["reported_at"], json!(0));

    // --- (e) reported to the controller when it comes back ------------------
    let pending = authority.unreported_break_glass().expect("pending");
    assert_eq!(pending.len(), 1);
    let response = controller_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/offices/break-glass")
                .method("POST")
                .header("x-zebflow-cluster-token", &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "office_id": "office-a", "events": pending }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("report response");
    assert_eq!(response.status(), StatusCode::OK);
    let accepted = response_json(response).await;
    assert_eq!(accepted["accepted"][0], json!(event.event_id));

    let (status, seen) = get_json(
        &controller_app,
        "/api/platform/cluster/office-break-glass",
        &controller_cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seen["events"][0]["event_id"], json!(event.event_id));
    assert_eq!(seen["events"][0]["office_id"], json!("office-a"));
    assert!(
        seen["events"][0]["reported_at"].as_i64().unwrap_or(0) > 0,
        "the controller stamps arrival: {seen}"
    );

    // One office cannot file a break-glass under another office's name.
    let other = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-b",
        "http://office-b.example:10610",
    )
    .await;
    let response = controller_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/offices/break-glass")
                .method("POST")
                .header("x-zebflow-cluster-token", &other)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "office_id": "office-a", "events": pending }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("crossed report response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let _ = fs::remove_dir_all(controller_root);
    let _ = fs::remove_dir_all(office_root);
}

/// `offices.md` §7: on detach the office keeps everything and its local
/// accounts become live again — and §8's vouch-created account stays reachable
/// by vouch and by nothing else, through both acts.
#[tokio::test]
async fn detach_keeps_everything_and_a_vouched_account_never_becomes_a_local_door() {
    let controller_root = temp_test_dir("detach-controller");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let controller_app = build_router(controller).await.expect("controller starts");
    let controller_cookie = login_cookie(controller_app.clone(), "superadmin", "test-pass").await;
    // The controller operator is somebody the office has never held.
    create_user(&controller_app, &controller_cookie, "remote-admin").await;
    let remote_cookie = login_cookie(controller_app.clone(), "remote-admin", "remote-pass").await;
    let token = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-a",
        "http://office-a.example:10610",
    )
    .await;

    let office_root = temp_test_dir("detach-office");
    let office_app = build_office(&office_root, &token).await;

    // A vouch creates `remote-admin` on the office, with no usable password.
    let (_, vouch) = mint_vouch(&controller_app, &remote_cookie, "office-a").await;
    let vouch = vouch["vouch"]["vouch"].as_str().expect("vouch").to_string();
    let (status, redeemed, _) = redeem_vouch(&office_app, &vouch).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(redeemed["owner"], json!("remote-admin"));
    assert_eq!(redeemed["action"], json!("created"));

    // What the office holds before anything is broken or left.
    let before = office_inventory(&office_root);
    assert!(
        !before.is_empty(),
        "an office seeds its own shelf and default project"
    );
    drop(office_app);

    // --- (g) break-glass, and the vouched account is still unreachable ------
    let authority = host_authority(&office_root);
    authority
        .break_glass("superadmin", "test")
        .expect("break-glass");
    let office_app = build_office(&office_root, &token).await;
    assert_eq!(
        attempt_login(&office_app, "superadmin", "test-pass")
            .await
            .0,
        StatusCode::SEE_OTHER
    );
    for guess in ["remote-pass", "test-pass", "", "remote-admin"] {
        assert_eq!(
            attempt_login(&office_app, "remote-admin", guess).await.0,
            StatusCode::UNAUTHORIZED,
            "a controller-created account must not become a local door"
        );
    }
    drop(office_app);

    // --- (f) detach ---------------------------------------------------------
    let event = authority.detach("test").expect("detach");
    assert_eq!(event.event, "detach");
    assert!(
        !office_root.join("platform/office-join-token").is_file(),
        "leaving removes the membership and nothing else"
    );
    assert_eq!(
        office_inventory(&office_root),
        before,
        "projects, data, files, and the blessed shelf survive a detach unchanged"
    );

    // Detached, it is a complete instance: no cluster configuration at all.
    let mut detached = PlatformConfig::default();
    detached.data_root = office_root.clone();
    detached.default_password = "unused-after-first-boot".to_string();
    let detached_app = build_router(detached)
        .await
        .expect("detached instance starts");
    assert_eq!(
        attempt_login(&detached_app, "superadmin", "test-pass")
            .await
            .0,
        StatusCode::SEE_OTHER,
        "local accounts were disabled, never deleted"
    );
    // Still true after leaving.
    for guess in ["remote-pass", "test-pass", ""] {
        assert_eq!(
            attempt_login(&detached_app, "remote-admin", guess).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    // The record of both acts survives, and the detach is not queued for a
    // report it can no longer authenticate.
    let cookie = login_cookie(detached_app.clone(), "superadmin", "test-pass").await;
    let (status, state) = get_json(&detached_app, "/api/platform/office/local-authority", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state["joined"], json!(false));
    assert_eq!(state["local_login_allowed"], json!(true));
    assert_eq!(state["events"].as_array().expect("events").len(), 2);
    // The break-glass is still unreported and now unreportable — this office
    // gave up the credential that would authenticate the report. The detach is
    // not queued at all, because §6 is the sentence that asks for a report and
    // §7 is not.
    let pending = authority.unreported_break_glass().expect("pending");
    assert_eq!(pending.len(), 1);
    assert!(pending.iter().all(|entry| entry.is_break_glass()));

    let _ = fs::remove_dir_all(controller_root);
    let _ = fs::remove_dir_all(office_root);
}

/// Every project path and blessed-shelf coordinate an instance holds.
///
/// Deliberately a whole-tree listing rather than a count: §7 says the office
/// keeps its projects, data, files, and shelf, and a count would pass while a
/// file moved.
fn office_inventory(data_root: &Path) -> Vec<String> {
    let mut items = blessed_shelf_coordinates(data_root);
    let users_dir = data_root.join("users");
    let mut walk = vec![users_dir.clone()];
    while let Some(dir) = walk.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(data_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            items.push(rel);
            if path.is_dir() {
                walk.push(path);
            }
        }
    }
    items.sort();
    items
}

/// One office's join token is **not** a project credential on its controller.
///
/// `offices.md` §8 makes a token "revocable for one office alone" and §2 gives
/// the controller exactly three verbs. Before this, every project-capability
/// check short-circuited on "is this any active office's token?", with the owner
/// taken from the URL — so any office could export or overwrite any user's
/// project on the controller, through `/api/internal/project-transfer/...` or
/// through any project route at all. That made §8's term false and §2's three
/// verbs a fiction, and it is what this test refuses.
///
/// The same token is proven to still work for the three things an office
/// legitimately calls its controller about, so the fix is a narrowing and not a
/// removal.
#[tokio::test]
async fn an_office_join_token_is_not_a_project_credential_on_its_controller() {
    let controller_root = temp_test_dir("controller-token-is-not-a-credential");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let app = build_router(controller).await.expect("controller starts");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let token = mint_join_token(&app, &cookie, "office-a", false).await.1["token"]
        .as_str()
        .expect("token")
        .to_string();

    // The office is a real, registered, heartbeating member. Nothing about the
    // refusals below is "this token was never valid".
    let nonce = "fedcba9876543210";
    assert_eq!(
        register_office(&app, &token, "office-a", nonce).await.0,
        StatusCode::OK
    );
    assert_eq!(
        heartbeat_office(&app, &token, "office-a").await.0,
        StatusCode::OK
    );

    // Every route the exploit reached, with the token and no session cookie.
    // `superadmin/default` is the controller's own first-boot project, so each
    // of these is one instance's operator acting as another's.
    let attempts: Vec<(&str, &str, String)> = vec![
        // The reviewer's route: read every byte of somebody else's project.
        (
            "POST",
            "/api/internal/project-transfer/superadmin/default/export/full",
            "{}".to_string(),
        ),
        // And the other direction: overwrite it.
        (
            "POST",
            "/api/internal/project-transfer/superadmin/default/import/full",
            "{}".to_string(),
        ),
        // Run somebody else's pipelines on their instance. The body is
        // well-formed on purpose, so the refusal is the auth gate and not a
        // deserialisation failure standing in for one.
        (
            "POST",
            "/api/internal/runtime/execute/superadmin/default",
            json!({
                "file_rel_path": "pipelines/anything.zf.json",
                "trigger": "manual",
            })
            .to_string(),
        ),
        // And the ordinary project surface, which the capability check gates.
        (
            "GET",
            "/api/projects/superadmin/default/pipelines",
            String::new(),
        ),
        (
            "GET",
            "/api/projects/superadmin/default/repo",
            String::new(),
        ),
        (
            "GET",
            "/api/projects/superadmin/default/settings/rwe",
            String::new(),
        ),
    ];
    for (method, uri, body) in attempts {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method(method)
                    .header("x-zebflow-cluster-token", &token)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "an office's token must not authorise {method} {uri} on its controller"
        );
    }

    // The narrowing did not break the three things an office really does call
    // its controller about: it is still registering, still heartbeating, and
    // still able to file a break-glass report.
    let reported = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/offices/break-glass")
                .method("POST")
                .header("x-zebflow-cluster-token", &token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "office_id": "office-a", "events": [] }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("report response");
    assert_eq!(reported.status(), StatusCode::OK);
    assert_eq!(
        heartbeat_office(&app, &token, "office-a").await.0,
        StatusCode::OK,
        "an office must still be an office"
    );

    let _ = fs::remove_dir_all(&controller_root);
}

/// What a controller stores about an office forges nothing at that office.
///
/// The original exploit: every controller-to-office proof was an `HMAC` keyed by
/// `sha256(secret)` — exactly the value the controller stores, and exactly the
/// value that used to come back from `GET /api/platform/cluster/join-tokens` and from the
/// mint response. Verifier equalled forger. A reviewer minted a vouch for an
/// identity the office had never held and got a `superadmin` account created,
/// then forged the controller-call header and walked past the office's project
/// auth with no cookie.
///
/// This test *is* that attacker: it holds the digest, and it tries both.
#[tokio::test]
async fn the_material_a_controller_stores_forges_nothing_at_its_office() {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn hmac_hex(key: &str, message: &str) -> String {
        let mut mac = <Hmac<Sha256>>::new_from_slice(key.as_bytes()).expect("any key length");
        mac.update(message.as_bytes());
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    let controller_root = temp_test_dir("controller-forgery");
    let office_root = temp_test_dir("office-forgery");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    let controller_app = build_router(controller).await.expect("controller starts");
    let controller_cookie = login_cookie(controller_app.clone(), "superadmin", "test-pass").await;

    let token = mint_join_token_at(
        &controller_app,
        &controller_cookie,
        "office-a",
        "http://office-a.example:10610",
    )
    .await;
    let office = build_office(&office_root, &token).await;

    // The attacker's whole holding: the digest the controller stores. Standing
    // in for a database read, a leaked backup, or the API response that used to
    // hand it over.
    let parsed = JoinToken::parse(&token).expect("parse");
    let stolen_digest = parsed.secret_digest();

    // --- the forged vouch --------------------------------------------------
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    let expires_at = now + 60;
    // Exactly the construction the old release used, and the new scheme word
    // as well, so the refusal is not merely "we renamed the prefix".
    for (scheme, message, expected) in [
        // The construction the exploit actually used. Refused at the scheme,
        // because a `zfjoin1v` value is not a vouch this release accepts at all.
        (
            "zfjoin1v",
            format!("zfjoin1/vouch:office-a:attacker-implanted:{expires_at}:n1"),
            "CLUSTER_VOUCH_MALFORMED",
        ),
        // The same construction wearing this release's scheme word, so the
        // refusal cannot be dismissed as a rename.
        (
            "zfjoin2v",
            format!("zfjoin2/vouch:office-a:attacker-implanted:{expires_at}:n1"),
            "CLUSTER_VOUCH_INVALID",
        ),
        // And with this release's exact signed message, fingerprint included:
        // the attacker knows the construction and still cannot key it.
        (
            "zfjoin2v",
            format!(
                "zfjoin2/vouch:office-a:attacker-implanted:{expires_at}:n1:{}",
                parsed.fingerprint()
            ),
            "CLUSTER_VOUCH_INVALID",
        ),
    ] {
        let forged = format!(
            "{scheme}:office-a:attacker-implanted:{expires_at}:n1:{}",
            hmac_hex(&stolen_digest, &message)
        );
        let refused = redeem_vouch(&office, &forged).await;
        assert!(
            refused.0.is_client_error(),
            "a vouch forged from stored material must not open an office: {refused:?}"
        );
        assert_eq!(refused.1["error"]["code"], json!(expected), "{refused:?}");
        assert!(refused.2.is_none(), "a refused vouch issues no session");
    }

    // Nothing was implanted. The office's own accounts log is the place §8
    // makes that checkable, and it has no row for the name that was attempted.
    let office_cookie = {
        // §6, run on the host: the only way to read this office's own log while
        // it is joined. Exactly what `zeb admin break-glass` does.
        let authority = host_authority(&office_root);
        authority
            .break_glass("", "reading this office's own log")
            .expect("break glass");
        login_cookie(office.clone(), "superadmin", "test-pass").await
    };
    let writes = response_json(
        office
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/platform/office/identity-writes")
                    .header(header::COOKIE, &office_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("identity writes"),
    )
    .await;
    assert!(
        !serde_json::to_string(&writes["writes"])
            .expect("json")
            .contains("attacker-implanted"),
        "a forged vouch must reach no account: {writes}"
    );

    // --- the forged controller-call header ---------------------------------
    // The second half of the exploit: with the header accepted, the office's
    // project-capability check short-circuits and every project route opens
    // with no cookie at all.
    for (scheme, message) in [
        ("zfjoin1c", "zfjoin1/controller-call:office-a".to_string()),
        ("zfjoin2c", "zfjoin2/controller-call:office-a".to_string()),
        (
            "zfjoin2c",
            format!("zfjoin2/controller-call:office-a:{}", parsed.fingerprint()),
        ),
    ] {
        let forged = format!("{scheme}:office-a:{}", hmac_hex(&stolen_digest, &message));
        let response = office
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projects/superadmin/default/pipelines")
                    .header("x-zebflow-cluster-token", &forged)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "a header forged from stored material must not pass an office's project auth"
        );
    }

    // And the honest header, derived from the controller's private key, is
    // still accepted — so this is a narrowing and not a removal.
    let signing = controller_signing_key(&controller_root);
    let honest = format!(
        "zfjoin2c:office-a:{}",
        signing.sign(&format!(
            "zfjoin2/controller-call:office-a:{}",
            parsed.fingerprint()
        ))
    );
    let response = office
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines")
                .header("x-zebflow-cluster-token", &honest)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the office must still accept its own controller"
    );

    let _ = fs::remove_dir_all(&controller_root);
    let _ = fs::remove_dir_all(&office_root);
}

/// Being logged out must never grant more than being logged in without the
/// capability. The pipeline upsert route carried an early-development
/// convenience that skipped its capability check whenever no session cookie was
/// present, so anyone who could reach the port could create or overwrite an
/// executable pipeline and hang a webhook trigger on it.
#[tokio::test]
async fn registering_a_pipeline_always_requires_the_write_capability() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("pipeline-upsert-authz");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");

    let definition = |file_rel_path: &str| {
        json!({
            "file_rel_path": file_rel_path,
            "title": "Backdoor",
            "description": "",
            "trigger_kind": "webhook",
            "source": r#"{"apiVersion":"zebflow.com/v1","kind":"Pipeline","metadata":{"name":"backdoor"},"spec":{"id":"backdoor","entry_nodes":["wh"],"nodes":[{"id":"wh","kind":"n.trigger.webhook","output_pins":["out"],"input_pins":[],"config":{"path":"/backdoor","method":"POST"}}],"edges":[]}}"#
        })
        .to_string()
    };

    let anonymous = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/definition")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(definition("anon/backdoor")))
                .expect("request"),
        )
        .await
        .expect("anonymous upsert response");
    assert_eq!(
        anonymous.status(),
        StatusCode::UNAUTHORIZED,
        "an unauthenticated caller must not be able to register a pipeline"
    );

    let repo_dir = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("repo");
    assert!(
        !repo_dir.join("pipelines/anon/backdoor.zf.json").exists(),
        "the refused request must not have written a pipeline"
    );

    // A forged cookie is not a session either.
    let forged = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(definition("forged/backdoor")))
                .expect("request"),
        )
        .await
        .expect("forged upsert response");
    assert_eq!(forged.status(), StatusCode::UNAUTHORIZED);

    let superadmin_cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let create_user = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/users")
                .method("POST")
                .header(header::COOKIE, &superadmin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"owner": "mallory", "password": "mallory-pass", "role": "member"})
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create user response");
    assert_eq!(create_user.status(), StatusCode::OK);

    let mallory_cookie = login_cookie(app.clone(), "mallory", "mallory-pass").await;
    let other_user = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, mallory_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(definition("mallory/backdoor")))
                .expect("request"),
        )
        .await
        .expect("other user upsert response");
    assert_eq!(other_user.status(), StatusCode::FORBIDDEN);

    // The owner still registers normally — the route is closed, not broken.
    let owner = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, &superadmin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(definition("owned/feed")))
                .expect("request"),
        )
        .await
        .expect("owner upsert response");
    assert_eq!(owner.status(), StatusCode::OK);

    let _ = fs::remove_dir_all(&data_root);
}

/// A published map layer must serve at the path the contract says it is stored
/// at, and must hand out only the properties the operator named. Both halves
/// were broken at once: the web publisher stored `path` with a leading slash
/// that the serving lookup could never match, and the resolver treated an empty
/// `allowed_properties` as "everything" instead of "geometry only".
#[tokio::test]
async fn a_web_published_layer_serves_and_shows_only_the_chosen_properties() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("mapserver-publish-serving");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let files_dir = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files");
    fs::create_dir_all(files_dir.join("mapserver")).expect("mapserver dir");
    fs::write(
        files_dir.join("mapserver").join("roads.geojson"),
        json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": { "name": "Jalan Sudirman", "owner_phone": "+62811000000" },
                "geometry": { "type": "Point", "coordinates": [106.8, -6.2] }
            }]
        })
        .to_string(),
    )
    .expect("source geojson");

    let publish = |allowed: Value| {
        json!({
            "layer_id": "roads",
            "path": "/roads",
            "source_path": "mapserver/roads.geojson",
            "bbox_required": false,
            "allowed_properties": allowed
        })
        .to_string()
    };

    let published = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/mapserver/default-mapserver/layers")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(publish(json!([]))))
                .expect("request"),
        )
        .await
        .expect("publish response");
    assert_eq!(published.status(), StatusCode::OK);
    let published = response_json(published).await;
    assert_eq!(
        published["item"]["path"],
        json!("roads"),
        "the contract stores `path` without a leading slash"
    );
    assert_eq!(
        published["public_url"],
        json!("/ms/superadmin/default/roads")
    );

    let registry_raw = fs::read_to_string(
        files_dir
            .join("mapserver")
            .join("default-mapserver.layers.json"),
    )
    .expect("registry file");
    assert!(
        !registry_raw.contains("\"/roads\""),
        "the stored path must not carry a leading slash: {registry_raw}"
    );

    // Addressing (docs/contracts/addressing.md §2): the `ms` surface is off
    // until the operator switches it on, and a disabled surface answers 404
    // on the platform form too. The publish itself is not the switch.
    let closed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ms/superadmin/default/roads?limit=10")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("serve response");
    assert_eq!(closed.status(), StatusCode::NOT_FOUND, "ms is off by default");
    let switched = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/settings/addressing")
                .method("PUT")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "data": { "hosts": [], "routes": [], "disabled": ["fs"] } }).to_string()))
                .expect("request"),
        )
        .await
        .expect("addressing write");
    assert_eq!(switched.status(), StatusCode::OK, "switch the ms surface on");

    let served = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ms/superadmin/default/roads?limit=10")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("serve response");
    assert_eq!(
        served.status(),
        StatusCode::OK,
        "a layer published through the web UI must serve once the surface is on"
    );
    let served = response_json(served).await;
    assert_eq!(served["count"], json!(1));
    assert_eq!(
        served["features"][0]["properties"],
        json!({}),
        "an empty allowed_properties is geometry only, not a wildcard"
    );
    assert!(served["features"][0]["geometry"].is_object());

    // The public stats endpoint never reports a hidden column. (An
    // artifact-backed layer has no single file to scan, so it reports none at
    // all here; `mapserver::resolve::stats` covers the pruning itself.)
    let public_stats = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ms/superadmin/default/roads/stats")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("public stats response");
    assert_eq!(public_stats.status(), StatusCode::OK);
    let public_stats = response_json(public_stats).await;
    assert_eq!(public_stats["columns"], json!([]));

    // Naming a property exposes exactly that one.
    let republish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/mapserver/default-mapserver/layers")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(publish(json!(["name"]))))
                .expect("request"),
        )
        .await
        .expect("republish response");
    assert_eq!(republish.status(), StatusCode::OK);

    let served = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/ms/superadmin/default/roads?limit=10")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("serve response");
    assert_eq!(served.status(), StatusCode::OK);
    let served = response_json(served).await;
    assert_eq!(
        served["features"][0]["properties"],
        json!({ "name": "Jalan Sudirman" }),
        "only the named property leaves the server"
    );

    // The operator view is authenticated and shows both the whole source and
    // what is exposed today, so the choice can be made without the public
    // endpoint ever carrying the hidden columns.
    let operator_stats = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/mapserver/default-mapserver/layers/roads/stats")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operator stats response");
    assert_eq!(operator_stats.status(), StatusCode::OK);
    let operator_stats = response_json(operator_stats).await;
    assert_eq!(operator_stats["allowed_properties"], json!(["name"]));
    let mut operator_columns: Vec<String> = operator_stats["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .map(|column| column["name"].as_str().unwrap_or_default().to_string())
        .collect();
    operator_columns.sort();
    assert_eq!(
        operator_columns,
        vec!["name".to_string(), "owner_phone".to_string()]
    );

    let anonymous_operator_stats = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/mapserver/default-mapserver/layers/roads/stats")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("anonymous operator stats response");
    assert_eq!(
        anonymous_operator_stats.status(),
        StatusCode::UNAUTHORIZED,
        "the whole-source view is not public"
    );

    let _ = fs::remove_dir_all(&data_root);
}

/// The file storage backend is a declaration, not an implicit hardcode.
///
/// Three things have to hold together for that to be true: a project that
/// declares nothing keeps the store it always had, a project that names the
/// local store explicitly behaves identically, and a project that names a store
/// this build cannot open is refused rather than quietly falling back to disk.
#[test]
fn the_file_storage_backend_is_declared_and_an_unknown_one_is_refused() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("files-backend-declaration");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = PlatformService::from_config(config).expect("platform service");

    for project in ["undeclared", "declared"] {
        platform
            .projects
            .create_or_update_project(
                "superadmin",
                &CreateProjectRequest {
                    project: project.to_string(),
                    title: Some(project.to_string()),
                    local_branch: None,
                    runtime: Default::default(),
                },
            )
            .expect("project");
    }

    // A project that declares nothing stores and serves exactly as before.
    let undeclared = platform
        .project_zebfs("superadmin", "undeclared")
        .expect("undeclared project storage");
    undeclared.put("notes/hello.txt", b"hello").expect("put");
    assert_eq!(
        undeclared.get("notes/hello.txt").expect("get").bytes,
        b"hello"
    );
    assert_eq!(
        undeclared.root(),
        data_root
            .join("users")
            .join("superadmin")
            .join("undeclared")
            .join("files")
    );
    let undeclared_yaml = fs::read_to_string(
        data_root
            .join("users")
            .join("superadmin")
            .join("undeclared")
            .join("repo")
            .join("zebflow.yaml"),
    )
    .expect("undeclared configuration");
    assert!(
        !undeclared_yaml.contains("backend"),
        "an absent declaration must stay absent: {undeclared_yaml}"
    );

    // The same project declaring the local store explicitly behaves the same.
    platform
        .zebflow_cfg
        .update("superadmin", "declared", |cfg| {
            cfg.configs.files.backend = Some("zebfs".to_string());
        })
        .expect("declare the local backend");
    let declared = platform
        .project_zebfs("superadmin", "declared")
        .expect("declared project storage");
    declared.put("notes/hello.txt", b"hello").expect("put");
    assert_eq!(
        declared.get("notes/hello.txt").expect("get").bytes,
        b"hello"
    );
    assert_eq!(
        declared.root(),
        data_root
            .join("users")
            .join("superadmin")
            .join("declared")
            .join("files")
    );
    let declared_yaml = fs::read_to_string(
        data_root
            .join("users")
            .join("superadmin")
            .join("declared")
            .join("repo")
            .join("zebflow.yaml"),
    )
    .expect("declared configuration");
    assert!(
        declared_yaml.contains("backend: zebfs"),
        "the declaration must be written: {declared_yaml}"
    );

    // An unrelated settings write does not erase the declaration.
    platform
        .zebflow_cfg
        .set_project_title("superadmin", "declared", "Renamed")
        .expect("unrelated update");
    assert_eq!(
        platform
            .zebflow_cfg
            .read_or_default("superadmin", "declared")
            .expect("reread")
            .configs
            .files
            .backend
            .as_deref(),
        Some("zebfs")
    );

    // A backend this build cannot open is refused by name, listing what is
    // accepted, rather than silently resolving to the local store.
    let config_path = data_root
        .join("users")
        .join("superadmin")
        .join("declared")
        .join("repo")
        .join("zebflow.yaml");
    fs::write(
        &config_path,
        declared_yaml.replace("backend: zebfs", "backend: s3"),
    )
    .expect("declare an unknown backend");
    let refused = platform
        .project_zebfs("superadmin", "declared")
        .expect_err("an unknown backend is refused");
    assert!(
        refused.message.contains("'s3'") && refused.message.contains("accepted: zebfs"),
        "{}",
        refused.message
    );

    let _ = fs::remove_dir_all(&data_root);
}

/// A stranger cannot read another owner's project through preview.
///
/// Preview asked only whether *someone* was signed in. A brand-new member with
/// no standing in `superadmin/default` could therefore turn preview on for a
/// private file and fetch the rendered page, while every other project route
/// refused them — the capability system was sound, these four handlers simply
/// never consulted it.
#[tokio::test]
async fn preview_refuses_a_subject_without_project_capabilities() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("preview-authz");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let owner_cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let create_user = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/users")
                .method("POST")
                .header(header::COOKIE, &owner_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "owner": "mallory", "password": "mallory-pass", "role": "member" })
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create user response");
    assert_eq!(create_user.status(), StatusCode::OK);

    let stranger = login_cookie(app.clone(), "mallory", "mallory-pass").await;

    // Turning preview on decides what is served outward, so it needs write.
    let toggle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/preview/toggle")
                .method("POST")
                .header(header::COOKIE, &stranger)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "file": "sample_web_page.tsx", "active": true }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("toggle response");
    assert_eq!(
        toggle.status(),
        StatusCode::FORBIDDEN,
        "a stranger must not decide what another project previews"
    );

    // Whether preview is on is itself a fact about someone else's project.
    let status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/preview/status?file=sample_web_page.tsx")
                .header(header::COOKIE, &stranger)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("status response");
    assert_eq!(status.status(), StatusCode::FORBIDDEN);

    // And the rendered page is the payload that mattered.
    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/preview/superadmin/default?file=sample_web_page.tsx")
                .header(header::COOKIE, &stranger)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("preview page response");
    assert_eq!(
        page.status(),
        StatusCode::FORBIDDEN,
        "the rendered private page must not be served to a stranger"
    );

    // The owner is unaffected.
    let owner_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/preview/status?file=sample_web_page.tsx")
                .header(header::COOKIE, &owner_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("owner status response");
    assert_eq!(
        owner_status.status(),
        StatusCode::OK,
        "the owner still reads their own preview state"
    );
}

/// One person's whole lifecycle: created, working, entangled with other
/// people's projects — and then deleted, with every refusal checked on the
/// way and no footprint left at the end.
/// docs/contracts/addressing.md §2: the dev host is the Studio's, a named
/// host is the public's. On `research.test` the platform API and `/_mcp` are
/// not there at all until the project switches `mcp` on; the dev host keeps
/// serving them so the Studio's preview and an agent's session keep working.
#[tokio::test]
async fn a_named_host_serves_only_the_site_until_mcp_is_switched_on() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("addressing-named-host-api");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let put_hosts = |disabled: serde_json::Value| {
        Request::builder()
            .uri("/api/projects/superadmin/default/settings/addressing")
            .method("PUT")
            .header(header::COOKIE, &cookie)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "data": { "hosts": ["research.test"], "routes": [], "disabled": disabled } }).to_string()))
            .expect("request")
    };
    let status = |host: &'static str, path: &'static str, with_cookie: bool| {
        let mut b = Request::builder().uri(path).header(header::HOST, host);
        if with_cookie {
            b = b.header(header::COOKIE, &cookie);
        }
        b.body(Body::empty()).expect("request")
    };

    let saved = app.clone().oneshot(put_hosts(json!(["fs", "ms", "mcp"]))).await.expect("addressing write");
    assert_eq!(saved.status(), StatusCode::OK);

    // Named host, api_on_hosts off (the default): the site is there, the API and the agent endpoint are not.
    let site = app.clone().oneshot(status("research.test", "/", false)).await.expect("site");
    assert_eq!(
        site.headers().get("x-zebflow-project").and_then(|v| v.to_str().ok()),
        Some("superadmin/default"),
        "the host resolved to the project (the default project has no home page, so the status is 404)"
    );
    let api = app.clone().oneshot(status("research.test", "/api/projects/superadmin/default/pipelines", true)).await.expect("api");
    assert_eq!(api.status(), StatusCode::NOT_FOUND, "the platform API is not on a public host by default, even with a session");
    let mcp = app.clone().oneshot(status("research.test", "/_mcp", false)).await.expect("mcp");
    assert_eq!(mcp.status(), StatusCode::NOT_FOUND, "/_mcp is off by default on a public host");

    // There is no dev mode: the dev host obeys the same switch, and the platform address always serves the API.
    let dev = app.clone().oneshot(status("default.superadmin.localhost", "/api/projects/superadmin/default/pipelines", true)).await.expect("dev");
    assert_eq!(dev.status(), StatusCode::NOT_FOUND, "the dev host is a project host like any other");
    let platform = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/pipelines").header(header::COOKIE, &cookie).body(Body::empty()).expect("request")).await.expect("platform");
    assert_eq!(platform.status(), StatusCode::OK);

    // Switch api_on_hosts and mcp on: the named host now serves the API (still behind auth) and the agent endpoint.
    let saved = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/settings/addressing").method("PUT")
        .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "data": { "hosts": ["research.test"], "routes": [], "disabled": ["fs", "ms"], "api_on_hosts": true } }).to_string())).expect("request")).await.expect("addressing write");
    assert_eq!(saved.status(), StatusCode::OK);
    let api_anon = app.clone().oneshot(status("research.test", "/api/projects/superadmin/default/pipelines", false)).await.expect("api anon");
    assert_eq!(api_anon.status(), StatusCode::UNAUTHORIZED, "reachable now, and still authenticated");
    let api = app.clone().oneshot(status("research.test", "/api/projects/superadmin/default/pipelines", true)).await.expect("api");
    assert_eq!(api.status(), StatusCode::OK);
    let mcp = app.clone().oneshot(status("research.test", "/_mcp", false)).await.expect("mcp");
    assert_ne!(mcp.status(), StatusCode::NOT_FOUND, "/_mcp answers (and refuses without a bearer) once switched on");
}

/// docs/contracts/project.md "Git": a conflict is a state the Studio resolves,
/// not "resolve locally". Against a bare repository on disk: the project
/// commits and pushes; a second clone pushes a conflicting change; the
/// project's next commit stays local, `sync` answers 409 with the file, the
/// rebase is kept, `resolve mine` + `continue` finish it, `push` lands both
/// histories, and status reports `clean` throughout with honest counts.
#[tokio::test]
async fn a_git_conflict_is_resolved_in_the_studio_not_locally() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("git-sync-conflict");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let git = |dir: &std::path::Path, args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(["-c", "user.name=Other", "-c", "user.email=other@example.test"])
            .arg("-C").arg(dir).args(args).output().expect("git runs");
        assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    let bare = data_root.join("remote.git");
    git(&data_root, &["init", "--bare", "--initial-branch=main", bare.to_str().unwrap()]);
    let remote_url = format!("file://{}", bare.display());

    let post = |path: &'static str, body: serde_json::Value| {
        let app = app.clone(); let cookie = cookie.clone();
        async move {
            app.oneshot(Request::builder().uri(format!("/api/projects/superadmin/default/git/{path}")).method("POST")
                .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())).expect("request")).await.expect("response")
        }
    };
    let write = |path: &'static str, body: &'static str| {
        let app = app.clone(); let cookie = cookie.clone();
        async move {
            app.oneshot(Request::builder().uri(format!("/api/projects/superadmin/default/repo/file?path={path}")).method("PUT")
                .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(body)).expect("request")).await.expect("response")
        }
    };
    let status = || {
        let app = app.clone(); let cookie = cookie.clone();
        async move {
            let r = app.oneshot(Request::builder().uri("/api/projects/superadmin/default/git/status")
                .header(header::COOKIE, &cookie).body(Body::empty()).expect("request")).await.expect("response");
            response_json(r).await
        }
    };

    // Connect the remote, commit and push the first version.
    let saved = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/git/remote").method("PUT")
        .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "credential_id": "", "repo_url": remote_url, "branch": "main" }).to_string())).expect("request")).await.expect("remote");
    let saved_status = saved.status();
    assert_eq!(saved_status, StatusCode::OK, "{}", response_json(saved).await);
    assert_eq!(write("docs/notes.md", "line one\n").await.status(), StatusCode::OK);
    let first = post("commit", json!({ "files": ["docs/notes.md"], "message": "first", "push": true })).await;
    assert_eq!(first.status(), StatusCode::OK, "{}", response_json(first).await);
    let st = status().await;
    assert_eq!(st["sync_word"], "clean", "{st}");

    // Someone else pushes a conflicting change to the same line.
    let other = data_root.join("other");
    git(&data_root, &["clone", &remote_url, other.to_str().unwrap()]);
    fs::write(other.join("docs/notes.md"), "line one, edited elsewhere\n").expect("write");
    git(&other, &["commit", "-am", "elsewhere"]);
    git(&other, &["push", "origin", "main"]);

    // The project commits its own version. The commit is local and fine.
    assert_eq!(write("docs/notes.md", "line one, edited here\n").await.status(), StatusCode::OK);
    let local = post("commit", json!({ "files": ["docs/notes.md"], "message": "here", "push": false })).await;
    let local_status = local.status();
    assert_eq!(local_status, StatusCode::OK, "{}", response_json(local).await);

    // A fetch shows the truth: one to push, one to pull.
    let fetched = post("fetch", json!({})).await;
    assert_eq!(fetched.status(), StatusCode::OK);
    let st = status().await;
    assert_eq!(st["sync"]["ahead"], json!(1), "{st}");
    assert_eq!(st["sync"]["behind"], json!(1), "{st}");
    assert_eq!(st["sync_word"], "diverged");

    // Push refuses while behind; sync stops at the conflict and keeps the rebase.
    let pushed = post("push", json!({})).await;
    assert_eq!(pushed.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(pushed).await["code"], "GIT_BEHIND");
    let synced = post("sync", json!({})).await;
    assert_eq!(synced.status(), StatusCode::CONFLICT);
    let body = response_json(synced).await;
    assert_eq!(body["outcome"], "conflict");
    assert_eq!(body["state"]["conflicts"], json!(["docs/notes.md"]), "{body}");
    let st = status().await;
    assert_eq!(st["sync_word"], "conflict", "the rebase is a state, not an aborted attempt: {st}");

    // Continue is refused while the file is unresolved; resolve keeps the project's version.
    let early = post("continue", json!({})).await;
    assert_eq!(early.status(), StatusCode::CONFLICT);
    let resolved = post("resolve", json!({ "path": "docs/notes.md", "resolution": "mine" })).await;
    assert_eq!(resolved.status(), StatusCode::OK, "{}", response_json(resolved).await);
    let repo_file = data_root.join("users/superadmin/default/repo/docs/notes.md");
    assert_eq!(fs::read_to_string(&repo_file).unwrap(), "line one, edited here\n", "mine means what the project had");
    let done = post("continue", json!({})).await;
    assert_eq!(done.status(), StatusCode::OK, "{}", response_json(done).await);
    let st = status().await;
    assert_eq!(st["sync_word"], "unpushed", "{st}");
    assert_eq!(st["sync"]["ahead"], json!(1));
    assert_eq!(st["sync"]["behind"], json!(0));

    // Push lands it; the remote now carries both commits in order.
    let pushed = post("push", json!({})).await;
    assert_eq!(pushed.status(), StatusCode::OK, "{}", response_json(pushed).await);
    assert_eq!(status().await["sync_word"], "clean");
    let log = git(&bare, &["log", "--format=%s", "main"]);
    assert_eq!(log.lines().collect::<Vec<_>>(), vec!["here", "elsewhere", "first"]);

    // Taking the remote's side works the same way, and abort puts everything back.
    git(&other, &["pull", "--rebase", "origin", "main"]);
    fs::write(other.join("docs/notes.md"), "theirs wins\n").expect("write");
    git(&other, &["commit", "-am", "elsewhere again"]);
    git(&other, &["push", "origin", "main"]);
    assert_eq!(write("docs/notes.md", "mine again\n").await.status(), StatusCode::OK);
    assert_eq!(post("commit", json!({ "files": ["docs/notes.md"], "message": "here again", "push": false })).await.status(), StatusCode::OK);
    assert_eq!(post("sync", json!({})).await.status(), StatusCode::CONFLICT);
    let aborted = post("abort", json!({})).await;
    assert_eq!(aborted.status(), StatusCode::OK);
    assert_eq!(fs::read_to_string(&repo_file).unwrap(), "mine again\n", "abort restores the project's commit untouched");
    assert_eq!(post("sync", json!({})).await.status(), StatusCode::CONFLICT);
    assert_eq!(post("resolve", json!({ "path": "docs/notes.md", "resolution": "theirs" })).await.status(), StatusCode::OK);
    assert_eq!(fs::read_to_string(&repo_file).unwrap(), "theirs wins\n", "theirs means the remote's version");
    assert_eq!(post("continue", json!({})).await.status(), StatusCode::OK);
    assert_eq!(post("push", json!({})).await.status(), StatusCode::OK);
    assert_eq!(status().await["sync_word"], "clean");
}

/// `addressing.md` §2a and `kinds/invocation-record`: an uncaught failure is a
/// 500 whatever the switch says; `hidden` shows the reference only, `shown` or
/// a route's `--errors show` adds the failure; every response carries the run
/// id as `X-Request-Id`, an inbound one in canonical form is adopted; the same
/// failure three times is one error group with a count of three, found by the
/// reference a visitor quotes.
#[tokio::test]
async fn an_uncaught_failure_hides_by_default_and_is_one_error_group() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("errors-switch-and-groups");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    for dsl in [
        r#"register pipelines/tests/boom -- | trigger.webhook --path /boom --method GET | script -- "throw new Error('column \"x\" does not exist at row ' + (input.query.n || 0))" | web.response"#,
        "activate pipeline pipelines/tests/boom.zf.json",
        r#"register pipelines/tests/boom-shown -- | trigger.webhook --path /boom-shown --method GET --errors show | script -- "throw new Error('shown on purpose')" | web.response"#,
        "activate pipeline pipelines/tests/boom-shown.zf.json",
    ] {
        let r = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/pipelines/dsl").method("POST")
            .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "dsl": dsl }).to_string())).expect("request")).await.expect("dsl");
        let j = response_json(r).await;
        assert_eq!(j["ok"], json!(true), "{j}");
    }
    let call = |path: &'static str, accept: &'static str, inbound: Option<&'static str>| {
        let app = app.clone();
        async move {
            let mut b = Request::builder().uri(format!("/wh/superadmin/default{path}")).header(header::ACCEPT, accept);
            if let Some(id) = inbound { b = b.header("x-request-id", id); }
            app.oneshot(b.body(Body::empty()).expect("request")).await.expect("response")
        }
    };
    let is_hex32 = |s: &str| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());

    // hidden (the default): a browser gets the neutral page with a reference and no internals
    let r = call("/boom?n=1", "text/html,*/*", None).await;
    assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let rid = r.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    assert!(is_hex32(&rid), "run id on the response: {rid:?}");
    let html = response_text(r).await;
    assert!(html.contains("Reference") && html.contains(&rid[..8]), "{html}");
    assert!(!html.contains("does not exist"), "hidden must not leak the message: {html}");
    // hidden, JSON request: code internal + the reference, nothing else
    let r = call("/boom?n=2", "application/json", None).await;
    assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let j = response_json(r).await;
    assert_eq!(j["error"]["code"], "internal");
    assert!(j["error"].get("message").is_none(), "{j}");
    // an inbound id in canonical form is adopted
    let mine = "0123456789abcdef0123456789abcdef";
    let r = call("/boom?n=3", "application/json", Some(mine)).await;
    assert_eq!(r.headers().get("x-request-id").and_then(|v| v.to_str().ok()), Some(mine));
    let j = response_json(r).await;
    assert_eq!(j["error"]["request_id"], mine);
    // a route that says --errors show reveals the failure whatever the project says
    let r = call("/boom-shown", "application/json", None).await;
    assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let j = response_json(r).await;
    assert!(j["error"]["message"].as_str().unwrap_or("").contains("shown on purpose"), "{j}");
    assert!(j["error"]["run_url"].as_str().unwrap_or("").contains("boom-shown"), "{j}");

    // three occurrences, two different row numbers: one group, count three, the quoted reference finds it
    let r = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/pipelines/errors").header(header::COOKIE, &cookie).body(Body::empty()).expect("request")).await.expect("errors");
    let groups = response_json(r).await;
    let boom: Vec<&Value> = groups["groups"].as_array().unwrap().iter().filter(|g| g["file_rel_path"] == "pipelines/tests/boom.zf.json").collect();
    assert_eq!(boom.len(), 1, "one group for the same error with different numbers: {groups}");
    assert_eq!(boom[0]["count"], json!(3));
    assert_eq!(boom[0]["occurrences"].as_array().unwrap().len(), 3);
    assert!(boom[0]["message_pattern"].as_str().unwrap().contains("row #"), "{}", boom[0]["message_pattern"]);
    assert_eq!(boom[0]["latest"]["run_id"], mine);
    let r = app.clone().oneshot(Request::builder().uri(format!("/api/projects/superadmin/default/pipelines/errors?run_id={}", &rid[..8])).header(header::COOKIE, &cookie).body(Body::empty()).expect("request")).await.expect("lookup");
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(response_json(r).await["group"]["count"], json!(3));

    // the project switch: shown, and the same route now reveals the failure
    let r = app.clone().oneshot(Request::builder().uri("/api/projects/superadmin/default/settings/addressing").method("PUT")
        .header(header::COOKIE, &cookie).header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "data": { "hosts": [], "routes": [], "disabled": ["fs", "ms", "mcp"], "errors": "shown" } }).to_string())).expect("request")).await.expect("addressing");
    assert_eq!(r.status(), StatusCode::OK);
    let r = call("/boom?n=4", "application/json", None).await;
    assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR, "the status never changes with the switch");
    let j = response_json(r).await;
    assert!(j["error"]["message"].as_str().unwrap_or("").contains("does not exist"), "{j}");
    assert_eq!(j["error"]["node_id"], "n1");
}

#[tokio::test]
async fn a_deleted_user_leaves_no_footprint_and_takes_no_hostages() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("user-delete-lifecycle");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let app = build_router(config).await.expect("platform router");

    async fn send(
        app: &axum::Router,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().uri(uri).method(method);
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        let request = match body {
            Some(body) => builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => builder.body(Body::empty()),
        }
        .expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, json)
    }

    let admin = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // Two people join the instance through the roster, at its instance-scope
    // address.
    for owner in ["sari", "dana"] {
        let (status, _) = send(
            &app,
            "POST",
            "/api/platform/users",
            Some(&admin),
            Some(json!({"owner": owner, "password": format!("{owner}-pass"), "role": "member"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "creating {owner}");
    }
    let (status, roster) = send(&app, "GET", "/api/platform/users", Some(&admin), None).await;
    assert_eq!(status, StatusCode::OK);
    let names = |v: &Value| -> Vec<String> {
        v["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|u| u["owner"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    assert!(names(&roster).contains(&"sari".to_string()));

    // Sari builds something of her own and joins something of superadmin's.
    let sari = login_cookie(app.clone(), "sari", "sari-pass").await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/users/sari/projects",
        Some(&sari),
        Some(json!({"project": "shop"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "sari creates shop");

    let (status, invite) = send(
        &app,
        "POST",
        "/api/projects/superadmin/default/invites",
        Some(&admin),
        Some(json!({"target_user": "sari", "role_preset": "reporter"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "inviting sari: {invite}");
    let invite_id = invite["invite"]["invite_id"].as_str().expect("invite id");
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/invites/superadmin/default/{invite_id}/accept"),
        Some(&sari),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "sari accepts");

    // The refusals, each a distinct answer.
    let (status, _) = send(
        &app,
        "DELETE",
        "/api/platform/users/superadmin",
        Some(&admin),
        Some(json!({"username": "superadmin", "password": "test-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "self-deletion is refused");

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/platform/users/sari",
        Some(&admin),
        Some(json!({"username": "wrong-name", "password": "test-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "typed-back name must match");

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/platform/users/sari",
        Some(&admin),
        Some(json!({"username": "sari", "password": "wrong-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "caller's password is re-verified");

    let (status, _) = send(
        &app,
        "DELETE",
        "/api/platform/users/dana",
        Some(&sari),
        Some(json!({"username": "dana", "password": "sari-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "only a superadmin deletes users");

    // Dana joins sari's shop, so purging sari would take a project someone
    // else works in. That is refused: deletion is never the path by which
    // another person's work disappears.
    let (status, invite) = send(
        &app,
        "POST",
        "/api/projects/sari/shop/invites",
        Some(&sari),
        Some(json!({"target_user": "dana", "role_preset": "reporter"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "sari invites dana: {invite}");
    let invite_id = invite["invite"]["invite_id"].as_str().expect("invite id");
    let dana = login_cookie(app.clone(), "dana", "dana-pass").await;
    let (status, _) = send(
        &app,
        "POST",
        &format!("/api/invites/sari/shop/{invite_id}/accept"),
        Some(&dana),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "dana accepts");

    let (status, refusal) = send(
        &app,
        "DELETE",
        "/api/platform/users/sari",
        Some(&admin),
        Some(json!({"username": "sari", "password": "test-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "purge with other members: {refusal}");

    // With a successor, the same request succeeds: the shop transfers whole.
    let (status, done) = send(
        &app,
        "DELETE",
        "/api/platform/users/sari",
        Some(&admin),
        Some(json!({"username": "sari", "password": "test-pass", "successor": "dana"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "transfer-delete: {done}");
    assert_eq!(done["resolution"], "transferred");

    // No roster entry, no login, no live session, no membership, no files.
    let (_, roster) = send(&app, "GET", "/api/platform/users", Some(&admin), None).await;
    assert!(!names(&roster).contains(&"sari".to_string()), "roster still lists sari");

    let dead_login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=sari&password=sari-pass"))
                .expect("request"),
        )
        .await
        .expect("login response");
    assert_ne!(dead_login.status(), StatusCode::SEE_OTHER, "sari can still log in");

    let (status, _) = send(&app, "GET", "/api/profile", Some(&sari), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "sari's old session still answers");

    let (_, members) = send(
        &app,
        "GET",
        "/api/projects/superadmin/default/members",
        Some(&admin),
        None,
    )
    .await;
    let member_names: Vec<String> = members["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|m| m["user_id"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(!member_names.iter().any(|m| m == "sari"), "sari's membership survived");

    let (_, danas) = send(&app, "GET", "/api/users/dana/projects", Some(&admin), None).await;
    let dana_projects: Vec<String> = danas["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|p| p["project"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(dana_projects.iter().any(|p| p == "shop"), "shop did not reach dana: {danas}");
    assert!(
        data_root.join("users").join("dana").join("shop").exists(),
        "shop's files did not move"
    );
    assert!(
        !data_root.join("users").join("sari").join("shop").exists(),
        "sari's shop directory survived"
    );

    // And the purge path, for someone whose work entangles nobody.
    let (status, _) = send(
        &app,
        "POST",
        "/api/platform/users",
        Some(&admin),
        Some(json!({"owner": "mallory", "password": "mallory-pass", "role": "member"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mallory = login_cookie(app.clone(), "mallory", "mallory-pass").await;
    let (status, _) = send(
        &app,
        "POST",
        "/api/users/mallory/projects",
        Some(&mallory),
        Some(json!({"project": "junk"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, done) = send(
        &app,
        "DELETE",
        "/api/platform/users/mallory",
        Some(&admin),
        Some(json!({"username": "mallory", "password": "test-pass"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "purge-delete: {done}");
    assert_eq!(done["resolution"], "purged");
    assert!(
        !data_root.join("users").join("mallory").exists(),
        "mallory's directory survived the purge"
    );
    let (_, roster) = send(&app, "GET", "/api/platform/users", Some(&admin), None).await;
    assert!(!names(&roster).contains(&"mallory".to_string()));
}

/// The instance scope is one prefix, and it never answers an anonymous caller.
///
/// `/api/platform/*` is where instance resources live — the roster, the db
/// console, the credential keyring, the cluster, the office identity, the hub
/// service. The old spellings (`/api/admin`, `/api/cluster`, `/api/office`)
/// are gone, not aliased: a route that exists under two names is a guard that
/// has to be checked twice. The one deliberate exception is
/// `POST /api/office/vouch`, called by a browser that has no session yet.
#[tokio::test]
async fn instance_scope_is_one_prefix_and_never_answers_an_anonymous_caller() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("instance-scope-prefix");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");

    async fn status_of(app: &axum::Router, method: &str, uri: &str) -> StatusCode {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .method(method)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        response.status()
    }

    // The old spellings no longer exist.
    for (method, uri) in [
        ("GET", "/api/users"),
        ("GET", "/api/admin/db/collections"),
        ("POST", "/api/admin/db/query"),
        ("GET", "/api/admin/credentials/keyring"),
        ("GET", "/api/cluster/workers"),
        ("GET", "/api/cluster/join-tokens"),
        ("GET", "/api/cluster/office-break-glass"),
        ("GET", "/api/office/identity-writes"),
        ("GET", "/api/office/local-authority"),
    ] {
        assert_eq!(
            status_of(&app, method, uri).await,
            StatusCode::NOT_FOUND,
            "{method} {uri} should be gone, not aliased"
        );
    }

    // The new spellings exist and refuse a caller with no session. 401, not
    // 404: the resource is real, the caller is nobody.
    for (method, uri) in [
        ("GET", "/api/platform/users"),
        ("GET", "/api/platform/db/collections"),
        ("GET", "/api/platform/credentials/keyring"),
        ("GET", "/api/platform/cluster/workers"),
        ("GET", "/api/platform/cluster/join-tokens"),
        ("GET", "/api/platform/cluster/office-break-glass"),
        ("GET", "/api/platform/office/identity-writes"),
        ("GET", "/api/platform/office/local-authority"),
        ("GET", "/api/platform/hub/service"),
        ("GET", "/api/platform/hub/assets"),
    ] {
        assert_eq!(
            status_of(&app, method, uri).await,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} answered an anonymous caller"
        );
    }
}

/// `--auth-optional`: a public page that knows who is signed in. The same
/// route answers a guest with `input.auth` null and a member with the claims;
/// an expired or foreign token is a guest, never a 401.
#[tokio::test]
async fn an_auth_optional_webhook_answers_guests_and_reads_a_valid_token() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("auth-optional");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let cred = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/credentials")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "credential_id": "auth_member",
                        "title": "Member sessions",
                        "kind": "jwt_signing_key",
                        "notes": "",
                        "secret": { "algorithm": "HS256", "secret": "test-secret-at-least-32-bytes-long!!", "cookie_name": "site_member", "auth_redirect": "/login" }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("credential response");
    assert_eq!(cred.status(), StatusCode::OK);

    for dsl in [
        r#"register pipelines/tests/whoami -- | trigger.webhook --path /whoami --method GET --auth-type jwt --auth-credential auth_member --auth-optional | script -- "return { who: input.auth ? input.auth.sub : 'guest' }" | web.response"#,
        "activate pipeline pipelines/tests/whoami.zf.json",
    ] {
        let dsl_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projects/superadmin/default/pipelines/dsl")
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "dsl": dsl }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("dsl response");
        let dsl_json = response_json(dsl_response).await;
        assert_eq!(dsl_json["ok"], json!(true), "dsl output: {dsl_json}");
    }

    let ask = |cookie_header: Option<String>| {
        let app = app.clone();
        async move {
            let mut req = Request::builder()
                .uri("/wh/superadmin/default/whoami")
                .method("GET");
            if let Some(c) = cookie_header {
                req = req.header(header::COOKIE, c);
            }
            let response = app
                .oneshot(req.body(Body::empty()).expect("request"))
                .await
                .expect("response");
            let status = response.status();
            (status, response_json(response).await)
        }
    };

    let (status, body) = ask(None).await;
    assert_eq!(status, StatusCode::OK, "a guest is answered: {body}");
    assert_eq!(body["who"], json!("guest"));

    use jsonwebtoken::{EncodingKey, Header, encode};
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    let key = EncodingKey::from_secret(b"test-secret-at-least-32-bytes-long!!");
    let valid = encode(&Header::default(), &json!({ "sub": "m_1", "exp": now + 600 }), &key).expect("jwt");
    let (status, body) = ask(Some(format!("site_member={valid}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["who"], json!("m_1"), "a valid token fills input.auth: {body}");

    let expired = encode(&Header::default(), &json!({ "sub": "m_1", "exp": now - 600 }), &key).expect("jwt");
    let (status, body) = ask(Some(format!("site_member={expired}"))).await;
    assert_eq!(status, StatusCode::OK, "an expired token is a guest, not a 401: {body}");
    assert_eq!(body["who"], json!("guest"));

    let foreign = encode(&Header::default(), &json!({ "sub": "p_1", "exp": now + 600 }), &EncodingKey::from_secret(b"another-key-entirely-for-participants"))
        .expect("jwt");
    let (status, body) = ask(Some(format!("site_member={foreign}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["who"], json!("guest"), "a token under another key is a guest: {body}");

    // Without --auth-optional the same credential refuses a guest — and a
    // browser navigation is sent to the credential's auth_redirect carrying
    // the page it wanted, so the sign-in can bring the visitor back.
    for dsl in [
        r#"register pipelines/tests/mine -- | trigger.webhook --path /mine/:slug --method GET --auth-type jwt --auth-credential auth_member | web.response --message ok"#,
        "activate pipeline pipelines/tests/mine.zf.json",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projects/superadmin/default/pipelines/dsl")
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "dsl": dsl }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("dsl response");
        assert_eq!(response_json(response).await["ok"], json!(true));
    }
    let nav = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/wh/superadmin/default/mine/abc?tab=2")
                .method("GET")
                .header(header::ACCEPT, "text/html,application/xhtml+xml")
                .header("sec-fetch-mode", "navigate")
                .header("sec-fetch-dest", "document")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(nav.status(), StatusCode::SEE_OTHER, "a navigation is redirected to sign in");
    assert_eq!(
        nav.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()),
        Some("/login?next=%2Fmine%2Fabc%3Ftab%3D2")
    );
}

/// A designed 404: `trigger.weberror --code 404 | web.response --template`
/// answers an unknown route with the page, not the JSON fallback. The two
/// halves this exercises: the DSL types `404` as a number and the node must
/// take it; the weberror dispatcher must load the template's markup before
/// running the graph, as the webhook path does.
#[tokio::test]
async fn a_weberror_template_page_answers_an_unknown_route() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("weberror-template");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let page = r#"export default function NotFound(input) { return <main><h1>Nothing here</h1><p id="path">{input.path}</p></main>; }
export function getPage() { return { head: { title: "Not found" } }; }"#;
    let put = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/repo/file?path=not-found.tsx")
                .method("PUT")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(page))
                .expect("request"),
        )
        .await
        .expect("put");
    assert_eq!(put.status(), StatusCode::OK);
    for dsl in [
        "register pipelines/not-found -- | trigger.weberror --code 404 | web.response --status 404 --template not-found.tsx",
        "activate pipeline pipelines/not-found.zf.json",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/projects/superadmin/default/pipelines/dsl")
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "dsl": dsl }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("dsl response");
        let body = response_json(response).await;
        assert_eq!(body["ok"], json!(true), "{body}");
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/wh/superadmin/default/no/such/page")
                .method("GET")
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let html = response_text(response).await;
    assert!(html.contains("<h1>Nothing here</h1>"), "the designed page, not the JSON fallback: {html}");
    assert!(html.contains(r#"<p id="path">/no/such/page</p>"#), "{html}");
    assert!(!html.contains("RWE component error"), "{html}");
}
