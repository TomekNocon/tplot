use crate::PixelBuffer;
use crate::layout::HeatmapLayout;

const SUB_Y_PER_CELL_ROW: usize = 2;

pub fn rasterize_heatmap(layout: &HeatmapLayout, buf: &mut PixelBuffer) {
    let cw = layout.cell_term_width;
    for (yi, row) in layout.cell_colors.iter().enumerate() {
        for (xi, cell) in row.iter().enumerate() {
            let Some(color) = *cell else {
                continue;
            };
            let x0 = xi * cw;
            let x1 = x0 + cw - 1;
            let y0 = yi * SUB_Y_PER_CELL_ROW;
            let y1 = y0 + SUB_Y_PER_CELL_ROW - 1;
            buf.fill_rect(x0, y0, x1, y1, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{HeatmapLayout, PlotBox};
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

    fn fake_layout() -> HeatmapLayout {
        // 2x2 grid of cells, cell_term_width = 3.
        HeatmapLayout {
            plot_box: PlotBox {
                pixel_width: 6,
                pixel_height: 4,
            },
            x_labels: vec!["a".into(), "b".into()],
            y_labels: vec!["1".into(), "2".into()],
            cell_colors: vec![
                vec![Some(RED), Some(BLUE)], // y=1 row
                vec![None, Some(RED)],       // y=2 row, (a,2) empty
            ],
            cell_values: vec![vec![Some(10.0), Some(20.0)], vec![None, Some(30.0)]],
            max_cell: Some((1, 1)),
            max_value: 30.0,
            min_value: 10.0,
            canvas_cells_w: 30,
            canvas_cells_h: 8,
            left_margin: 4,
            bottom_reserve: 3,
            cell_term_width: 3,
        }
    }

    #[test]
    fn paints_each_cell_in_its_color() {
        let mut buf = crate::PixelBuffer::new(6, 4);
        rasterize_heatmap(&fake_layout(), &mut buf);
        // Top-left cell (a, 1) should be RED across cols 0-2, rows 0-1.
        for y in 0..=1 {
            for x in 0..=2 {
                assert_eq!(buf.get(x, y), Some(RED), "expected RED at ({x},{y})");
            }
        }
        // Top-right cell (b, 1) should be BLUE.
        for y in 0..=1 {
            for x in 3..=5 {
                assert_eq!(buf.get(x, y), Some(BLUE), "expected BLUE at ({x},{y})");
            }
        }
        // Bottom-left cell is None — should remain unset.
        assert_eq!(buf.get(0, 2), None);
        assert_eq!(buf.get(2, 3), None);
        // Bottom-right cell (b, 2) should be RED.
        assert_eq!(buf.get(3, 2), Some(RED));
        assert_eq!(buf.get(5, 3), Some(RED));
    }
}
