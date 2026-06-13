//! Deterministic pipeline graph layout helpers.
//!
//! The persisted editor convention stores node coordinates in
//! `PipelineNode.config.ui.{x,y}`.  This module assigns those coordinates from graph
//! structure so DSL-authored and agent-authored pipelines open in the visual editor
//! as a readable left-to-right flow.

use std::collections::{BTreeMap, VecDeque};

use serde_json::{Map, Value, json};

use crate::pipeline::model::{PipelineGraph, PipelineNode};

const BASE_X: i64 = 120;
const BASE_Y: i64 = 120;
const RANK_GAP_X: i64 = 360;
const NODE_GAP_Y: i64 = 180;

/// Assigns stable, left-to-right coordinates to every node in `graph`.
///
/// The layout is a compact Sugiyama-style layered layout:
/// - rank nodes by maximum upstream dependency depth,
/// - keep disconnected/cyclic remnants stable by original order,
/// - run parent/child barycentric ordering sweeps to reduce crossings,
/// - write the result to `config.ui.x/y`.
pub fn auto_tidy_pipeline_graph(graph: &mut PipelineGraph) {
    if graph.nodes.is_empty() {
        return;
    }

    let ids = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    let original_order = ids
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.clone(), idx))
        .collect::<BTreeMap<_, _>>();
    let id_set = ids
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let output_pins = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.output_pins.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut incoming = ids
        .iter()
        .map(|id| (id.clone(), Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = ids
        .iter()
        .map(|id| (id.clone(), Vec::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut out_slot_order = BTreeMap::<(String, String), usize>::new();

    for edge in &graph.edges {
        if id_set.contains(&edge.from_node) && id_set.contains(&edge.to_node) {
            outgoing
                .entry(edge.from_node.clone())
                .or_default()
                .push(edge.to_node.clone());
            incoming
                .entry(edge.to_node.clone())
                .or_default()
                .push(edge.from_node.clone());
            let slot = output_pins
                .get(&edge.from_node)
                .and_then(|pins| pins.iter().position(|pin| pin == &edge.from_pin))
                .unwrap_or(0);
            out_slot_order.insert((edge.from_node.clone(), edge.to_node.clone()), slot);
        }
    }

    let mut indegree = incoming
        .iter()
        .map(|(id, parents)| (id.clone(), parents.len()))
        .collect::<BTreeMap<_, _>>();
    let mut ranks = BTreeMap::<String, usize>::new();
    let mut queue = VecDeque::<String>::new();

    let mut roots = graph
        .entry_nodes
        .iter()
        .filter(|id| id_set.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    roots.extend(
        ids.iter()
            .filter(|id| incoming.get(*id).map(Vec::is_empty).unwrap_or(true))
            .cloned(),
    );
    roots.extend(
        graph
            .nodes
            .iter()
            .filter(|node| node.kind.starts_with("n.trigger."))
            .map(|node| node.id.clone()),
    );
    roots.sort_by_key(|id| *original_order.get(id).unwrap_or(&usize::MAX));
    roots.dedup();
    for id in roots {
        if !ranks.contains_key(&id) {
            ranks.insert(id.clone(), 0);
            queue.push_back(id);
        }
    }

    while let Some(id) = queue.pop_front() {
        let rank = *ranks.get(&id).unwrap_or(&0);
        let children = outgoing.get(&id).cloned().unwrap_or_default();
        for child in children {
            let next_rank = rank + 1;
            ranks
                .entry(child.clone())
                .and_modify(|existing| *existing = (*existing).max(next_rank))
                .or_insert(next_rank);
            if let Some(count) = indegree.get_mut(&child) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    queue.push_back(child);
                }
            }
        }
    }

    // Cycles are valid at runtime.  For layout, place any nodes not reached by
    // the acyclic walk after their ranked parents where possible.
    for id in &ids {
        if ranks.contains_key(id) {
            continue;
        }
        let parent_rank = incoming
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|parent| ranks.get(parent).copied())
            .max();
        ranks.insert(id.clone(), parent_rank.map(|rank| rank + 1).unwrap_or(0));
    }

    let mut levels = BTreeMap::<usize, Vec<String>>::new();
    for id in &ids {
        levels
            .entry(*ranks.get(id).unwrap_or(&0))
            .or_default()
            .push(id.clone());
    }

    let mut order = original_order.clone();
    for _ in 0..4 {
        for ids_in_level in levels.values_mut() {
            ids_in_level.sort_by(|a, b| {
                let aw = average_neighbor_order(incoming.get(a), &order)
                    .unwrap_or(*original_order.get(a).unwrap_or(&usize::MAX) as f64);
                let bw = average_neighbor_order(incoming.get(b), &order)
                    .unwrap_or(*original_order.get(b).unwrap_or(&usize::MAX) as f64);
                aw.partial_cmp(&bw)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| original_order[a].cmp(&original_order[b]))
            });
            for (idx, id) in ids_in_level.iter().enumerate() {
                order.insert(id.clone(), idx);
            }
        }
        for ids_in_level in levels.values_mut().rev() {
            ids_in_level.sort_by(|a, b| {
                let aw = average_neighbor_order(outgoing.get(a), &order)
                    .unwrap_or(*original_order.get(a).unwrap_or(&usize::MAX) as f64);
                let bw = average_neighbor_order(outgoing.get(b), &order)
                    .unwrap_or(*original_order.get(b).unwrap_or(&usize::MAX) as f64);
                let slot_a = min_out_slot(a, outgoing.get(a), &out_slot_order);
                let slot_b = min_out_slot(b, outgoing.get(b), &out_slot_order);
                aw.partial_cmp(&bw)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| slot_a.cmp(&slot_b))
                    .then_with(|| original_order[a].cmp(&original_order[b]))
            });
            for (idx, id) in ids_in_level.iter().enumerate() {
                order.insert(id.clone(), idx);
            }
        }
    }

    for (rank, ids_in_level) in levels {
        for (row, id) in ids_in_level.iter().enumerate() {
            if let Some(node) = graph.nodes.iter_mut().find(|node| &node.id == id) {
                set_node_ui_position(
                    node,
                    BASE_X + rank as i64 * RANK_GAP_X,
                    BASE_Y + row as i64 * NODE_GAP_Y,
                );
            }
        }
    }
}

fn min_out_slot(
    from: &str,
    children: Option<&Vec<String>>,
    out_slot_order: &BTreeMap<(String, String), usize>,
) -> usize {
    children
        .into_iter()
        .flatten()
        .filter_map(|to| out_slot_order.get(&(from.to_string(), to.clone())).copied())
        .min()
        .unwrap_or(0)
}

fn average_neighbor_order(
    neighbors: Option<&Vec<String>>,
    order: &BTreeMap<String, usize>,
) -> Option<f64> {
    let values = neighbors?
        .iter()
        .filter_map(|id| order.get(id).copied())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<usize>() as f64 / values.len() as f64)
    }
}

fn set_node_ui_position(node: &mut PipelineNode, x: i64, y: i64) {
    if !node.config.is_object() {
        node.config = json!({});
    }
    let Some(config) = node.config.as_object_mut() else {
        return;
    };
    let ui = config
        .entry("ui")
        .or_insert_with(|| Value::Object(Map::new()));
    if !ui.is_object() {
        *ui = Value::Object(Map::new());
    }
    if let Some(ui_obj) = ui.as_object_mut() {
        ui_obj.insert("x".to_string(), json!(x));
        ui_obj.insert("y".to_string(), json!(y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::model::PipelineEdge;

    fn node(id: &str) -> PipelineNode {
        PipelineNode {
            id: id.to_string(),
            kind: "n.script".to_string(),
            input_pins: vec!["in".to_string()],
            output_pins: vec!["out".to_string()],
            config: json!({}),
        }
    }

    fn ui_x(node: &PipelineNode) -> i64 {
        node.config["ui"]["x"].as_i64().unwrap()
    }

    fn ui_y(node: &PipelineNode) -> i64 {
        node.config["ui"]["y"].as_i64().unwrap()
    }

    #[test]
    fn auto_tidy_layers_pipeline_left_to_right() {
        let mut graph = PipelineGraph {
            kind: "zebflow.pipeline".to_string(),
            version: "0.1".to_string(),
            id: "test".to_string(),
            description: None,
            metadata: None,
            entry_nodes: vec!["trigger".to_string()],
            nodes: vec![
                node("trigger"),
                node("branch_a"),
                node("branch_b"),
                node("join"),
            ],
            edges: vec![
                PipelineEdge {
                    from_node: "trigger".to_string(),
                    from_pin: "out".to_string(),
                    to_node: "branch_a".to_string(),
                    to_pin: "in".to_string(),
                },
                PipelineEdge {
                    from_node: "trigger".to_string(),
                    from_pin: "out".to_string(),
                    to_node: "branch_b".to_string(),
                    to_pin: "in".to_string(),
                },
                PipelineEdge {
                    from_node: "branch_a".to_string(),
                    from_pin: "out".to_string(),
                    to_node: "join".to_string(),
                    to_pin: "in".to_string(),
                },
                PipelineEdge {
                    from_node: "branch_b".to_string(),
                    from_pin: "out".to_string(),
                    to_node: "join".to_string(),
                    to_pin: "in".to_string(),
                },
            ],
        };

        auto_tidy_pipeline_graph(&mut graph);

        let by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<BTreeMap<_, _>>();
        assert!(ui_x(by_id["trigger"]) < ui_x(by_id["branch_a"]));
        assert_eq!(ui_x(by_id["branch_a"]), ui_x(by_id["branch_b"]));
        assert!(ui_x(by_id["branch_a"]) < ui_x(by_id["join"]));
        assert_ne!(ui_y(by_id["branch_a"]), ui_y(by_id["branch_b"]));
    }
}
