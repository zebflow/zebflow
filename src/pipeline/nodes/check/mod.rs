//! A pipeline checked when it is saved, not when it runs
//! (`node-conventions.md` §3, §4, §11).
//!
//! Everything here reads the definitions alone, so the save, the activation
//! and `pipeline_check` agree with the run on every value already written
//! down:
//!
//! - [`check_node_config`] — one node's config: unknown keys, missing
//!   required flags, closed choices, numbers, durations and sizes, repeat
//!   ceilings, literal-only flags, the provider profile.
//! - [`check_pipeline`] — a whole graph: every node, kinds the catalogue
//!   does not have ("did you mean"), a provider's credential of the wrong
//!   kind, the flow rules; and, as warnings, references to keys no upstream
//!   node answers ([`references`]).

mod config;
pub mod references;
pub mod suggest;

use std::fmt;

use serde::Serialize;

pub use config::{LITERAL_FLAGS, check_node_config, provider_credential};

use crate::pipeline::NodeDefinition;
use crate::pipeline::model::PipelineGraph;
use crate::pipeline::nodes::shared::profile;

/// One thing wrong with a pipeline: where (node id and kind, empty for the
/// whole pipeline), which flag when one is to blame, and a sentence naming
/// what is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub node_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    pub message: String,
}

impl Problem {
    pub fn node(node_id: &str, kind: &str, flag: Option<&str>, message: String) -> Self {
        Self { node_id: node_id.to_string(), kind: kind.to_string(), flag: flag.map(str::to_string), message }
    }

    pub fn pipeline(message: String) -> Self {
        Self { node_id: String::new(), kind: String::new(), flag: None, message }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.node_id.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "node `{}`: {}", self.node_id, self.message)
        }
    }
}

/// What a check found: refusals stop a save and an activation; warnings
/// are reported and stop nothing (yet).
#[derive(Debug, Clone, Default, Serialize)]
pub struct PipelineCheck {
    pub refusals: Vec<Problem>,
    pub warnings: Vec<Problem>,
}

impl PipelineCheck {
    pub fn is_clean(&self) -> bool {
        self.refusals.is_empty() && self.warnings.is_empty()
    }

    /// The refusals as one message, headed by the file they are in.
    pub fn refusal_message(&self, file: &str) -> String {
        let count = self.refusals.len();
        let mut out = format!(
            "pipeline '{file}' is refused — {count} problem{}:",
            if count == 1 { "" } else { "s" }
        );
        for problem in &self.refusals {
            out.push_str(&format!("\n- {problem}"));
        }
        out
    }
}

/// How the catalogue a check is given relates to the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Catalogue {
    /// Every kind the project can use is in it: an unknown kind is refused.
    Complete,
    /// Only the official kinds are in it: an installed `x.` kind is left alone.
    Official,
}

/// Every node of a graph against its definition, and a kind the catalogue
/// does not have, with the kinds it most likely meant. `credential_kind`
/// answers the kind of a project credential by id, `None` when there is no
/// such credential (a missing one is the run's to refuse).
pub fn check_graph_nodes(
    graph: &PipelineGraph,
    defs: &[NodeDefinition],
    catalogue: Catalogue,
    credential_kind: &dyn Fn(&str) -> Option<String>,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    for node in &graph.nodes {
        let Some(def) = defs.iter().find(|d| d.kind == node.kind) else {
            if catalogue == Catalogue::Official && node.kind.starts_with("x.") {
                continue;
            }
            let hint = suggest::did_you_mean_phrase(&suggest::suggest_kinds(&node.kind, defs, 3));
            problems.push(Problem::node(
                &node.id,
                &node.kind,
                None,
                format!("unknown node kind `{}`{hint} — help(\"pipeline/nodes\") lists every kind", node.kind),
            ));
            continue;
        };
        for mut problem in check_node_config(def, &node.config) {
            problem.node_id = node.id.clone();
            problems.push(problem);
        }
        if let Some((provider, id)) = provider_credential(def, &node.config)
            && let Some(kind) = credential_kind(&id)
            && let Err(message) = profile::check_credential(def, &provider, &id, &kind)
        {
            problems.push(Problem::node(&node.id, &node.kind, Some(profile::CREDENTIAL_FLAG), format!("{} {message}", node.kind)));
        }
    }
    problems
}

/// The flow rules (§4) as a problem: a stray cycle, a loop wired across its
/// boundary, a `$nodes` reference to a node not upstream.
pub fn check_flow(graph: &PipelineGraph) -> Vec<Problem> {
    match crate::pipeline::engines::basic::validate_flow(graph) {
        Ok(()) => Vec::new(),
        Err(err) => vec![Problem::pipeline(err.message)],
    }
}

/// Everything a save refuses, and the reference warnings `pipeline_check`
/// adds.
pub fn check_pipeline(
    graph: &PipelineGraph,
    defs: &[NodeDefinition],
    catalogue: Catalogue,
    credential_kind: &dyn Fn(&str) -> Option<String>,
) -> PipelineCheck {
    let mut refusals = check_graph_nodes(graph, defs, catalogue, credential_kind);
    refusals.extend(check_flow(graph));
    PipelineCheck { refusals, warnings: references::check_references(graph, defs) }
}

/// No credential is known: the credential-kind check is skipped.
pub fn no_credentials(_: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests;
