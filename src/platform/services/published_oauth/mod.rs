//! The state of a published route's OAuth (`published-mcp.md` § `--auth
//! oauth`): registered clients, tickets, authorization codes and refresh
//! token families, kept in the project's durable store
//! (`data/store/kv.db`, through the state bus) so they travel with the
//! project. The `kv.*` nodes refuse every key under `zf.`.
//!
//! Every secret is stored as its SHA-256 (the key is the hash): the store
//! never holds a ticket, code or refresh token a reader could replay. A
//! secret is spent by deleting its record — of two racers only the one whose
//! delete removed the row wins. Counts are capped per route by
//! [`MAX_REGISTRATIONS_PER_DAY`] and [`MAX_TICKETS_PER_MINUTE`].
//!
//! The one in-memory part is the cache of client ID metadata documents.

pub mod rules;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::infra::io::state::DynStateBus;

/// Clients one route may register in a day (RFC 7591).
pub const MAX_REGISTRATIONS_PER_DAY: i64 = 200;
/// Authorizations one route may start in a minute.
pub const MAX_TICKETS_PER_MINUTE: i64 = 120;
/// How long a person has to sign in after authorize.
pub const TICKET_TTL: u64 = 10 * 60;
/// How long an authorization code may wait to be exchanged.
pub const CODE_TTL: u64 = 2 * 60;
/// How long a spent code is remembered, to catch its replay.
pub const USED_CODE_TTL: u64 = 10 * 60;
/// An access token's life.
pub const ACCESS_TTL: u64 = 60 * 60;
/// A refresh family's absolute life, rotation or not.
pub const REFRESH_FAMILY_TTL: u64 = 30 * 24 * 60 * 60;
/// A registered client unused this long is forgotten.
pub const CLIENT_TTL: u64 = 90 * 24 * 60 * 60;
/// How long a revoked family stays revoked.
const REVOKED_FAMILY_TTL: u64 = 30 * 24 * 60 * 60;
/// Client ID metadata documents cached before an old one is swept.
const CIMD_CACHE_MAX: usize = 1_000;

/// A client registered through `/_oauth/register`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub created_at: i64,
}

/// An authorization waiting for the app's login (`auth.oauth.approve`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ticket {
    pub route: String,
    pub client_id: String,
    pub client_name: String,
    pub redirect_uri: String,
    pub redirect_host: String,
    pub challenge: String,
    pub resource: String,
    pub issuer: String,
    pub scope: String,
    pub state: Option<String>,
    /// The route's `--credential`: verifies the app's token at approve.
    pub credential: String,
    pub created_at: i64,
}

/// An approved authorization, waiting for the client's token request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Code {
    pub client_id: String,
    pub redirect_uri: String,
    pub challenge: String,
    pub resource: String,
    pub issuer: String,
    pub scope: String,
    /// The approved app token's claims: who signed in.
    pub claims: Value,
    pub family: String,
}

/// A live refresh token; rotating it spends it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Refresh {
    pub family: String,
    pub client_id: String,
    pub resource: String,
    pub issuer: String,
    pub scope: String,
    pub claims: Value,
    pub family_expires_at: i64,
}

/// A client ID metadata document, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct CimdClient {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

/// The store could not be reached: the caller answers "try again".
#[derive(Debug, Clone)]
pub struct StoreError(pub String);

/// What spending a secret found.
#[derive(Debug, Clone, PartialEq)]
pub enum Spent<T> {
    /// Live and now spent: this caller won.
    Fresh(T),
    /// Spent before (its family is now revoked), unknown, expired or revoked.
    Refused,
}

pub struct PublishedOAuthService {
    bus: DynStateBus,
    cimd: Mutex<HashMap<String, (Instant, Duration, CimdClient)>>,
}

pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

fn key(route: &str, rest: &str) -> String {
    format!("zf.oauth/{}/{rest}", rules::route_hash(route))
}

impl PublishedOAuthService {
    pub fn new(bus: DynStateBus) -> Self {
        Self { bus, cimd: Mutex::new(HashMap::new()) }
    }

    fn get<T: DeserializeOwned>(&self, owner: &str, project: &str, key: &str) -> Result<Option<T>, StoreError> {
        let value = self.bus.durable_get(owner, project, key).map_err(|e| StoreError(e.message))?;
        Ok(value.and_then(|v| serde_json::from_value(v).ok()))
    }

    fn put<T: Serialize>(&self, owner: &str, project: &str, key: &str, value: &T, ttl: u64) -> Result<(), StoreError> {
        let value = serde_json::to_value(value).map_err(|e| StoreError(e.to_string()))?;
        self.bus.durable_set(owner, project, key, value, Some(ttl.max(1))).map_err(|e| StoreError(e.message))
    }

    fn exists(&self, owner: &str, project: &str, key: &str) -> Result<bool, StoreError> {
        self.bus.durable_exists(owner, project, key).map_err(|e| StoreError(e.message))
    }

    /// Reads and deletes; `Some` only for the caller whose delete removed it.
    fn take<T: DeserializeOwned>(&self, owner: &str, project: &str, key: &str) -> Result<Option<T>, StoreError> {
        let Some(value) = self.get::<T>(owner, project, key)? else {
            return Ok(None);
        };
        let won = self.bus.durable_del(owner, project, key).map_err(|e| StoreError(e.message))?;
        Ok(won.then_some(value))
    }

    /// Counts one more under `bucket`; `false` once `cap` is passed.
    fn count(&self, owner: &str, project: &str, bucket: &str, ttl: u64, cap: i64) -> Result<bool, StoreError> {
        let n = self.bus.durable_incr(owner, project, bucket, 1).map_err(|e| StoreError(e.message))?;
        if n == 1 {
            let _ = self.bus.durable_expire(owner, project, bucket, Some(ttl));
        }
        Ok(n <= cap)
    }

    // ── clients ──────────────────────────────────────────────────────────

    /// Registers a client, unless the route's registrations today are spent.
    pub fn register_client(&self, owner: &str, project: &str, route: &str, client: &Client) -> Result<bool, StoreError> {
        let day = chrono::Utc::now().format("%Y%m%d");
        if !self.count(owner, project, &key(route, &format!("rate/register/{day}")), 2 * 86_400, MAX_REGISTRATIONS_PER_DAY)? {
            return Ok(false);
        }
        self.put(owner, project, &key(route, &format!("client/{}", client.client_id)), client, CLIENT_TTL)?;
        Ok(true)
    }

    pub fn client(&self, owner: &str, project: &str, route: &str, client_id: &str) -> Result<Option<Client>, StoreError> {
        if !rules::is_issued_client_id(client_id) {
            return Ok(None);
        }
        self.get(owner, project, &key(route, &format!("client/{client_id}")))
    }

    /// A client that keeps getting tokens is kept another [`CLIENT_TTL`].
    pub fn renew_client(&self, owner: &str, project: &str, route: &str, client_id: &str) {
        if rules::is_issued_client_id(client_id) {
            let _ = self.bus.durable_expire(owner, project, &key(route, &format!("client/{client_id}")), Some(CLIENT_TTL));
        }
    }

    // ── tickets ──────────────────────────────────────────────────────────

    /// Stores a ticket under its hash and answers the ticket, unless the
    /// route's authorizations this minute are spent (`None`).
    pub fn open_ticket(&self, owner: &str, project: &str, ticket: &Ticket) -> Result<Option<String>, StoreError> {
        let minute = chrono::Utc::now().format("%Y%m%d%H%M");
        if !self.count(owner, project, &key(&ticket.route, &format!("rate/ticket/{minute}")), 120, MAX_TICKETS_PER_MINUTE)? {
            return Ok(None);
        }
        let secret = rules::new_secret();
        // Project-wide: the app's login page knows the ticket, not the route.
        self.put(owner, project, &format!("zf.oauth/ticket/{}", rules::sha256_hex(&secret)), ticket, TICKET_TTL)?;
        Ok(Some(secret))
    }

    /// Spends a ticket: `None` when unknown, expired or already spent.
    pub fn take_ticket(&self, owner: &str, project: &str, secret: &str) -> Result<Option<Ticket>, StoreError> {
        self.take(owner, project, &format!("zf.oauth/ticket/{}", rules::sha256_hex(secret)))
    }

    // ── codes ────────────────────────────────────────────────────────────

    /// Stores an authorization code and answers it.
    pub fn issue_code(&self, owner: &str, project: &str, route: &str, code: &Code) -> Result<String, StoreError> {
        let secret = rules::new_secret();
        self.put(owner, project, &key(route, &format!("code/{}", rules::sha256_hex(&secret))), code, CODE_TTL)?;
        Ok(secret)
    }

    /// Spends a code. A code presented a second time revokes the family it
    /// started, so the tokens it was exchanged for die with it.
    pub fn take_code(&self, owner: &str, project: &str, route: &str, secret: &str) -> Result<Spent<Code>, StoreError> {
        let hash = rules::sha256_hex(secret);
        let used = key(route, &format!("code-used/{hash}"));
        if let Some(family) = self.get::<String>(owner, project, &used)? {
            self.revoke_family(owner, project, route, &family)?;
            return Ok(Spent::Refused);
        }
        let Some(code) = self.take::<Code>(owner, project, &key(route, &format!("code/{hash}")))? else {
            return Ok(Spent::Refused);
        };
        self.put(owner, project, &used, &code.family, USED_CODE_TTL)?;
        Ok(Spent::Fresh(code))
    }

    // ── refresh families ─────────────────────────────────────────────────

    /// Stores a refresh token of its family and answers it.
    pub fn issue_refresh(&self, owner: &str, project: &str, route: &str, refresh: &Refresh) -> Result<String, StoreError> {
        let secret = rules::new_secret();
        let ttl = (refresh.family_expires_at - now_unix()).max(1) as u64;
        self.put(owner, project, &key(route, &format!("refresh/{}", rules::sha256_hex(&secret))), refresh, ttl)?;
        Ok(secret)
    }

    /// Spends a refresh token. One presented again — stolen, or replayed —
    /// revokes its whole family (OAuth 2.1 §4.3.1).
    pub fn rotate_refresh(&self, owner: &str, project: &str, route: &str, secret: &str) -> Result<Spent<Refresh>, StoreError> {
        let hash = rules::sha256_hex(secret);
        let used = key(route, &format!("refresh-used/{hash}"));
        if let Some(family) = self.get::<String>(owner, project, &used)? {
            self.revoke_family(owner, project, route, &family)?;
            return Ok(Spent::Refused);
        }
        let Some(refresh) = self.take::<Refresh>(owner, project, &key(route, &format!("refresh/{hash}")))? else {
            return Ok(Spent::Refused);
        };
        let ttl = (refresh.family_expires_at - now_unix()).max(1) as u64;
        self.put(owner, project, &used, &refresh.family, ttl)?;
        if self.family_revoked(owner, project, route, &refresh.family)? || refresh.family_expires_at <= now_unix() {
            return Ok(Spent::Refused);
        }
        Ok(Spent::Fresh(refresh))
    }

    pub fn revoke_family(&self, owner: &str, project: &str, route: &str, family: &str) -> Result<(), StoreError> {
        self.put(owner, project, &key(route, &format!("family-revoked/{family}")), &true, REVOKED_FAMILY_TTL)
    }

    pub fn family_revoked(&self, owner: &str, project: &str, route: &str, family: &str) -> Result<bool, StoreError> {
        self.exists(owner, project, &key(route, &format!("family-revoked/{family}")))
    }

    // ── client ID metadata documents ─────────────────────────────────────

    pub fn cached_cimd(&self, client_id: &str) -> Option<CimdClient> {
        let cache = self.cimd.lock().unwrap_or_else(|e| e.into_inner());
        cache.get(client_id).filter(|(at, ttl, _)| at.elapsed() < *ttl).map(|(_, _, c)| c.clone())
    }

    pub fn cache_cimd(&self, client_id: &str, client: CimdClient, ttl: Duration) {
        let mut cache = self.cimd.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() >= CIMD_CACHE_MAX {
            cache.retain(|_, (at, ttl, _)| at.elapsed() < *ttl);
            if cache.len() >= CIMD_CACHE_MAX {
                cache.clear();
            }
        }
        cache.insert(client_id.to_string(), (Instant::now(), ttl, client));
    }
}

#[cfg(test)]
mod tests;
