//! Treemap rasterizer — paints each leaf rectangle in its assigned color.
//! Focal-on-top order: focal is painted last so any rounding overlap resolves
//! to the focal color.

use crate::PixelBuffer;
use crate::layout::TreemapLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_treemap(
    layout: &TreemapLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal first, focal last (matches line/scatter convention).
    for leaf in &layout.leaves {
        if Some(leaf.series_key.as_str()) == focal {
            continue;
        }
        paint_leaf(leaf, palette, buf);
    }
    if let Some(name) = focal
        && let Some(leaf) = layout.leaves.iter().find(|l| l.series_key == name)
    {
        paint_leaf(leaf, palette, buf);
    }
}

fn paint_leaf(
    leaf: &crate::layout::TreemapLeaf,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&leaf.series_key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });
    if leaf.pixel_w == 0 || leaf.pixel_h == 0 {
        return;
    }
    let x0 = leaf.pixel_x;
    let x1 = leaf.pixel_x + leaf.pixel_w.saturating_sub(1);
    let y0 = leaf.pixel_y;
    let y1 = leaf.pixel_y + leaf.pixel_h.saturating_sub(1);
    buf.fill_rect(x0, y0, x1, y1, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, TreemapLayout, TreemapLeaf};
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

    fn fake_layout() -> TreemapLayout {
        TreemapLayout {
            plot_box: PlotBox {
                pixel_width: 12,
                pixel_height: 12,
            },
            leaves: vec![
                TreemapLeaf {
                    label: "BIG".into(),
                    value: 60.0,
                    cell_x: 0,
                    cell_y: 0,
                    cell_w: 8,
                    cell_h: 6,
                    pixel_x: 0,
                    pixel_y: 0,
                    pixel_w: 8,
                    pixel_h: 12,
                    series_key: "BIG".into(),
                },
                TreemapLeaf {
                    label: "small".into(),
                    value: 40.0,
                    cell_x: 8,
                    cell_y: 0,
                    cell_w: 4,
                    cell_h: 6,
                    pixel_x: 8,
                    pixel_y: 0,
                    pixel_w: 4,
                    pixel_h: 12,
                    series_key: "small".into(),
                },
            ],
            canvas_cells_w: 30,
            canvas_cells_h: 9,
            left_margin: 0,
            bottom_reserve: 3,
        }
    }

    #[test]
    fn paints_rects_in_assigned_colors() {
        let mut buf = crate::PixelBuffer::new(12, 12);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("BIG".into(), ORANGE);
        palette.insert("small".into(), GRAY);
        rasterize_treemap(&fake_layout(), Some("BIG"), &palette, &mut buf);
        assert_eq!(buf.get(0, 0), Some(ORANGE));
        assert_eq!(buf.get(7, 11), Some(ORANGE));
        assert_eq!(buf.get(8, 0), Some(GRAY));
        assert_eq!(buf.get(11, 11), Some(GRAY));
    }

    #[test]
    fn focal_paints_last_so_overlap_resolves_to_focal() {
        // (Treemap rects don't overlap by construction, but the painting order
        // still matters if cell rounding produces an off-by-one shared edge.)
        let mut buf = crate::PixelBuffer::new(12, 12);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("BIG".into(), ORANGE);
        palette.insert("small".into(), GRAY);
        rasterize_treemap(&fake_layout(), Some("BIG"), &palette, &mut buf);
        // BIG's bounds are x=0..7. The boundary at x=8 should be GRAY.
        assert_eq!(buf.get(8, 0), Some(GRAY));
    }
}
