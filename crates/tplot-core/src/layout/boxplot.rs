//! Box plot layout — long-form input grouped by `x_col`, per-group 5-number
//! summary, mapped to pixel-y space (inverted: high data → low pixel y).

use crate::dataframe::{DataFrame, Series, comma_list};
use crate::layout::bar::PlotBox;
use crate::stats::{FiveNumberSummary, five_number_summary};

#[derive(Debug, Clone)]
pub struct BoxPlotElement {
    pub label: String,
    pub summary: FiveNumberSummary,
    /// Center column of this box, in sub-pixel coords.
    pub pixel_x: usize,
    /// Pixel-y for each statistic. Inverted: high data → low pixel y.
    pub whisker_top_y: usize, // = pixel for `max`
    pub box_top_y: usize,        // = pixel for `q3`
    pub median_y: usize,         // = pixel for `median`
    pub box_bottom_y: usize,     // = pixel for `q1`
    pub whisker_bottom_y: usize, // = pixel for `min`
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct BoxPlotLayout {
    pub plot_box: PlotBox,
    pub boxes: Vec<BoxPlotElement>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    pub bar_cell_width: usize,
    pub gap_cell_width: usize,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum BoxPlotError {
    #[error("y column `{name}` must be numeric (numeric columns: {numeric})")]
    NonNumericY { name: String, numeric: String },
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X_PER_CELL: usize = 1; // half-blocks: 1 sub-pixel column per cell
const SUB_Y_PER_CELL: usize = 2; // half-blocks: 2 sub-pixel rows per cell
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_boxplot(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<BoxPlotLayout, BoxPlotError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => {
            return Err(BoxPlotError::NonNumericY {
                name: y_col.to_string(),
                numeric: comma_list(df.numeric_columns()),
            });
        }
    };
    if labels.is_empty() {
        return Err(BoxPlotError::Empty);
    }

    // Group values by label, preserve first-seen order.
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(slot) = groups.iter_mut().find(|(name, _)| name == l) {
            slot.1.push(*v);
        } else {
            groups.push((l.clone(), vec![*v]));
        }
    }

    // Filter out empty groups; compute summaries.
    let summaries: Vec<(String, FiveNumberSummary)> = groups
        .into_iter()
        .filter_map(|(l, vs)| five_number_summary(&vs).map(|s| (l, s)))
        .collect();
    if summaries.is_empty() {
        return Err(BoxPlotError::Empty);
    }

    // Global y-range: pad slightly above and below for whiskers to read clearly.
    let y_min_raw = summaries
        .iter()
        .map(|(_, s)| s.min)
        .fold(f64::INFINITY, f64::min);
    let y_max_raw = summaries
        .iter()
        .map(|(_, s)| s.max)
        .fold(f64::NEG_INFINITY, f64::max);
    let span_raw = (y_max_raw - y_min_raw).max(1e-9);
    let y_min = y_min_raw - span_raw * 0.05;
    let y_max = y_max_raw + span_raw * 0.05;
    let y_span = (y_max - y_min).max(1e-9);

    let n = summaries.len();
    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_h = plot_cells_h * SUB_Y_PER_CELL;

    let max_bar_w = plot_cells_w / n;
    if max_bar_w == 0 {
        return Err(BoxPlotError::Empty);
    }
    let bar_cell_width = max_bar_w.clamp(3, 7);
    let gap_cell_width = (bar_cell_width / 3)
        .max(1)
        .min(max_bar_w.saturating_sub(bar_cell_width));
    let group_w = bar_cell_width + gap_cell_width;
    let total_w = group_w * n;
    let leading = (plot_cells_w.saturating_sub(total_w)) / 2;

    let plot_box = PlotBox {
        pixel_width: plot_cells_w * SUB_X_PER_CELL,
        pixel_height: plot_pixels_h,
    };

    let map_y = |v: f64| -> usize {
        let py = ((y_max - v) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        py.min(plot_pixels_h - 1)
    };

    let boxes: Vec<BoxPlotElement> = summaries
        .into_iter()
        .enumerate()
        .map(|(i, (l, s))| {
            let pixel_x = leading + i * group_w + bar_cell_width / 2;
            BoxPlotElement {
                label: l.clone(),
                summary: s,
                pixel_x,
                whisker_top_y: map_y(s.max),
                box_top_y: map_y(s.q3),
                median_y: map_y(s.median),
                box_bottom_y: map_y(s.q1),
                whisker_bottom_y: map_y(s.min),
                series_key: l,
            }
        })
        .collect();

    Ok(BoxPlotLayout {
        plot_box,
        boxes,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        bar_cell_width,
        gap_cell_width,
        y_min,
        y_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn endpoints_df() -> DataFrame {
        // Two endpoints with different latency distributions.
        // /users: tight cluster around 50ms.
        // /orders: wider spread, much higher tail.
        DataFrame::from_columns(vec![
            Column::new(
                "endpoint",
                Series::Strings(
                    vec![
                        "/users", "/users", "/users", "/users", "/users", "/orders", "/orders",
                        "/orders", "/orders", "/orders", "/orders",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                ),
            ),
            Column::new(
                "ms",
                Series::Numbers(vec![
                    48.0, 50.0, 51.0, 52.0, 53.0, 30.0, 60.0, 100.0, 250.0, 400.0, 80.0,
                ]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_box_per_group() {
        let layout = layout_boxplot(&endpoints_df(), "endpoint", "ms", 80, 16).unwrap();
        assert_eq!(layout.boxes.len(), 2);
    }

    #[test]
    fn medians_match_data() {
        let layout = layout_boxplot(&endpoints_df(), "endpoint", "ms", 80, 16).unwrap();
        // /users median = 51
        let users = layout.boxes.iter().find(|b| b.label == "/users").unwrap();
        assert!((users.summary.median - 51.0).abs() < 0.01);
        // /orders median = 90 (between 80 and 100, interpolated to 90)
        let orders = layout.boxes.iter().find(|b| b.label == "/orders").unwrap();
        assert!((orders.summary.median - 90.0).abs() < 0.01);
    }

    #[test]
    fn wider_iqr_is_recognized() {
        let layout = layout_boxplot(&endpoints_df(), "endpoint", "ms", 80, 16).unwrap();
        let users = layout.boxes.iter().find(|b| b.label == "/users").unwrap();
        let orders = layout.boxes.iter().find(|b| b.label == "/orders").unwrap();
        assert!(
            orders.summary.iqr() > users.summary.iqr() * 5.0,
            "/orders IQR ({}) should dominate /users IQR ({})",
            orders.summary.iqr(),
            users.summary.iqr()
        );
    }

    #[test]
    fn non_numeric_y_lists_alternatives() {
        let df = endpoints_df();
        let err = layout_boxplot(&df, "endpoint", "endpoint", 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must be numeric"));
        assert!(
            msg.contains("ms"),
            "error should list numeric alternatives: {msg}"
        );
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = endpoints_df();
        let err = layout_boxplot(&df, "endpont", "ms", 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("endpont"),
            "error should name the bad column: {msg}"
        );
        assert!(
            msg.contains("endpoint"),
            "error should suggest the closest match: {msg}"
        );
    }

    #[test]
    fn pixel_y_is_inverted_max_at_top() {
        let layout = layout_boxplot(&endpoints_df(), "endpoint", "ms", 80, 16).unwrap();
        let orders = layout.boxes.iter().find(|b| b.label == "/orders").unwrap();
        // The `max` value (400) should map to a SMALL pixel_y (top of plot);
        // the `min` value (30) should map to a LARGE pixel_y (bottom).
        assert!(
            orders.whisker_top_y < orders.whisker_bottom_y,
            "whisker_top_y ({}) should be ABOVE whisker_bottom_y ({}) (smaller pixel y)",
            orders.whisker_top_y,
            orders.whisker_bottom_y
        );
    }
}
