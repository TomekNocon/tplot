//! Graph utility for sankey diagrams: layer assignment + node ordering.
//!
//! Takes a Vec of `(source, target, value)` raw edges, assigns each unique
//! node a layer via Kahn's topological sort (`layer = max(layer_of_predecessors) + 1`),
//! aggregates duplicate edges, computes per-node value (`max(inflow, outflow)`),
//! and groups nodes per layer in first-seen order.
//!
//! Errors on cycles (sankey requires a DAG) and on empty input.

#[derive(Debug, Clone)]
pub struct RawEdge {
    pub source: String,
    pub target: String,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct SankeyNode {
    pub name: String,
    pub layer: usize,
    /// `value` = max(sum_inflow, sum_outflow). Used for sizing the node.
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct SankeyEdge {
    pub source: usize, // index into nodes
    pub target: usize, // index into nodes
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct SankeyGraph {
    pub nodes: Vec<SankeyNode>,
    pub edges: Vec<SankeyEdge>,
    /// `layers[layer_idx]` = ordered node indices in that layer.
    pub layers: Vec<Vec<usize>>,
}

#[derive(Debug, thiserror::Error)]
pub enum SankeyGraphError {
    #[error("graph has a cycle (sankey requires a DAG)")]
    Cycle,
    #[error("no edges supplied")]
    Empty,
}

pub fn build_graph(raws: &[RawEdge]) -> Result<SankeyGraph, SankeyGraphError> {
    if raws.is_empty() {
        return Err(SankeyGraphError::Empty);
    }

    // Discover unique node names in first-seen order across all edges.
    let mut names: Vec<String> = Vec::new();
    let mut name_to_idx = std::collections::HashMap::new();
    for r in raws {
        for n in [&r.source, &r.target] {
            if !name_to_idx.contains_key(n) {
                name_to_idx.insert(n.clone(), names.len());
                names.push(n.clone());
            }
        }
    }

    // Aggregate edge values: collapse duplicate (src, tgt) pairs into one.
    // Preserve first-seen order so iteration is deterministic.
    let mut edge_order: Vec<(usize, usize)> = Vec::new();
    let mut edge_idx: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();
    let mut edge_values: Vec<f64> = Vec::new();
    for r in raws {
        let s = name_to_idx[&r.source];
        let t = name_to_idx[&r.target];
        if let Some(&i) = edge_idx.get(&(s, t)) {
            edge_values[i] += r.value;
        } else {
            edge_idx.insert((s, t), edge_values.len());
            edge_order.push((s, t));
            edge_values.push(r.value);
        }
    }
    let edges: Vec<SankeyEdge> = edge_order
        .into_iter()
        .zip(edge_values)
        .map(|((source, target), value)| SankeyEdge {
            source,
            target,
            value,
        })
        .collect();

    // Build adjacency + indegree counts.
    let n = names.len();
    let mut indegree = vec![0usize; n];
    let mut adj: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for e in &edges {
        adj[e.source].push((e.target, e.value));
        indegree[e.target] += 1;
    }

    // Kahn's topological sort to assign layers.
    let mut layer_of = vec![0usize; n];
    let mut queue: std::collections::VecDeque<usize> =
        (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut visited = 0;
    while let Some(u) = queue.pop_front() {
        visited += 1;
        for &(v, _) in &adj[u] {
            if layer_of[u] + 1 > layer_of[v] {
                layer_of[v] = layer_of[u] + 1;
            }
            indegree[v] -= 1;
            if indegree[v] == 0 {
                queue.push_back(v);
            }
        }
    }
    if visited < n {
        return Err(SankeyGraphError::Cycle);
    }

    // Compute per-node value = max(inflow_sum, outflow_sum).
    let mut inflow = vec![0.0_f64; n];
    let mut outflow = vec![0.0_f64; n];
    for e in &edges {
        outflow[e.source] += e.value;
        inflow[e.target] += e.value;
    }
    let nodes: Vec<SankeyNode> = (0..n)
        .map(|i| SankeyNode {
            name: names[i].clone(),
            layer: layer_of[i],
            value: inflow[i].max(outflow[i]),
        })
        .collect();

    // Group node indices by layer (first-seen order within each layer).
    let max_layer = nodes.iter().map(|n| n.layer).max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (i, node) in nodes.iter().enumerate() {
        layers[node.layer].push(i);
    }

    Ok(SankeyGraph {
        nodes,
        edges,
        layers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(rows: &[(&str, &str, f64)]) -> Vec<RawEdge> {
        rows.iter()
            .map(|(s, t, v)| RawEdge {
                source: s.to_string(),
                target: t.to_string(),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn two_layer_funnel_has_two_layers() {
        let g = build_graph(&edges(&[
            ("landing", "signup", 100.0),
            ("landing", "bounce", 50.0),
        ]))
        .unwrap();
        assert_eq!(g.layers.len(), 2);
        assert_eq!(g.layers[0].len(), 1); // landing
        assert_eq!(g.layers[1].len(), 2); // signup + bounce
    }

    #[test]
    fn three_stage_funnel_has_three_layers() {
        let g = build_graph(&edges(&[
            ("landing", "signup", 100.0),
            ("signup", "active", 80.0),
        ]))
        .unwrap();
        assert_eq!(g.layers.len(), 3);
    }

    #[test]
    fn detects_cycles() {
        // a → b → a forms a cycle.
        let result = build_graph(&edges(&[("a", "b", 1.0), ("b", "a", 1.0)]));
        assert!(matches!(result, Err(SankeyGraphError::Cycle)));
    }

    #[test]
    fn node_layer_is_max_of_predecessor_layers_plus_one() {
        // a → b, a → c, b → c. c receives from both a (layer 0) and b (layer 1).
        // c's layer should be max(0, 1) + 1 = 2.
        let g = build_graph(&edges(&[("a", "b", 1.0), ("a", "c", 1.0), ("b", "c", 1.0)])).unwrap();
        let c_layer = g.nodes.iter().position(|n| n.name == "c").unwrap();
        assert_eq!(g.nodes[c_layer].layer, 2);
    }

    #[test]
    fn node_value_is_max_of_inflow_outflow() {
        let g = build_graph(&edges(&[
            ("a", "b", 100.0),
            ("a", "b", 50.0), // duplicate edge — sum to 150
        ]))
        .unwrap();
        let a = g.nodes.iter().find(|n| n.name == "a").unwrap();
        // a has 150 outflow and 0 inflow → value = 150
        assert!((a.value - 150.0).abs() < 1e-9);
        let b = g.nodes.iter().find(|n| n.name == "b").unwrap();
        assert!((b.value - 150.0).abs() < 1e-9);
    }
}
