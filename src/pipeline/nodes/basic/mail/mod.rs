//! `mail.*` — outbound mail: `mail.send`.
//!
//! The message and the SMTP conversation belong to
//! [`mailbourne`](https://crates.io/crates/mailbourne). What stays here is
//! the part a mail engine cannot decide for its caller: which stored
//! credential, and what wraps the socket.

use crate::pipeline::NodeDefinition;

pub mod send;
pub mod transport;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![send::definition()]
}
