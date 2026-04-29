use crate::PixelBuffer;
use crate::layout::StackedAreaLayout;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_stacked_area(
    layout: &StackedAreaLayout,
    palette: &HashMap<String, RgbColor>,
    buf: &mut PixelBuffer,
) {
    let pw = layout.plot_box.pixel_width;
    let ph = layout.plot_box.pixel_height;
    if layout.series.is_empty() || layout.x_count == 0 || ph == 0 || pw == 0 {
        return;
    }
    let x_span = (layout.x_max - layout.x_min).max(1e-9);
    let y_max = layout.y_max.max(1e-9);

    for px in 0..pw {
        // Convert pixel-x to data-x via linear interpolation.
        let frac_x = if pw <= 1 {
            0.0
        } else {
            px as f64 / (pw - 1) as f64
        };
        let data_x = layout.x_min + frac_x * x_span;

        // Find surrounding data-x indices.
        let (i_lo, i_hi, t) = bracket(&layout.x_values, data_x);

        // Compute interpolated cumulative tops (data space) for each series.
        let mut cum_data: f64 = 0.0;
        for s in &layout.series {
            let v_lo = s.y_values[i_lo];
            let v_hi = s.y_values[i_hi];
            let v = v_lo + (v_hi - v_lo) * t;
            let cum_top_data = cum_data + v;
            let cum_bottom_data = cum_data;

            // Map data-y to pixel-y (inverted: high data → low pixel y).
            let py_top = ((y_max - cum_top_data) / y_max * (ph - 1) as f64).round() as usize;
            let py_bottom =
                ((y_max - cum_bottom_data) / y_max * (ph - 1) as f64).round() as usize;

            let lo = py_top.min(py_bottom);
            let hi = py_top.max(py_bottom).min(ph - 1);

            let color = palette.get(&s.key).copied().unwrap_or(RgbColor {
                r: 0x76,
                g: 0x76,
                b: 0x76,
            });

            for py in lo..=hi {
                buf.set(px, py, color);
            }

            cum_data = cum_top_data;
        }
    }
}

/// Linear-interpolation bracket: given a sorted `xs` and a target `x`, return
/// (lo_idx, hi_idx, t) such that `x ≈ xs[lo] + (xs[hi] - xs[lo]) * t`.
fn bracket(xs: &[f64], x: f64) -> (usize, usize, f64) {
    let n = xs.len();
    if n == 1 {
        return (0, 0, 0.0);
    }
    if x <= xs[0] {
        return (0, 0, 0.0);
    }
    if x >= xs[n - 1] {
        return (n - 1, n - 1, 0.0);
    }
    for i in 0..n - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let span = (xs[i + 1] - xs[i]).max(1e-9);
            return (i, i + 1, (x - xs[i]) / span);
        }
    }
    (n - 1, n - 1, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, StackedAreaLayout, StackedAreaSeries};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const RED: RgbColor = RgbColor {
        r: 0xff,
        g: 0x00,
        b: 0x00,
    };
    const BLUE: RgbColor = RgbColor {
        r: 0x00,
        g: 0x00,
        b: 0xff,
    };

    fn fake_layout() -> StackedAreaLayout {
        StackedAreaLayout {
            plot_box: PlotBox {
                pixel_width: 10,
                pixel_height: 10,
            },
            series: vec![
                StackedAreaSeries {
                    key: "A".into(),
                    y_values: vec![10.0, 10.0, 10.0],
                    total: 30.0,
                },
                StackedAreaSeries {
                    key: "B".into(),
                    y_values: vec![5.0, 5.0, 5.0],
                    total: 15.0,
                },
            ],
            x_values: vec![1.0, 2.0, 3.0],
            x_count: 3,
            totals_at_x: vec![15.0, 15.0, 15.0],
            canvas_cells_w: 30,
            canvas_cells_h: 8,
            left_margin: 6,
            bottom_reserve: 3,
            x_min: 1.0,
            x_max: 3.0,
            y_max: 15.0,
        }
    }

    #[test]
    fn paints_two_layers_in_their_colors() {
        let mut buf = crate::PixelBuffer::new(10, 10);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), RED);
        palette.insert("B".into(), BLUE);
        rasterize_stacked_area(&fake_layout(), &palette, &mut buf);

        // With totals all = 15, A occupies 10/15 = 2/3 of the height (top portion of the buffer).
        // B occupies the remaining 1/3 (bottom portion).
        // Pixel (5, 0) = top of buffer = top of stack. With y inverted, that's series B (cum_top=15).
        // Pixel (5, 9) = bottom of buffer = baseline (cum=0). That's INSIDE series A.
        assert_eq!(buf.get(5, 9), Some(RED), "bottom should be A");
        assert_eq!(buf.get(5, 0), Some(BLUE), "top should be B");
    }

    #[test]
    fn empty_buffer_when_no_series() {
        let layout = StackedAreaLayout {
            series: vec![],
            ..fake_layout()
        };
        let mut buf = crate::PixelBuffer::new(10, 10);
        let palette: HashMap<String, RgbColor> = HashMap::new();
        rasterize_stacked_area(&layout, &palette, &mut buf);
        assert_eq!(buf.get(0, 0), None);
        assert_eq!(buf.get(5, 5), None);
    }
}
