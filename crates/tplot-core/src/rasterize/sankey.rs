//! Rasterizer for sankey diagrams: paints edges as smoothstep-curved bands,
//! then nodes as filled rectangles on top.

use crate::PixelBuffer;
use crate::layout::SankeyLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

/// Cubic smoothstep: 3t² - 2t³. Maps [0,1] → [0,1] with slow-fast-slow easing.
fn smoothstep(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn rasterize_sankey(
    layout: &SankeyLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    node_color: RgbColor,
    buf: &mut PixelBuffer,
) {
    // 1. Paint edges (non-focal first, focal last).
    for e in &layout.edges {
        if Some(e.label.as_str()) == focal {
            continue;
        }
        paint_edge(e, palette, buf);
    }
    if let Some(name) = focal
        && let Some(e) = layout.edges.iter().find(|e| e.label == name)
    {
        paint_edge(e, palette, buf);
    }
    // 2. Paint nodes on top (so edge tips are tucked under the node rectangles).
    for n in &layout.nodes {
        let x0 = n.pixel_x;
        let x1 = n.pixel_x + n.pixel_width.saturating_sub(1);
        let y0 = n.pixel_y;
        let y1 = n.pixel_y + n.pixel_height.saturating_sub(1);
        buf.fill_rect(x0, y0, x1, y1, node_color);
    }
}

fn paint_edge(
    e: &crate::layout::SankeyEdgePos,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&e.label).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });
    if e.target_x <= e.source_x {
        return;
    }

    let x_span = (e.target_x - e.source_x) as f64;
    for px in e.source_x..=e.target_x {
        let t = (px - e.source_x) as f64 / x_span.max(1.0);
        let s = smoothstep(t);
        let y_top = e.source_y_top as f64 + (e.target_y_top as f64 - e.source_y_top as f64) * s;
        let y_top = y_top.round() as usize;
        let y_bottom = y_top + e.edge_height.saturating_sub(1);
        buf.fill_rect(px, y_top, px, y_bottom, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, SankeyEdgePos, SankeyLayout, SankeyNodePos};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };
    const GRAY: RgbColor = RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    };

    fn fake_layout() -> SankeyLayout {
        // Two nodes, one edge connecting them.
        SankeyLayout {
            plot_box: PlotBox {
                pixel_width: 30,
                pixel_height: 12,
            },
            nodes: vec![
                SankeyNodePos {
                    name: "src".into(),
                    layer: 0,
                    value: 100.0,
                    pixel_x: 2,
                    pixel_y: 2,
                    pixel_width: 3,
                    pixel_height: 8,
                },
                SankeyNodePos {
                    name: "tgt".into(),
                    layer: 1,
                    value: 100.0,
                    pixel_x: 25,
                    pixel_y: 2,
                    pixel_width: 3,
                    pixel_height: 8,
                },
            ],
            edges: vec![SankeyEdgePos {
                source_idx: 0,
                target_idx: 1,
                value: 100.0,
                source_x: 5,
                source_y_top: 2,
                target_x: 25,
                target_y_top: 2,
                edge_height: 8,
                label: "src → tgt".into(),
            }],
            canvas_cells_w: 40,
            canvas_cells_h: 8,
            bottom_reserve: 3,
        }
    }

    #[test]
    fn paints_node_rectangles() {
        let mut buf = crate::PixelBuffer::new(30, 12);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("src → tgt".into(), ORANGE);
        rasterize_sankey(&fake_layout(), Some("src → tgt"), &palette, GRAY, &mut buf);
        // src node spans (x=2..4, y=2..9).
        assert_eq!(buf.get(2, 2), Some(GRAY));
        assert_eq!(buf.get(4, 9), Some(GRAY));
        // tgt node spans (x=25..27, y=2..9).
        assert_eq!(buf.get(25, 2), Some(GRAY));
        assert_eq!(buf.get(27, 9), Some(GRAY));
    }

    #[test]
    fn paints_edge_curve_in_focal_color() {
        let mut buf = crate::PixelBuffer::new(30, 12);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("src → tgt".into(), ORANGE);
        rasterize_sankey(&fake_layout(), Some("src → tgt"), &palette, GRAY, &mut buf);
        // The edge should fill horizontally from x=5 to x=24 between source and target,
        // at roughly y=2..9 (source/target are aligned, so the curve is essentially a straight band).
        for x in 6..24 {
            assert_eq!(buf.get(x, 5), Some(ORANGE), "edge missing at ({x}, 5)");
            assert_eq!(buf.get(x, 8), Some(ORANGE), "edge missing at ({x}, 8)");
        }
    }
}
