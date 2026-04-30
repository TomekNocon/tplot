//! Violin plot rasterizer — paints the mirrored KDE shape per pixel-y row,
//! plus a horizontal median line in `MEDIAN_COLOR`.

use crate::PixelBuffer;
use crate::layout::ViolinLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

const MEDIAN_COLOR: RgbColor = RgbColor {
    r: 0xf0,
    g: 0xf6,
    b: 0xfc,
};

pub fn rasterize_violin(
    layout: &ViolinLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal first, focal last (so focal sits on top).
    for v in &layout.violins {
        if Some(v.series_key.as_str()) == focal {
            continue;
        }
        paint_violin(v, palette, buf);
    }
    if let Some(name) = focal
        && let Some(v) = layout.violins.iter().find(|v| v.series_key == name)
    {
        paint_violin(v, palette, buf);
    }
}

fn paint_violin(
    v: &crate::layout::Violin,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&v.series_key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });

    // Body fill: at each pixel-y row, paint horizontal pixels [pixel_x - hw, pixel_x + hw].
    for (py, &hw) in v.half_widths.iter().enumerate() {
        if hw == 0 {
            buf.set(v.pixel_x, py, color); // always paint at least the centerline
        } else {
            let x0 = v.pixel_x.saturating_sub(hw);
            let x1 = v.pixel_x + hw;
            buf.fill_rect(x0, py, x1, py, color);
        }
    }

    // Median line: 1-px horizontal across the widest part of the violin at median_y.
    let max_hw = *v.half_widths.iter().max().unwrap_or(&0);
    let mx0 = v.pixel_x.saturating_sub(max_hw);
    let mx1 = v.pixel_x + max_hw;
    for x in mx0..=mx1 {
        buf.set(x, v.median_y, MEDIAN_COLOR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, Violin, ViolinLayout};
    use crate::stats::FiveNumberSummary;
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn fake_layout() -> ViolinLayout {
        // Single violin centered at x=10, plot 20×20.
        // Diamond-shaped half-widths: 0, 1, 2, 3, 4, 4, 3, 2, 1, 0 ... etc.
        let half_widths: Vec<usize> = (0..20)
            .map(|y| if y < 10 { y / 2 } else { (19 - y) / 2 })
            .collect();
        ViolinLayout {
            plot_box: PlotBox {
                pixel_width: 20,
                pixel_height: 20,
            },
            violins: vec![Violin {
                label: "A".into(),
                summary: FiveNumberSummary {
                    min: 0.0,
                    q1: 25.0,
                    median: 50.0,
                    q3: 75.0,
                    max: 100.0,
                },
                pixel_x: 10,
                half_widths,
                median_y: 10,
                series_key: "A".into(),
            }],
            canvas_cells_w: 30,
            canvas_cells_h: 12,
            left_margin: 6,
            bottom_reserve: 3,
            bar_cell_width: 9,
            gap_cell_width: 2,
            y_min: -5.0,
            y_max: 105.0,
        }
    }

    #[test]
    fn paints_violin_shape_widest_at_middle() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        rasterize_violin(&fake_layout(), Some("A"), &palette, &mut buf);
        // At y=9 (just above mid), half_width = 4, so cells [6, 14] should be ORANGE.
        // Median line at y=10 will overwrite the body color with MEDIAN_COLOR.
        // Check painting a row above the median line.
        assert_eq!(buf.get(10, 9), Some(ORANGE)); // centerline
        assert_eq!(buf.get(6, 9), Some(ORANGE)); // left edge
        assert_eq!(buf.get(14, 9), Some(ORANGE)); // right edge
        // Outside the violin, no fill.
        assert_eq!(buf.get(5, 9), None);
        assert_eq!(buf.get(15, 9), None);
        // At y=0, half_width = 0 — only the centerline (or even nothing).
        // Allow either; test just ensures we don't paint outside.
        assert_eq!(buf.get(15, 0), None);
    }

    #[test]
    fn median_line_uses_distinct_color() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        rasterize_violin(&fake_layout(), Some("A"), &palette, &mut buf);
        let mid = buf.get(10, 10).unwrap();
        assert_ne!(mid, ORANGE, "median line should differ from violin body");
    }
}
