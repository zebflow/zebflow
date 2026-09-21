//! `kv.*` — the project key-value store: `kv.get`, `kv.set`, `kv.del`,
//! `kv.exists`, `kv.expire`, `kv.incr`, `kv.publish`.

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
