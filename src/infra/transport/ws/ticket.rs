//! Room tickets (`docs/contracts/rooms.md` § Opening a room and letting
//! people in): a pass for one user into one room, signed by this process
//! ([`crate::infra::secrets::signed`]), usable once, valid for its `--ttl`.
//!
//! A ticket names the room's key and the room's instance, so a ticket for a
//! room that closed does not open a later room of the same name. Its nonce
//! is remembered until the ticket would have expired anyway.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;

/// Shortest and longest life of a ticket, and the default.
pub const MIN_TTL: Duration = Duration::from_secs(1);
pub const MAX_TTL: Duration = Duration::from_secs(600);
pub const DEFAULT_TTL: Duration = Duration::from_secs(60);

/// A ticket as issued.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub token: String,
    /// Unix seconds.
    pub expires_at: i64,
}

/// A ticket that verified for a room: who it admits, and its nonce, spent at join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketPass {
    pub user: String,
    /// The room instance it was read against.
    pub instance: String,
    pub nonce: String,
    pub expires_at: i64,
}

/// The process's ticket key and the nonces already spent.
pub struct TicketBook {
    key: [u8; 32],
    spent: Mutex<HashMap<String, i64>>,
}

impl Default for TicketBook {
    fn default() -> Self {
        Self { key: crate::infra::secrets::signed::process_key(), spent: Mutex::new(HashMap::new()) }
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl TicketBook {
    /// A ticket for `user` into the room `room_key` (instance `instance`).
    pub fn issue(&self, room_key: &str, instance: &str, user: &str, ttl: Duration) -> Ticket {
        let expires_at = now() + ttl.as_secs() as i64;
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let token = crate::infra::secrets::signed::sign(
            &self.key,
            &json!({ "k": "room_ticket", "r": room_key, "i": instance, "u": user, "n": nonce, "x": expires_at }),
        );
        Ticket { token, expires_at }
    }

    /// The pass `token` gives into `room_key` (instance `instance`), unspent.
    pub fn read(&self, token: &str, room_key: &str, instance: &str) -> Result<TicketPass, String> {
        let payload = crate::infra::secrets::signed::verify(&self.key, token)?;
        let text = |k: &str| payload.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if text("k") != "room_ticket" {
            return Err("not a room ticket".to_string());
        }
        if text("r") != room_key || text("i") != instance {
            return Err("the ticket is for another room".to_string());
        }
        let expires_at = payload.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
        if expires_at <= now() {
            return Err("the ticket has expired".to_string());
        }
        let nonce = text("n");
        if self.spent.lock().unwrap_or_else(|e| e.into_inner()).contains_key(&nonce) {
            return Err("the ticket was already used".to_string());
        }
        Ok(TicketPass { user: text("u"), instance: instance.to_string(), nonce, expires_at })
    }

    /// Spend a pass; false when it was spent already.
    pub fn spend(&self, pass: &TicketPass) -> bool {
        let mut spent = self.spent.lock().unwrap_or_else(|e| e.into_inner());
        let now = now();
        spent.retain(|_, expires_at| *expires_at > now);
        spent.insert(pass.nonce.clone(), pass.expires_at).is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_admits_its_user_into_its_room_once() {
        let book = TicketBook::default();
        let ticket = book.issue("o/p/match-1", "i1", "u-17", DEFAULT_TTL);
        let pass = book.read(&ticket.token, "o/p/match-1", "i1").expect("valid");
        assert_eq!(pass.user, "u-17");
        assert!(book.read(&ticket.token, "o/p/match-2", "i1").is_err(), "another room");
        assert!(book.read(&ticket.token, "o/p/match-1", "i2").is_err(), "a reopened room of the same name");
        assert!(TicketBook::default().read(&ticket.token, "o/p/match-1", "i1").is_err(), "another process");
        assert!(book.spend(&pass));
        assert!(!book.spend(&pass), "usable once");
        assert_eq!(book.read(&ticket.token, "o/p/match-1", "i1").unwrap_err(), "the ticket was already used");
    }

    #[test]
    fn an_expired_ticket_is_refused() {
        let book = TicketBook::default();
        let ticket = book.issue("o/p/lobby", "i1", "u-17", Duration::ZERO);
        assert_eq!(book.read(&ticket.token, "o/p/lobby", "i1").unwrap_err(), "the ticket has expired");
    }
}
