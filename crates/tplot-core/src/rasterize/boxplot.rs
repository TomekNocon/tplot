//! Box plot rasterizer — paints whiskers, IQR boxes, median lines, whisker
//! caps. Focal-on-top painting (consistent with line/scatter).

use crate::PixelBuffer;
use crate::layout::BoxPlotLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

const MEDIAN_COLOR: RgbColor = RgbColor {
    r: 0xf0,
    g: 0xf6,
    b: 0xfc,
};

pub fn rasterize_boxplot(
    layout: &BoxPlotLayout,
    focal: Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    // Paint non-focal first, focal last (consistent with line/scatter).
    for el in &layout.boxes {
        if Some(el.series_key.as_str()) == focal {
            continue;
        }
        paint_box(el, palette, layout.bar_cell_width, buf);
    }
    if let Some(name) = focal
        && let Some(el) = layout.boxes.iter().find(|e| e.series_key == name)
    {
        paint_box(el, palette, layout.bar_cell_width, buf);
    }
}

fn paint_box(
    el: &crate::layout::BoxPlotElement,
    palette: &HashMap<String, RgbColor>,
    bar_cells: usize,
    buf: &mut PixelBuffer,
) {
    let color = palette.get(&el.series_key).copied().unwrap_or(RgbColor {
        r: 0x76,
        g: 0x76,
        b: 0x76,
    });

    let half = bar_cells / 2;
    let x0 = el.pixel_x.saturating_sub(half);
    let x1 = el.pixel_x + half;

    // 1. Whisker: vertical 1-pixel line at pixel_x, from whisker_bottom_y to whisker_top_y.
    buf.draw_line(
        el.pixel_x,
        el.whisker_top_y,
        el.pixel_x,
        el.whisker_bottom_y,
        color,
    );

    // 2. IQR box: filled rectangle covering the box columns × box rows.
    buf.fill_rect(x0, el.box_top_y, x1, el.box_bottom_y, color);

    // 3. Median line: 1-pixel horizontal across the box at median_y, in MEDIAN_COLOR.
    for x in x0..=x1 {
        buf.set(x, el.median_y, MEDIAN_COLOR);
    }

    // 4. Whisker caps: a short horizontal at top and bottom.
    let cap_half = (bar_cells / 4).max(1);
    let cap_x0 = el.pixel_x.saturating_sub(cap_half);
    let cap_x1 = el.pixel_x + cap_half;
    for x in cap_x0..=cap_x1 {
        buf.set(x, el.whisker_top_y, color);
        buf.set(x, el.whisker_bottom_y, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{BoxPlotElement, BoxPlotLayout, PlotBox};
    use crate::stats::FiveNumberSummary;
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor {
        r: 0xee,
        g: 0x7b,
        b: 0x3d,
    };

    fn fake_layout() -> BoxPlotLayout {
        BoxPlotLayout {
            plot_box: PlotBox {
                pixel_width: 20,
                pixel_height: 20,
            },
            boxes: vec![BoxPlotElement {
                label: "/orders".into(),
                summary: FiveNumberSummary {
                    min: 30.0,
                    q1: 50.0,
                    median: 90.0,
                    q3: 200.0,
                    max: 400.0,
                },
                pixel_x: 5,
                whisker_top_y: 0,     // pixel for max=400 (top)
                box_top_y: 5,         // pixel for q3=200
                median_y: 10,         // pixel for median=90
                box_bottom_y: 14,     // pixel for q1=50
                whisker_bottom_y: 19, // pixel for min=30 (bottom)
                series_key: "/orders".into(),
            }],
            canvas_cells_w: 30,
            canvas_cells_h: 12,
            left_margin: 6,
            bottom_reserve: 3,
            bar_cell_width: 5,
            gap_cell_width: 2,
            y_min: 28.0,
            y_max: 420.0,
        }
    }

    #[test]
    fn paints_whisker_along_pixel_x() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("/orders".into(), ORANGE);
        rasterize_boxplot(&fake_layout(), Some("/orders"), &palette, &mut buf);

        // Whisker is a vertical line at pixel_x = 5 from y=0 to y=19.
        // (Inside the IQR box it's overwritten by the box fill, but ABOVE q3 and BELOW q1 it remains thin.)
        assert_eq!(buf.get(5, 0), Some(ORANGE), "whisker at top missing");
        assert_eq!(buf.get(5, 19), Some(ORANGE), "whisker at bottom missing");
        // Outside the whisker column, those rows should be empty.
        assert_eq!(buf.get(0, 0), None);
        assert_eq!(buf.get(8, 0), None);
    }

    #[test]
    fn paints_box_filled_across_bar_cell_width() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("/orders".into(), ORANGE);
        rasterize_boxplot(&fake_layout(), Some("/orders"), &palette, &mut buf);

        // Box spans pixel_x ± 2 (bar_cell_width=5 / 2 = 2 pixels each side), rows 5 to 14.
        // Pixel (3, 7) should be inside the box and painted ORANGE.
        assert_eq!(buf.get(3, 7), Some(ORANGE));
        assert_eq!(buf.get(7, 7), Some(ORANGE));
        // Outside the box (column-wise), e.g. (1, 7), should NOT be painted.
        assert_eq!(buf.get(1, 7), None);
    }

    #[test]
    fn paints_median_line_in_distinct_color() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("/orders".into(), ORANGE);
        rasterize_boxplot(&fake_layout(), Some("/orders"), &palette, &mut buf);

        // Median row (y=10) across the box should be painted with the
        // distinct median color (not ORANGE).
        let mid = buf.get(5, 10).unwrap();
        assert_ne!(mid, ORANGE, "median line should differ from box color");
    }
}
