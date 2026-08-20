//! What a package's nodes can reach, derived rather than declared.
//!
//! The safety review used to guess this from the node kind string. A kind is a
//! name, and a name is not evidence: `contains("pg")` matches any kind with
//! those two letters anywhere. This module answers the same question from the
//! node catalog instead, which is the only place that knows what a node does.
//!
//! Nothing in a package is taken at its word. A native node's capabilities are
//! a compile-time constant of this build. A composite node's are the union of
//! the nodes its function pipelines compose, so a bundle cannot understate them
//! without also removing the nodes. A WASM node's are empty, and empty is a
//! fact about [`crate::pipeline::engines::wasm_host`] rather than a claim the
//! bundle made.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use serde_json::Value;

use crate::pipeline::model::NodeCapability;
use crate::platform::model::MultiNodePackageDefinition;

/// The prefix every runnable node kind carries.
///
/// A pipeline document nests node kinds under a `kind` key, and so does the
/// contract envelope around it -- whose own `kind` is `Pipeline`. The prefix is
/// what separates the two without this module having to know which nesting a
/// given document uses, and the engine agrees: dispatch treats anything under
/// `n.` as a node and everything else as unsupported.
const NODE_KIND_PREFIX: &str = "n.";

/// Every node kind a pipeline document names, at any nesting.
pub fn node_kinds_in_pipeline(value: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect_node_kinds(value, &mut found);
    found
}

fn collect_node_kinds(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(kind) = map
                .get("kind")
                .and_then(Value::as_str)
                .filter(|kind| kind.starts_with(NODE_KIND_PREFIX))
            {
                out.insert(kind.to_string());
            }
            for child in map.values() {
                collect_node_kinds(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_node_kinds(item, out);
            }
        }
        _ => {}
    }
}

/// What every node kind this build can run without installing anything can do.
///
/// Two sources, both fixed at compile time: the native catalog, and the
/// composite bundles baked into the binary. The embedded bundles are already
/// derived by the registry that loads them, so this map never derives the same
/// bundle twice and never disagrees with the runtime about one.
pub fn known_node_capabilities() -> &'static BTreeMap<String, BTreeSet<NodeCapability>> {
    static KNOWN: LazyLock<BTreeMap<String, BTreeSet<NodeCapability>>> = LazyLock::new(|| {
        let mut known = crate::pipeline::nodes::native_node_capabilities().clone();
        for definition in
            crate::platform::services::NodeRegistryService::embedded_official_definitions()
        {
            known.insert(
                definition.kind,
                definition.capabilities.into_iter().collect(),
            );
        }
        known
    });
    &KNOWN
}

/// What each node a bundle declares can do, and what could not be resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DerivedBundleCapabilities {
    /// Node kind the bundle declares, mapped to what it can reach.
    pub nodes: BTreeMap<String, BTreeSet<NodeCapability>>,
    /// Kinds the bundle's function pipelines name that neither this build nor
    /// the bundle itself provides.
    ///
    /// These are the nodes the derivation could not see through. They are a
    /// finding rather than an absence: a set that silently skipped them would
    /// understate the bundle, which is the failure this module exists to avoid.
    pub unresolved: BTreeSet<String>,
}

/// Derives every declared node's capabilities from the pipelines it composes.
///
/// `function_graph` reads one package-relative function pipeline, returning
/// `None` when the package does not carry it. A function that cannot be read
/// contributes nothing and is not guessed at -- the caller sees the missing
/// file as its own finding.
pub fn derive_bundle_capabilities(
    package: &MultiNodePackageDefinition,
    function_graph: impl Fn(&str) -> Option<Value>,
    known: &BTreeMap<String, BTreeSet<NodeCapability>>,
) -> DerivedBundleCapabilities {
    // Read each function once. Several nodes may share one, and a lifecycle
    // hook is just another entry in the same map.
    let composed_by_function: BTreeMap<String, BTreeSet<String>> = package
        .functions
        .iter()
        .filter_map(|(name, rel_path)| {
            let graph = function_graph(rel_path)?;
            Some((name.clone(), node_kinds_in_pipeline(&graph)))
        })
        .collect();

    let mut composed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in &package.nodes {
        let mut kinds = BTreeSet::new();
        for binding in node.run.iter().chain(
            node.lifecycle
                .iter()
                .flat_map(|lifecycle| lifecycle.on_activate.iter().chain(&lifecycle.on_deactivate)),
        ) {
            // A WASM binding names a module, not a function, and a module gets
            // no host imports at all -- so it composes nothing and contributes
            // nothing. See `NodeCapability` for why that is a property of the
            // host rather than a claim of the bundle's.
            if let Some(function) = binding.function.as_deref()
                && let Some(inner) = composed_by_function.get(function)
            {
                kinds.extend(inner.iter().cloned());
            }
        }
        composed.insert(node.kind.clone(), kinds);
    }

    let mut derived: BTreeMap<String, BTreeSet<NodeCapability>> = composed
        .keys()
        .map(|kind| (kind.clone(), BTreeSet::new()))
        .collect();
    let mut unresolved = BTreeSet::new();

    // A bundle node may compose another node from the same bundle, so one pass
    // can read a set that is still empty. Sets only ever grow and are bounded,
    // so repeating until nothing changes terminates -- and it terminates on a
    // cycle too, which a recursive walk would not.
    loop {
        let mut changed = false;
        for (kind, inner_kinds) in &composed {
            let mut union = BTreeSet::new();
            for inner in inner_kinds {
                if let Some(capabilities) = known.get(inner) {
                    union.extend(capabilities.iter().copied());
                } else if let Some(capabilities) = derived.get(inner) {
                    union.extend(capabilities.iter().copied());
                } else {
                    unresolved.insert(inner.clone());
                }
            }
            let entry = derived.entry(kind.clone()).or_default();
            if !union.is_subset(entry) {
                entry.extend(union);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    DerivedBundleCapabilities {
        nodes: derived,
        unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn package(spec: Value) -> MultiNodePackageDefinition {
        serde_json::from_value(spec).expect("package spec")
    }

    fn graph(kinds: &[&str]) -> Value {
        json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "Pipeline",
            "metadata": {"name": "fn"},
            "spec": {
                "id": "fn",
                "nodes": kinds
                    .iter()
                    .enumerate()
                    .map(|(index, kind)| json!({"id": format!("n{index}"), "kind": kind}))
                    .collect::<Vec<_>>(),
                "edges": [],
            }
        })
    }

    /// The envelope's own `kind` is `Pipeline`, and a node kind is not.
    #[test]
    fn only_node_kinds_are_read_out_of_a_pipeline_document() {
        let kinds = node_kinds_in_pipeline(&graph(&["n.trigger.function", "n.http.request"]));
        assert_eq!(
            kinds,
            BTreeSet::from([
                "n.http.request".to_string(),
                "n.trigger.function".to_string()
            ])
        );
    }

    /// The headline claim: a composite is exactly the union of what it composes,
    /// and it composed nothing it did not name.
    #[test]
    fn a_composite_is_the_union_of_the_nodes_it_composes() {
        let spec = package(json!({
            "package": "acme",
            "version": "1.0.0",
            "title": "Acme",
            "functions": {"main": "functions/main.zf.json"},
            "nodes": [{
                "kind": "n.x.acme.sync",
                "title": "Sync",
                "run": {"function": "main"},
                "definition": {}
            }]
        }));
        let known = BTreeMap::from([
            (
                "n.http.request".to_string(),
                BTreeSet::from([NodeCapability::Network]),
            ),
            (
                "n.fs.put".to_string(),
                BTreeSet::from([NodeCapability::Filesystem]),
            ),
            ("n.trigger.function".to_string(), BTreeSet::new()),
        ]);

        let derived = derive_bundle_capabilities(
            &spec,
            |rel| {
                (rel == "functions/main.zf.json")
                    .then(|| graph(&["n.trigger.function", "n.http.request", "n.fs.put"]))
            },
            &known,
        );

        assert_eq!(
            derived.nodes["n.x.acme.sync"],
            BTreeSet::from([NodeCapability::Network, NodeCapability::Filesystem])
        );
        assert!(derived.unresolved.is_empty());
    }

    /// A WASM node names a module rather than a function, so there is no graph
    /// to union and the host grants it no imports. Empty is the answer, and it
    /// is the answer for every WASM node in every bundle.
    #[test]
    fn a_wasm_node_derives_no_capability_at_all() {
        let spec = package(json!({
            "package": "acme",
            "version": "1.0.0",
            "title": "Acme",
            "modules": {"core": {"path": "wasm/core.wasm", "abi": "zebflow-wasm-json-v1"}},
            "nodes": [{
                "kind": "n.x.acme.crunch",
                "title": "Crunch",
                "run": {"module": "core", "export": "run"},
                "definition": {}
            }]
        }));

        let derived = derive_bundle_capabilities(&spec, |_| None, &BTreeMap::new());

        assert!(derived.nodes["n.x.acme.crunch"].is_empty());
        assert!(derived.unresolved.is_empty());
    }

    /// A node composing another node of the same bundle inherits through it,
    /// and a cycle between two of them still terminates.
    #[test]
    fn a_composite_inherits_through_another_node_of_its_own_bundle() {
        let spec = package(json!({
            "package": "acme",
            "version": "1.0.0",
            "title": "Acme",
            "functions": {
                "outer": "functions/outer.zf.json",
                "inner": "functions/inner.zf.json"
            },
            "nodes": [
                {"kind": "n.x.acme.outer", "title": "Outer",
                 "run": {"function": "outer"}, "definition": {}},
                {"kind": "n.x.acme.inner", "title": "Inner",
                 "run": {"function": "inner"}, "definition": {}}
            ]
        }));
        let known = BTreeMap::from([(
            "n.pg.query".to_string(),
            BTreeSet::from([NodeCapability::Database, NodeCapability::Credential]),
        )]);

        let derived = derive_bundle_capabilities(
            &spec,
            |rel| match rel {
                // The cycle is deliberate: outer names inner and inner names
                // outer back.
                "functions/outer.zf.json" => Some(graph(&["n.x.acme.inner"])),
                "functions/inner.zf.json" => Some(graph(&["n.pg.query", "n.x.acme.outer"])),
                _ => None,
            },
            &known,
        );

        let expected = BTreeSet::from([NodeCapability::Database, NodeCapability::Credential]);
        assert_eq!(derived.nodes["n.x.acme.outer"], expected);
        assert_eq!(derived.nodes["n.x.acme.inner"], expected);
    }

    /// A kind neither this build nor the bundle provides is reported, not
    /// skipped: a silently dropped node understates what the bundle does.
    #[test]
    fn a_kind_nothing_provides_is_reported_as_unresolved() {
        let spec = package(json!({
            "package": "acme",
            "version": "1.0.0",
            "title": "Acme",
            "functions": {"main": "functions/main.zf.json"},
            "nodes": [{
                "kind": "n.x.acme.sync",
                "title": "Sync",
                "run": {"function": "main"},
                "definition": {}
            }]
        }));

        let derived = derive_bundle_capabilities(
            &spec,
            |_| Some(graph(&["n.x.elsewhere.thing"])),
            &BTreeMap::new(),
        );

        assert_eq!(
            derived.unresolved,
            BTreeSet::from(["n.x.elsewhere.thing".to_string()])
        );
        assert!(derived.nodes["n.x.acme.sync"].is_empty());
    }

    /// The bundles this binary ships are the derivation's real input, and the
    /// embedding node is exactly the case a name could never have revealed: its
    /// only outbound call is a URL assembled from a credential placeholder.
    #[test]
    fn an_embedded_composite_reports_what_its_function_pipeline_reaches() {
        let embedding = known_node_capabilities()
            .get("n.ai.embedding")
            .expect("the openai-embedding bundle is embedded in this binary");

        assert!(
            embedding.contains(&NodeCapability::Network),
            "{embedding:?}"
        );
        assert!(
            embedding.contains(&NodeCapability::Credential),
            "{embedding:?}"
        );
        assert!(
            embedding.contains(&NodeCapability::Process),
            "{embedding:?}"
        );
    }

    /// Every kind the engine can dispatch natively has an answer, so the review
    /// never has to fall back to guessing for a node this build ships.
    #[test]
    fn every_native_kind_has_a_capability_answer() {
        let known = known_node_capabilities();
        for definition in crate::pipeline::nodes::builtin_node_definitions() {
            assert!(
                known.contains_key(&definition.kind),
                "{} has no capability entry",
                definition.kind
            );
        }
    }
}
