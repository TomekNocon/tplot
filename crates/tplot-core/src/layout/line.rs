use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct LineSeries {
    pub key: String,
    /// Sub-pixel coordinates within the plot area (NOT canvas).
    /// (pixel_x, pixel_y) — pixel_y is inverted (high data → low y).
    pub points: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct LineLayout {
    pub plot_box: PlotBox,
    pub series: Vec<LineSeries>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    /// Data ranges for axis label rendering by the composer.
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum LineLayoutError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X: usize = 2; // Braille: 2 sub-pixel cols per cell
const SUB_Y: usize = 4; // Braille: 4 sub-pixel rows per cell
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_line(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    group_col: Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<LineLayout, LineLayoutError> {
    let xs: Vec<f64> = match df.column(x_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(LineLayoutError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(LineLayoutError::NonNumericY(y_col.to_string())),
    };
    if xs.is_empty() {
        return Err(LineLayoutError::Empty);
    }

    // Optional grouping: split rows into series.
    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g)?.series() {
            Series::Strings(v) => v.clone(),
            Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    // Compute global ranges (across all series) so all lines share the same axes.
    let x_min = xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_max = xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_min = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b)).min(0.0);
    let y_max = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let x_span = (x_max - x_min).max(1e-9);
    let y_span = (y_max - y_min).max(1e-9);

    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w = plot_cells_w * SUB_X;
    let plot_pixels_h = plot_cells_h * SUB_Y;

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    // Bucket rows by group, preserving first-seen order.
    let mut series: Vec<LineSeries> = Vec::new();
    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let px = (((x - x_min) / x_span) * (plot_pixels_w - 1) as f64).round() as usize;
        let py = ((y_max - y) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        let py = py.min(plot_pixels_h - 1);
        if let Some(s) = series.iter_mut().find(|s| s.key == *g) {
            s.points.push((px, py));
        } else {
            series.push(LineSeries {
                key: g.clone(),
                points: vec![(px, py)],
            });
        }
    }
    // Sort each series' points by x (so lines connect in order).
    for s in &mut series {
        s.points.sort_by_key(|p| p.0);
    }

    Ok(LineLayout {
        plot_box,
        series,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        x_min,
        x_max,
        y_min,
        y_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn ts_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("t", Series::Numbers(vec![1.0, 2.0, 3.0, 4.0, 5.0])),
            Column::new("v", Series::Numbers(vec![10.0, 25.0, 18.0, 42.0, 30.0])),
        ])
        .unwrap()
    }

    fn multi_series_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("t", Series::Numbers(vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0])),
            Column::new("v", Series::Numbers(vec![10.0, 5.0, 20.0, 8.0, 30.0, 12.0])),
            Column::new(
                "g",
                Series::Strings(
                    vec!["A", "B", "A", "B", "A", "B"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn single_series_produces_one_polyline() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        assert_eq!(layout.series.len(), 1);
        assert_eq!(layout.series[0].points.len(), 5);
    }

    #[test]
    fn multi_series_splits_by_group() {
        let layout = layout_line(&multi_series_df(), "t", "v", Some("g"), 80, 16).unwrap();
        assert_eq!(layout.series.len(), 2);
        for s in &layout.series {
            assert_eq!(s.points.len(), 3);
        }
    }

    #[test]
    fn x_axis_spans_data_range() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        let pts = &layout.series[0].points;
        // Leftmost point should be near x=0; rightmost should be near plot_pixel_width-1.
        let first_x = pts[0].0;
        let last_x = pts[pts.len() - 1].0;
        assert!(first_x < 4);
        assert!(last_x > layout.plot_box.pixel_width - 8);
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = ts_df();
        let err = layout_line(&df, "tt", "v", None, 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("tt"), "error should name the bad column: {msg}");
        assert!(msg.contains("`t`"), "error should suggest closest match: {msg}");
    }

    #[test]
    fn y_axis_inverted_max_at_top() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        let pts = &layout.series[0].points;
        // The point with highest data y (42 at t=4) should have the LOWEST pixel_y.
        let (_, max_idx) = pts
            .iter()
            .enumerate()
            .map(|(i, &(_, y))| (y, i))
            .min_by_key(|(y, _)| *y)
            .unwrap();
        assert_eq!(
            max_idx, 3,
            "highest data y should map to lowest pixel y (top of plot)"
        );
    }
}
