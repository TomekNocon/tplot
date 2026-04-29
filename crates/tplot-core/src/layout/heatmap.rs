use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;
use tplot_protocol::{HeatRamp, RgbColor};

#[derive(Debug, Clone)]
pub struct HeatmapLayout {
    pub plot_box: PlotBox,
    pub x_labels: Vec<String>,
    pub y_labels: Vec<String>,
    /// `cell_colors[y_idx][x_idx]` = Some(color) for populated cells, None for empty.
    pub cell_colors: Vec<Vec<Option<RgbColor>>>,
    /// `cell_values[y_idx][x_idx]` for the takeaway template.
    pub cell_values: Vec<Vec<Option<f64>>>,
    /// (x_idx, y_idx) of the hottest cell, if any cells are populated.
    pub max_cell: Option<(usize, usize)>,
    pub max_value: f64,
    pub min_value: f64,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    /// Width per data column in terminal cells (≥ 1).
    pub cell_term_width: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum HeatmapError {
    #[error("value column `{0}` must be numeric")]
    NonNumericValue(String),
    #[error("no data rows")]
    Empty,
}

const SUB_Y_PER_CELL_ROW: usize = 2; // half-blocks: 2 sub-pixel rows per cell

pub fn layout_heatmap(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    value_col: &str,
    ramp: HeatRamp,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<HeatmapLayout, HeatmapError> {
    let xs: Vec<String> = match df.column(x_col).map_err(|_| HeatmapError::Empty)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let ys: Vec<String> = match df.column(y_col).map_err(|_| HeatmapError::Empty)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let vs: Vec<f64> = match df
        .column(value_col)
        .map_err(|_| HeatmapError::Empty)?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(HeatmapError::NonNumericValue(value_col.to_string())),
    };
    if xs.is_empty() {
        return Err(HeatmapError::Empty);
    }

    // Discover unique x and y labels in first-seen order.
    let mut x_labels: Vec<String> = Vec::new();
    let mut y_labels: Vec<String> = Vec::new();
    for x in &xs {
        if !x_labels.iter().any(|l| l == x) {
            x_labels.push(x.clone());
        }
    }
    for y in &ys {
        if !y_labels.iter().any(|l| l == y) {
            y_labels.push(y.clone());
        }
    }

    let nx = x_labels.len();
    let ny = y_labels.len();
    let mut grid: Vec<Vec<Option<f64>>> = vec![vec![None; nx]; ny];

    for ((x, y), v) in xs.iter().zip(ys.iter()).zip(vs.iter()) {
        let xi = x_labels.iter().position(|l| l == x).unwrap();
        let yi = y_labels.iter().position(|l| l == y).unwrap();
        let entry = &mut grid[yi][xi];
        *entry = Some(entry.unwrap_or(0.0) + v);
    }

    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    let mut max_cell: Option<(usize, usize)> = None;
    for (yi, row) in grid.iter().enumerate() {
        for (xi, cell) in row.iter().enumerate() {
            if let Some(v) = cell {
                if *v > max_v {
                    max_v = *v;
                    max_cell = Some((xi, yi));
                }
                if *v < min_v {
                    min_v = *v;
                }
            }
        }
    }
    if min_v.is_infinite() {
        min_v = 0.0;
        max_v = 0.0;
    }
    let span = (max_v - min_v).max(1e-9);

    // Map each populated cell to a color.
    let cell_colors: Vec<Vec<Option<RgbColor>>> = grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.map(|v| ramp.sample((v - min_v) / span)))
                .collect()
        })
        .collect();

    // Determine plot dimensions.
    let left_margin = y_labels.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 2;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(2);
    let cell_term_width = (plot_cells_w / nx).max(1);
    let plot_pixels_w = nx * cell_term_width;
    let plot_pixels_h = ny * SUB_Y_PER_CELL_ROW;
    let _ = plot_cells_h; // not load-bearing for the buffer; bottom_reserve handles it

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    Ok(HeatmapLayout {
        plot_box,
        x_labels,
        y_labels,
        cell_colors,
        cell_values: grid,
        max_cell,
        max_value: max_v,
        min_value: min_v,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        cell_term_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};
    use tplot_protocol::HeatRamp;

    fn small_grid_df() -> DataFrame {
        // 3 hours × 2 days, 6 cells total.
        DataFrame::from_columns(vec![
            Column::new(
                "hour",
                Series::Strings(
                    vec!["09", "10", "11", "09", "10", "11"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new(
                "day",
                Series::Strings(
                    vec!["Mon", "Mon", "Mon", "Tue", "Tue", "Tue"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new(
                "count",
                Series::Numbers(vec![5.0, 12.0, 8.0, 3.0, 25.0, 7.0]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_grid_with_one_color_per_cell() {
        let layout = layout_heatmap(
            &small_grid_df(),
            "hour",
            "day",
            "count",
            HeatRamp::Inferno,
            80,
            16,
        )
        .unwrap();
        // 3 columns × 2 rows = 6 colored cells.
        assert_eq!(layout.x_labels.len(), 3);
        assert_eq!(layout.y_labels.len(), 2);
        assert_eq!(layout.cell_colors.len(), 2);
        for row in &layout.cell_colors {
            assert_eq!(row.len(), 3);
        }
    }

    #[test]
    fn extremes_are_at_ramp_endpoints() {
        let layout = layout_heatmap(
            &small_grid_df(),
            "hour",
            "day",
            "count",
            HeatRamp::Inferno,
            80,
            16,
        )
        .unwrap();
        // count=25 (Tue × 10) should be the hottest → end of ramp.
        let (max_ix, max_iy) = layout.max_cell.unwrap();
        assert_eq!(layout.x_labels[max_ix], "10");
        assert_eq!(layout.y_labels[max_iy], "Tue");
        let max_color = layout.cell_colors[max_iy][max_ix].unwrap();
        // Inferno end is red-ish.
        assert!(max_color.r > 150);
    }

    #[test]
    fn aggregates_duplicate_pairs_by_sum() {
        let df = DataFrame::from_columns(vec![
            Column::new(
                "h",
                Series::Strings(
                    vec!["9", "9", "10"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new(
                "d",
                Series::Strings(
                    vec!["Mon", "Mon", "Mon"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new("c", Series::Numbers(vec![1.0, 2.0, 5.0])),
        ])
        .unwrap();
        let layout = layout_heatmap(&df, "h", "d", "c", HeatRamp::Inferno, 80, 8).unwrap();
        // (9, Mon) appears twice → summed to 3. (10, Mon) is 5.
        // Find positions and check.
        let nine_idx = layout.x_labels.iter().position(|l| l == "9").unwrap();
        let ten_idx = layout.x_labels.iter().position(|l| l == "10").unwrap();
        let mon_idx = layout.y_labels.iter().position(|l| l == "Mon").unwrap();
        // Both cells colored (no None).
        assert!(layout.cell_colors[mon_idx][nine_idx].is_some());
        assert!(layout.cell_colors[mon_idx][ten_idx].is_some());
        // Max is 5 (at "10, Mon"), so it should be ramp end.
        let max_color = layout.cell_colors[mon_idx][ten_idx].unwrap();
        let nine_color = layout.cell_colors[mon_idx][nine_idx].unwrap();
        assert!(
            max_color.r > nine_color.r,
            "expected hotter at the higher value"
        );
    }
}
