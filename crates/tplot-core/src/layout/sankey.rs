//! Layout for sankey diagrams: places nodes in vertical "layer" columns,
//! computes per-edge endpoint positions stacked along each node's right/left edge.

use crate::layout::bar::PlotBox;
use crate::sankey_graph::SankeyGraph;

#[derive(Debug, Clone)]
pub struct SankeyNodePos {
    pub name: String,
    pub layer: usize,
    pub value: f64,
    /// Top-left of the node's rectangle in sub-pixel coords.
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

#[derive(Debug, Clone)]
pub struct SankeyEdgePos {
    pub source_idx: usize, // index into layout.nodes
    pub target_idx: usize, // index into layout.nodes
    pub value: f64,
    /// (x, y_top) for the source endpoint (right edge of source node).
    pub source_x: usize,
    pub source_y_top: usize,
    /// (x, y_top) for the target endpoint (left edge of target node).
    pub target_x: usize,
    pub target_y_top: usize,
    /// Edge height in sub-pixel rows (constant from source to target;
    /// proportional to flow value).
    pub edge_height: usize,
    /// Display label = "<source> → <target>" (used by the legend).
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct SankeyLayout {
    pub plot_box: PlotBox,
    pub nodes: Vec<SankeyNodePos>,
    pub edges: Vec<SankeyEdgePos>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub bottom_reserve: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SankeyLayoutError {
    #[error("graph has no nodes")]
    Empty,
}

const SUB_X_PER_CELL: usize = 1;
const SUB_Y_PER_CELL: usize = 2;
const NODE_WIDTH_CELLS: usize = 3;
const NODE_GAP_PIXELS: usize = 1;

pub fn layout_sankey(
    graph: &SankeyGraph,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<SankeyLayout, SankeyLayoutError> {
    if graph.nodes.is_empty() {
        return Err(SankeyLayoutError::Empty);
    }

    let bottom_reserve = 3; // legend + takeaway
    let plot_cells_w = canvas_cells_w.max(20);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(8);
    let plot_pixels_w = plot_cells_w * SUB_X_PER_CELL;
    let plot_pixels_h = plot_cells_h * SUB_Y_PER_CELL;

    let n_layers = graph.layers.len();
    let column_step = plot_pixels_w / n_layers.max(1);
    let node_w = (NODE_WIDTH_CELLS * SUB_X_PER_CELL)
        .min(column_step / 3)
        .max(1);

    // Compute per-node pixel placement.
    // Within each layer, total node value sums to the layer's "flow throughput";
    // scale heights so the largest layer fits in plot_pixels_h with gaps.
    let mut node_positions: Vec<SankeyNodePos> = Vec::with_capacity(graph.nodes.len());
    let mut idx_to_pos: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    let max_layer_value = graph
        .layers
        .iter()
        .map(|layer| layer.iter().map(|&i| graph.nodes[i].value).sum::<f64>())
        .fold(0.0_f64, f64::max)
        .max(1e-9);

    for (li, layer) in graph.layers.iter().enumerate() {
        let layer_total: f64 = layer.iter().map(|&i| graph.nodes[i].value).sum();
        let n_nodes_in_layer = layer.len();
        let total_gaps = NODE_GAP_PIXELS * n_nodes_in_layer.saturating_sub(1);
        let height_for_nodes = plot_pixels_h.saturating_sub(total_gaps);
        let scale = (height_for_nodes as f64) / max_layer_value;
        let layer_used_h: f64 = layer_total * scale;
        let leading_y = ((plot_pixels_h as f64 - layer_used_h - total_gaps as f64) / 2.0).max(0.0)
            as usize;

        let layer_x = li * column_step + column_step / 2 - node_w / 2;
        let mut y = leading_y;
        for &gi in layer.iter() {
            let node = &graph.nodes[gi];
            let h = (node.value * scale).round() as usize;
            let h = h.max(1);
            idx_to_pos.insert(gi, node_positions.len());
            node_positions.push(SankeyNodePos {
                name: node.name.clone(),
                layer: node.layer,
                value: node.value,
                pixel_x: layer_x,
                pixel_y: y,
                pixel_width: node_w,
                pixel_height: h,
            });
            y += h + NODE_GAP_PIXELS;
        }
    }

    // Compute edge endpoint placements.
    // For each source node, edges leaving it stack along its right edge in the
    // order of their targets' layer-position. Same logic for target endpoints.
    let mut source_offset: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut target_offset: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    let scale = (plot_pixels_h as f64) / max_layer_value;

    let mut edges_pos: Vec<SankeyEdgePos> = Vec::with_capacity(graph.edges.len());
    for e in &graph.edges {
        let src_pos_idx = idx_to_pos[&e.source];
        let tgt_pos_idx = idx_to_pos[&e.target];
        let src = &node_positions[src_pos_idx];
        let tgt = &node_positions[tgt_pos_idx];
        let edge_h = (e.value * scale).round() as usize;
        let edge_h = edge_h.max(1);

        let s_off = source_offset.entry(src_pos_idx).or_insert(0);
        let source_y_top = src.pixel_y + *s_off;
        *s_off += edge_h;

        let t_off = target_offset.entry(tgt_pos_idx).or_insert(0);
        let target_y_top = tgt.pixel_y + *t_off;
        *t_off += edge_h;

        edges_pos.push(SankeyEdgePos {
            source_idx: src_pos_idx,
            target_idx: tgt_pos_idx,
            value: e.value,
            source_x: src.pixel_x + src.pixel_width,
            source_y_top,
            target_x: tgt.pixel_x,
            target_y_top,
            edge_height: edge_h,
            label: format!("{} → {}", src.name, tgt.name),
        });
    }

    Ok(SankeyLayout {
        plot_box: PlotBox {
            pixel_width: plot_pixels_w,
            pixel_height: plot_pixels_h,
        },
        nodes: node_positions,
        edges: edges_pos,
        canvas_cells_w,
        canvas_cells_h,
        bottom_reserve,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sankey_graph::{RawEdge, build_graph};

    fn raws(rows: &[(&str, &str, f64)]) -> Vec<RawEdge> {
        rows.iter()
            .map(|(s, t, v)| RawEdge {
                source: s.to_string(),
                target: t.to_string(),
                value: *v,
            })
            .collect()
    }

    #[test]
    fn produces_one_node_pos_per_node() {
        let graph = build_graph(&raws(&[("a", "b", 10.0), ("a", "c", 5.0)])).unwrap();
        let layout = layout_sankey(&graph, 80, 16).unwrap();
        assert_eq!(layout.nodes.len(), 3);
    }

    #[test]
    fn produces_one_edge_pos_per_edge() {
        let graph = build_graph(&raws(&[("a", "b", 10.0), ("a", "c", 5.0)])).unwrap();
        let layout = layout_sankey(&graph, 80, 16).unwrap();
        assert_eq!(layout.edges.len(), 2);
    }

    #[test]
    fn layers_are_at_increasing_pixel_x() {
        let graph = build_graph(&raws(&[("a", "b", 10.0), ("b", "c", 10.0)])).unwrap();
        let layout = layout_sankey(&graph, 80, 16).unwrap();
        let a_x = layout.nodes.iter().find(|n| n.name == "a").unwrap().pixel_x;
        let b_x = layout.nodes.iter().find(|n| n.name == "b").unwrap().pixel_x;
        let c_x = layout.nodes.iter().find(|n| n.name == "c").unwrap().pixel_x;
        assert!(a_x < b_x);
        assert!(b_x < c_x);
    }

    #[test]
    fn nodes_have_height_proportional_to_value() {
        let graph = build_graph(&raws(&[
            ("source", "big", 100.0),
            ("source", "small", 10.0),
        ]))
        .unwrap();
        let layout = layout_sankey(&graph, 80, 16).unwrap();
        let big = layout.nodes.iter().find(|n| n.name == "big").unwrap();
        let small = layout.nodes.iter().find(|n| n.name == "small").unwrap();
        assert!(big.pixel_height > small.pixel_height * 5);
    }
}
