//! Ridgeline rasterizer — paints each ridge as a filled area under the curve,
//! column by column, from `baseline_y - heights[px]` up to `baseline_y`.
//!
//! Painting order: non-focal ridges first in vec order (top to bottom in the
//! buffer); focal ridge painted last so it sits on top in any overlap zone.

use crate::PixelBuffer;
use crate::layout::RidgelineLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_ridgeline(
    layout: &RidgelineLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal ridges in order (top to bottom in the buffer = first to
    // last in the vec). Later ridges overwrite earlier ones in the overlap
    // zone, which produces the joy-plot stacking effect.
    for r in &layout.ridges {
        if Some(r.series_key.as_str()) == focal {
            continue;
        }
        paint_ridge(r, palette, buf);
    }
    // Focal ridge painted last → on top.
    if let Some(name) = focal
        && let Some(r) = layout.ridges.iter().find(|r| r.series_key == name)
    {
        paint_ridge(r, palette, buf);
    }
}

fn paint_ridge(r: &crate::layout::Ridge, palette: &HashMap<String, RgbColor>, buf: &mut PixelBuffer) {
    let color = palette.get(&r.series_key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });

    for (px, &h) in r.heights.iter().enumerate() {
        if h == 0 {
            continue;
        }
        let top = r.baseline_y.saturating_sub(h);
        let bot = r.baseline_y;
        for py in top..=bot {
            buf.set(px, py, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, Ridge, RidgelineLayout};
    use crate::stats::FiveNumberSummary;
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

    fn fake_layout() -> RidgelineLayout {
        // Two ridges in a 20×20 buffer. First (jan) baseline at y=10, second
        // (feb) at y=18. Both have a triangular peak shape.
        let triangle = |center: usize, n: usize, peak: usize| -> Vec<usize> {
            (0..n)
                .map(|i| {
                    let dist = if i > center { i - center } else { center - i };
                    if dist >= peak { 0 } else { peak - dist }
                })
                .collect()
        };
        RidgelineLayout {
            plot_box: PlotBox {
                pixel_width: 20,
                pixel_height: 20,
            },
            ridges: vec![
                Ridge {
                    label: "jan".into(),
                    summary: FiveNumberSummary {
                        min: 0.0,
                        q1: 25.0,
                        median: 50.0,
                        q3: 75.0,
                        max: 100.0,
                    },
                    heights: triangle(5, 20, 6),
                    baseline_y: 10,
                    max_height: 6,
                    median_x: 5,
                    series_key: "jan".into(),
                },
                Ridge {
                    label: "feb".into(),
                    summary: FiveNumberSummary {
                        min: 0.0,
                        q1: 25.0,
                        median: 50.0,
                        q3: 75.0,
                        max: 100.0,
                    },
                    heights: triangle(15, 20, 6),
                    baseline_y: 18,
                    max_height: 6,
                    median_x: 15,
                    series_key: "feb".into(),
                },
            ],
            canvas_cells_w: 30,
            canvas_cells_h: 12,
            left_margin: 8,
            bottom_reserve: 3,
            x_min: 0.0,
            x_max: 100.0,
        }
    }

    #[test]
    fn paints_ridge_under_curve_in_color() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("jan".into(), ORANGE);
        palette.insert("feb".into(), GRAY);
        rasterize_ridgeline(&fake_layout(), Some("jan"), &palette, &mut buf);

        // Jan's peak at column 5 has height 6 — fill from y=4 to y=10 should be ORANGE.
        assert_eq!(buf.get(5, 5), Some(ORANGE));
        assert_eq!(buf.get(5, 10), Some(ORANGE));
        // Above the peak (y=3) at column 5 should be empty.
        assert_eq!(buf.get(5, 3), None);
        // Feb's peak at column 15 has height 6 — fill from y=12 to y=18 should be GRAY.
        assert_eq!(buf.get(15, 14), Some(GRAY));
        assert_eq!(buf.get(15, 18), Some(GRAY));
    }
}
