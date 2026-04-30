use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct StackedAreaSeries {
    pub key: String,
    /// Aligned to the layout's `x_values` grid; missing entries = 0.
    pub y_values: Vec<f64>,
    /// Sum across all x positions (used for the focal heuristic).
    pub total: f64,
}

#[derive(Debug, Clone)]
pub struct StackedAreaLayout {
    pub plot_box: PlotBox,
    pub series: Vec<StackedAreaSeries>,
    /// Sorted unique x values (data space).
    pub x_values: Vec<f64>,
    /// Number of unique x positions.
    pub x_count: usize,
    /// Total stack height per x position (sum of all series at that x).
    pub totals_at_x: Vec<f64>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum StackedAreaError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X_PER_CELL: usize = 1;
const SUB_Y_PER_CELL: usize = 2;
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_stacked_area(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    group_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<StackedAreaLayout, StackedAreaError> {
    let xs: Vec<f64> = match df.column(x_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(StackedAreaError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(StackedAreaError::NonNumericY(y_col.to_string())),
    };
    let groups: Vec<String> = match df.column(group_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    if xs.is_empty() {
        return Err(StackedAreaError::Empty);
    }

    // Discover unique sorted x values.
    let mut x_values: Vec<f64> = xs.clone();
    x_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    x_values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let x_count = x_values.len();

    // Discover unique series keys in first-seen order.
    let mut series_keys: Vec<String> = Vec::new();
    for g in &groups {
        if !series_keys.iter().any(|k| k == g) {
            series_keys.push(g.clone());
        }
    }

    // Build per-series y_values aligned to the x_values grid (sum on duplicates).
    let mut series: Vec<StackedAreaSeries> = series_keys
        .into_iter()
        .map(|key| StackedAreaSeries {
            key,
            y_values: vec![0.0; x_count],
            total: 0.0,
        })
        .collect();

    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let xi = x_values.iter().position(|v| (v - x).abs() < 1e-9).unwrap();
        let s = series.iter_mut().find(|s| s.key == *g).unwrap();
        s.y_values[xi] += y;
        s.total += y;
    }

    let totals_at_x: Vec<f64> = (0..x_count)
        .map(|xi| series.iter().map(|s| s.y_values[xi]).sum())
        .collect();

    let x_min = *x_values.first().unwrap();
    let x_max = *x_values.last().unwrap();
    let y_max = totals_at_x
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        .max(1e-9);

    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w = plot_cells_w * SUB_X_PER_CELL;
    let plot_pixels_h = plot_cells_h * SUB_Y_PER_CELL;

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    Ok(StackedAreaLayout {
        plot_box,
        series,
        x_values,
        x_count,
        totals_at_x,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        x_min,
        x_max,
        y_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn revenue_df() -> DataFrame {
        // 3 months × 2 regions. NA grows; EMEA stays flat-ish.
        DataFrame::from_columns(vec![
            Column::new("month", Series::Numbers(vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0])),
            Column::new(
                "rev",
                Series::Numbers(vec![10.0, 5.0, 20.0, 5.0, 40.0, 6.0]),
            ),
            Column::new(
                "g",
                Series::Strings(
                    vec!["NA", "EMEA", "NA", "EMEA", "NA", "EMEA"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_series_per_group() {
        let layout = layout_stacked_area(&revenue_df(), "month", "rev", "g", 80, 16).unwrap();
        assert_eq!(layout.series.len(), 2);
        assert_eq!(layout.x_count, 3);
    }

    #[test]
    fn cumulative_total_matches_sum_per_x() {
        let layout = layout_stacked_area(&revenue_df(), "month", "rev", "g", 80, 16).unwrap();
        // Month 1 total = 10 + 5 = 15. Month 3 total = 40 + 6 = 46.
        assert!((layout.totals_at_x[0] - 15.0).abs() < 1e-6);
        assert!((layout.totals_at_x[2] - 46.0).abs() < 1e-6);
    }

    #[test]
    fn series_order_is_first_seen() {
        let layout = layout_stacked_area(&revenue_df(), "month", "rev", "g", 80, 16).unwrap();
        assert_eq!(layout.series[0].key, "NA");
        assert_eq!(layout.series[1].key, "EMEA");
    }

    #[test]
    fn series_total_is_sum_across_x() {
        let layout = layout_stacked_area(&revenue_df(), "month", "rev", "g", 80, 16).unwrap();
        // NA: 10 + 20 + 40 = 70. EMEA: 5 + 5 + 6 = 16.
        let na = layout.series.iter().find(|s| s.key == "NA").unwrap();
        let emea = layout.series.iter().find(|s| s.key == "EMEA").unwrap();
        assert!((na.total - 70.0).abs() < 1e-6);
        assert!((emea.total - 16.0).abs() < 1e-6);
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = revenue_df();
        let err = layout_stacked_area(&df, "monthh", "rev", "g", 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("monthh"),
            "error should name the bad column: {msg}"
        );
        assert!(
            msg.contains("month"),
            "error should suggest the closest match: {msg}"
        );
    }
}
