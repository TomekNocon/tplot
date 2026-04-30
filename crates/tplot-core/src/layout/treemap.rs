//! Flat treemap layout — one leaf rectangle per input row, packed via squarify.

use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;
use crate::squarify::{Rect, squarify};

#[derive(Debug, Clone)]
pub struct TreemapLeaf {
    pub label: String,
    pub value: f64,
    /// Cell-coordinate position (column, row). Inclusive top-left.
    pub cell_x: usize,
    pub cell_y: usize,
    /// Cell-coordinate dimensions (width, height in cells).
    pub cell_w: usize,
    pub cell_h: usize,
    /// Sub-pixel coordinates for the rasterizer.
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub pixel_w: usize,
    pub pixel_h: usize,
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct TreemapLayout {
    pub plot_box: PlotBox,
    pub leaves: Vec<TreemapLeaf>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TreemapError {
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X_PER_CELL: usize = 1; // half-blocks renderer width factor
const SUB_Y_PER_CELL: usize = 2; // half-blocks renderer height factor

pub fn layout_treemap(
    df: &DataFrame,
    label_col: &str,
    value_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<TreemapLayout, TreemapError> {
    let labels: Vec<String> = match df.column(label_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(value_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(TreemapError::NonNumericY(value_col.to_string())),
    };
    if labels.is_empty() {
        return Err(TreemapError::Empty);
    }

    // Pair, filter out non-positive values, sort descending by value.
    let mut pairs: Vec<(String, f64)> = labels
        .into_iter()
        .zip(values)
        .filter(|(_, v)| *v > 0.0)
        .collect();
    if pairs.is_empty() {
        return Err(TreemapError::Empty);
    }
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Layout in cell coordinates so each rectangle's bounds align with cells.
    // Reserve a small margin for the legend below.
    let left_margin = 0; // treemap fills the full width
    let bottom_reserve = 3; // legend + takeaway
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);

    let target = Rect {
        x: 0.0,
        y: 0.0,
        w: plot_cells_w as f64,
        h: plot_cells_h as f64,
    };
    let just_values: Vec<f64> = pairs.iter().map(|(_, v)| *v).collect();
    let rects = squarify(&just_values, target);

    let leaves: Vec<TreemapLeaf> = pairs
        .into_iter()
        .zip(rects)
        .map(|((label, value), r)| {
            let cell_x = r.x.round() as usize;
            let cell_y = r.y.round() as usize;
            // Use right/bottom edges (rounded) to be edge-consistent.
            let cell_x_end = (r.x + r.w).round() as usize;
            let cell_y_end = (r.y + r.h).round() as usize;
            let cell_w = cell_x_end.saturating_sub(cell_x).max(1);
            let cell_h = cell_y_end.saturating_sub(cell_y).max(1);

            TreemapLeaf {
                label: label.clone(),
                value,
                cell_x,
                cell_y,
                cell_w,
                cell_h,
                pixel_x: cell_x * SUB_X_PER_CELL,
                pixel_y: cell_y * SUB_Y_PER_CELL,
                pixel_w: cell_w * SUB_X_PER_CELL,
                pixel_h: cell_h * SUB_Y_PER_CELL,
                series_key: label,
            }
        })
        .collect();

    let plot_box = PlotBox {
        pixel_width: plot_cells_w * SUB_X_PER_CELL,
        pixel_height: plot_cells_h * SUB_Y_PER_CELL,
    };

    Ok(TreemapLayout {
        plot_box,
        leaves,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn portfolio_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new(
                "asset",
                Series::Strings(
                    vec!["AAPL", "MSFT", "GOOG", "AMZN", "NVDA", "META"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new(
                "weight",
                Series::Numbers(vec![25.0, 20.0, 15.0, 12.0, 18.0, 10.0]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_leaf_per_row() {
        let layout = layout_treemap(&portfolio_df(), "asset", "weight", 80, 24).unwrap();
        assert_eq!(layout.leaves.len(), 6);
    }

    #[test]
    fn leaves_sorted_by_value_descending() {
        let layout = layout_treemap(&portfolio_df(), "asset", "weight", 80, 24).unwrap();
        assert_eq!(layout.leaves[0].label, "AAPL"); // 25
        assert_eq!(layout.leaves[1].label, "MSFT"); // 20
        for w in layout.leaves.windows(2) {
            assert!(w[0].value >= w[1].value);
        }
    }

    #[test]
    fn largest_leaf_has_largest_area() {
        let layout = layout_treemap(&portfolio_df(), "asset", "weight", 80, 24).unwrap();
        let aapl = layout.leaves.iter().find(|l| l.label == "AAPL").unwrap();
        let meta = layout.leaves.iter().find(|l| l.label == "META").unwrap();
        let aapl_area = aapl.cell_w * aapl.cell_h;
        let meta_area = meta.cell_w * meta.cell_h;
        assert!(aapl_area > meta_area);
    }
}
