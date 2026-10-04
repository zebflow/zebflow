use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::infra::io::state::MemStateBus;

const O: &str = "site-a";
const P: &str = "demo";
const ROUTE: &str = "/library";

fn service() -> (tempfile::TempDir, PublishedOAuthService) {
    let dir = tempfile::tempdir().expect("temp dir");
    let bus = Arc::new(MemStateBus::new_with_durable(dir.path().to_path_buf()));
    (dir, PublishedOAuthService::new(bus))
}

fn code(family: &str) -> Code {
    Code {
        client_id: "zfc_aaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        redirect_uri: "https://client.example/cb".into(),
        challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
        resource: "https://library.example/_mcp/library".into(),
        issuer: "https://library.example/_mcp/library".into(),
        scope: "mcp".into(),
        claims: json!({ "sub": "reader-1" }),
        family: family.into(),
    }
}

fn refresh(family: &str) -> Refresh {
    Refresh {
        family: family.into(),
        client_id: "zfc_aaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        resource: "https://library.example/_mcp/library".into(),
        issuer: "https://library.example/_mcp/library".into(),
        scope: "mcp".into(),
        claims: json!({ "sub": "reader-1" }),
        family_expires_at: now_unix() + 3600,
    }
}

#[test]
fn a_code_is_spent_once_and_its_replay_revokes_the_family() {
    let (_dir, oauth) = service();
    let secret = oauth.issue_code(O, P, ROUTE, &code("fam-1")).unwrap();
    assert_eq!(oauth.take_code(O, P, ROUTE, &secret).unwrap(), Spent::Fresh(code("fam-1")));
    assert!(!oauth.family_revoked(O, P, ROUTE, "fam-1").unwrap());
    assert_eq!(oauth.take_code(O, P, ROUTE, &secret).unwrap(), Spent::Refused);
    assert!(oauth.family_revoked(O, P, ROUTE, "fam-1").unwrap(), "a replayed code revokes what it was exchanged for");
    assert_eq!(oauth.take_code(O, P, "/other", &oauth.issue_code(O, P, ROUTE, &code("fam-2")).unwrap()).unwrap(), Spent::Refused, "a code belongs to its route");
}

#[test]
fn a_refresh_token_rotates_and_a_replay_kills_the_family() {
    let (_dir, oauth) = service();
    let r1 = oauth.issue_refresh(O, P, ROUTE, &refresh("fam-r")).unwrap();
    let Spent::Fresh(first) = oauth.rotate_refresh(O, P, ROUTE, &r1).unwrap() else { panic!("r1 is live") };
    let r2 = oauth.issue_refresh(O, P, ROUTE, &first).unwrap();
    assert_eq!(oauth.rotate_refresh(O, P, ROUTE, &r1).unwrap(), Spent::Refused, "r1 is spent");
    assert_eq!(oauth.rotate_refresh(O, P, ROUTE, &r2).unwrap(), Spent::Refused, "and its replay revoked r2's family");
}

#[test]
fn a_ticket_is_spent_once_and_the_route_has_a_cap() {
    let (_dir, oauth) = service();
    let ticket = Ticket {
        route: ROUTE.into(),
        client_id: "zfc_aaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        client_name: "Reader app".into(),
        redirect_uri: "https://client.example/cb".into(),
        redirect_host: "client.example".into(),
        challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into(),
        resource: "r".into(),
        issuer: "r".into(),
        scope: "mcp".into(),
        state: Some("s1".into()),
        credential: "library-jwt".into(),
        created_at: now_unix(),
    };
    let secret = oauth.open_ticket(O, P, &ticket).unwrap().expect("under the cap");
    assert_eq!(oauth.take_ticket(O, P, &secret).unwrap(), Some(ticket.clone()));
    assert_eq!(oauth.take_ticket(O, P, &secret).unwrap(), None);
    for _ in 1..MAX_TICKETS_PER_MINUTE {
        oauth.open_ticket(O, P, &ticket).unwrap();
    }
    // The minute may turn over mid-loop; a second try then hits the new cap.
    let over = oauth.open_ticket(O, P, &ticket).unwrap().is_none() || {
        for _ in 0..MAX_TICKETS_PER_MINUTE {
            oauth.open_ticket(O, P, &ticket).unwrap();
        }
        oauth.open_ticket(O, P, &ticket).unwrap().is_none()
    };
    assert!(over, "the cap refuses");
}

#[test]
fn only_issued_client_ids_are_looked_up() {
    let (_dir, oauth) = service();
    let client = Client {
        client_id: rules::new_client_id(),
        client_name: "Reader app".into(),
        redirect_uris: vec!["https://client.example/cb".into()],
        grant_types: vec!["authorization_code".into()],
        created_at: now_unix(),
    };
    assert!(oauth.register_client(O, P, ROUTE, &client).unwrap());
    assert_eq!(oauth.client(O, P, ROUTE, &client.client_id).unwrap(), Some(client.clone()));
    assert_eq!(oauth.client(O, P, "/other", &client.client_id).unwrap(), None);
    assert_eq!(oauth.client(O, P, ROUTE, "../ticket").unwrap(), None);
}

#[test]
fn an_expired_code_or_ticket_is_refused() {
    let (_dir, oauth) = service();
    let code_secret = rules::new_secret();
    oauth.put(O, P, &key(ROUTE, &format!("code/{}", rules::sha256_hex(&code_secret))), &code("fam-x"), 1).unwrap();
    let ticket_secret = rules::new_secret();
    oauth.put(O, P, &format!("zf.oauth/ticket/{}", rules::sha256_hex(&ticket_secret)), &json!({ "route": ROUTE }), 1).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2100));
    assert_eq!(oauth.take_code(O, P, ROUTE, &code_secret).unwrap(), Spent::Refused);
    assert_eq!(oauth.take_ticket(O, P, &ticket_secret).unwrap(), None);
}
