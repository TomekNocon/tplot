use crate::dataframe::{DataFrame, Series};

pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct VerticalBarRect {
    pub label: String,
    pub value: f64,
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct VerticalBarLayout {
    pub plot_box: PlotBox,
    pub bars: Vec<VerticalBarRect>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    /// Cell width per bar column in the rendered output (≥ 1).
    pub bar_cell_width: usize,
    /// Cell width of the gap between bars.
    pub gap_cell_width: usize,
    /// Cells reserved on the left for the y-axis labels.
    pub left_margin: usize,
    /// Cells reserved at the bottom for x-axis labels and takeaway.
    pub bottom_reserve: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum VerticalBarLayoutError {
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
    #[error("canvas too narrow for {n} bars (need at least {needed} cells, have {have})")]
    TooNarrow {
        n: usize,
        needed: usize,
        have: usize,
    },
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_PIXELS_PER_CELL_Y: usize = 8;
const Y_AXIS_LABEL_WIDTH: usize = 6; // " 21.4 " etc.

pub fn layout_vertical_bar(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    _group_col: Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<VerticalBarLayout, VerticalBarLayoutError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(VerticalBarLayoutError::NonNumericY(y_col.to_string())),
    };
    if labels.is_empty() {
        return Err(VerticalBarLayoutError::Empty);
    }

    // Aggregate duplicate labels by sum.
    let mut agg: Vec<(String, f64)> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(slot) = agg.iter_mut().find(|(name, _)| name == l) {
            slot.1 += v;
        } else {
            agg.push((l.clone(), *v));
        }
    }

    let n = agg.len();
    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3; // 1 row x-axis labels + 1 blank + 1 takeaway

    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_h = plot_cells_h * SUB_PIXELS_PER_CELL_Y;

    // Pick bar_w + gap_w such that n*(bar_w+gap_w) ≤ plot_cells_w.
    // Default: gap_w = max(1, bar_w / 3).
    let max_bar_w = plot_cells_w / n;
    if max_bar_w == 0 {
        return Err(VerticalBarLayoutError::TooNarrow {
            n,
            needed: n,
            have: plot_cells_w,
        });
    }
    let bar_cell_width = max_bar_w.clamp(1, 8);
    let gap_cell_width = (bar_cell_width / 3)
        .max(1)
        .min(max_bar_w.saturating_sub(bar_cell_width));
    let group_w = bar_cell_width + gap_cell_width;
    let total_w = group_w * n;
    let leading = (plot_cells_w.saturating_sub(total_w)) / 2;

    let plot_box = PlotBox {
        pixel_width: plot_cells_w,
        pixel_height: plot_pixels_h,
    };

    let max_value = agg
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::MIN, f64::max)
        .max(1e-9);

    let mut bars = Vec::with_capacity(n);
    for (i, (label, value)) in agg.into_iter().enumerate() {
        let pixel_height = ((value / max_value) * plot_pixels_h as f64).round() as usize;
        // pixel_y is measured from the top of the plot area.
        let pixel_y = plot_pixels_h.saturating_sub(pixel_height);
        let pixel_x = leading + i * group_w;
        bars.push(VerticalBarRect {
            label: label.clone(),
            value,
            pixel_x,
            pixel_y,
            pixel_width: bar_cell_width,
            pixel_height,
            series_key: label,
        });
    }

    Ok(VerticalBarLayout {
        plot_box,
        bars,
        canvas_cells_w,
        canvas_cells_h,
        bar_cell_width,
        gap_cell_width,
        left_margin,
        bottom_reserve,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn small_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new(
                "month",
                Series::Strings(
                    vec!["Jan", "Feb", "Mar", "Apr"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new("active", Series::Numbers(vec![12.4, 13.1, 18.2, 21.4])),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_bar_per_row() {
        let layout = layout_vertical_bar(&small_df(), "month", "active", None, 80, 16).unwrap();
        assert_eq!(layout.bars.len(), 4);
    }

    #[test]
    fn tallest_bar_uses_most_of_plot_height() {
        let layout = layout_vertical_bar(&small_df(), "month", "active", None, 80, 16).unwrap();
        let tallest = layout.bars.iter().max_by_key(|b| b.pixel_height).unwrap();
        assert_eq!(tallest.label, "Apr");
        assert!(tallest.pixel_height >= layout.plot_box.pixel_height / 2);
    }

    #[test]
    fn shortest_bar_is_proportional() {
        let layout = layout_vertical_bar(&small_df(), "month", "active", None, 80, 16).unwrap();
        let shortest = layout.bars.iter().min_by_key(|b| b.pixel_height).unwrap();
        let tallest = layout.bars.iter().max_by_key(|b| b.pixel_height).unwrap();
        let ratio = shortest.pixel_height as f64 / tallest.pixel_height as f64;
        // 12.4 / 21.4 ≈ 0.579
        assert!((ratio - (12.4 / 21.4)).abs() < 0.05);
    }

    #[test]
    fn bars_are_at_least_one_cell_wide() {
        let layout = layout_vertical_bar(&small_df(), "month", "active", None, 30, 16).unwrap();
        for bar in &layout.bars {
            assert!(bar.pixel_width >= 1, "bar {:?} too narrow", bar.label);
        }
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = DataFrame::from_columns(vec![
            Column::new(
                "month",
                Series::Strings(vec!["Jan".into(), "Feb".into()]),
            ),
            Column::new("active", Series::Numbers(vec![10.0, 20.0])),
        ])
        .unwrap();
        let err = layout_vertical_bar(&df, "monht", "active", None, 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("monht"), "error should name the bad column: {msg}");
        assert!(
            msg.contains("month"),
            "error should suggest the closest match: {msg}"
        );
    }
}
