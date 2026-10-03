//! `kv.*` — the project key-value store: `kv.entry.get`, `kv.entry.put`, `kv.entry.delete`,
//! `kv.entry.head`, `kv.entry.expire`, `kv.entry.increment`, `kv.message.publish`.

use crate::pipeline::NodeDefinition;

pub mod del;
pub mod exists;
pub mod expire;
pub mod get;
pub mod incr;
pub mod publish;
pub mod set;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        del::definition(),
        exists::definition(),
        expire::definition(),
        get::definition(),
        incr::definition(),
        publish::definition(),
        set::definition(),
    ]
}
