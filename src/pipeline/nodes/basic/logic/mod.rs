//! `logic.*` — control flow: `logic.if`, `logic.match`, `logic.collect`,
//! `logic.foreach`, `logic.reduce`, `logic.retry`.

use crate::pipeline::NodeDefinition;

pub mod collect;
pub mod foreach_;
pub mod if_;
pub mod match_;
pub mod reduce;
pub mod retry;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        if_::definition(),
        match_::definition(),
        collect::definition(),
        foreach_::definition(),
        reduce::definition(),
        retry::definition(),
    ]
}
