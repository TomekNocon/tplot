use crate::dataframe::{DataFrame, Series, comma_list};

/// Region of the buffer (in sub-pixel coords) reserved for the chart itself.
/// In v1 the buffer holds *only* the plot area — labels and value strings are
/// composed around the rendered output by the binary, not painted into the
/// buffer.
#[derive(Debug, Clone, Copy)]
pub struct PlotBox {
    pub pixel_width: usize,
    pub pixel_height: usize,
}

#[derive(Debug, Clone)]
pub struct BarRect {
    pub label: String,
    pub value: f64,
    /// Sub-pixel rect within the plot area (x always starts at 0 in v1).
    pub pixel_x: usize,
    pub pixel_y: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
    /// Series the bar belongs to (for color routing). For ungrouped charts
    /// each bar is its own "series" keyed by `label`.
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct BarLayout {
    pub plot_box: PlotBox,
    pub bars: Vec<BarRect>,
    /// Cell width / height of the surrounding canvas (for the composer).
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    /// Width in cells of the longest left label (used by the composer).
    pub label_margin: usize,
    /// Width in cells reserved on the right for the value text.
    pub value_margin: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("y column `{name}` must be numeric (numeric columns: {numeric})")]
    NonNumericY { name: String, numeric: String },
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_PIXELS_PER_CELL_X: usize = 1; // half-blocks: 1 sub-pixel column per cell
const SUB_PIXELS_PER_CELL_Y: usize = 2; // half-blocks: 2 sub-pixel rows per cell

pub fn layout_horizontal_bar(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    _group_col: Option<&str>, // grouping handled in plan 2 — this plan keeps it single-series
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<BarLayout, LayoutError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => {
            return Err(LayoutError::NonNumericY {
                name: y_col.to_string(),
                numeric: comma_list(df.numeric_columns()),
            });
        }
    };
    if labels.is_empty() {
        return Err(LayoutError::Empty);
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

    // Margins live OUTSIDE the buffer. The composer prepends the label and
    // appends the value to each rendered cell row.
    let label_margin = agg
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let value_margin = 8;
    let plot_cells_w = canvas_cells_w
        .saturating_sub(label_margin + value_margin)
        .max(8);
    let plot_pixels_w = plot_cells_w * SUB_PIXELS_PER_CELL_X;

    // One cell per bar (with a 1-row gap between bars when there's space).
    // For half-blocks each row covers 2 sub-pixels; the bar fills both halves.
    let bar_count = agg.len();
    let row_per_bar: usize = if bar_count <= canvas_cells_h.saturating_sub(2) {
        2
    } else {
        1
    };
    let plot_cells_h = bar_count * row_per_bar;
    let plot_pixels_h = plot_cells_h * SUB_PIXELS_PER_CELL_Y;

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    let max_value = agg
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::MIN, f64::max)
        .max(1e-9);

    let mut bars = Vec::with_capacity(bar_count);
    for (i, (label, value)) in agg.into_iter().enumerate() {
        let pixel_width = ((value / max_value) * plot_pixels_w as f64).round() as usize;
        // Each bar fills one cell row vertically — both upper and lower
        // sub-pixels — so half-block rendering produces a FULL block.
        let pixel_y = i * row_per_bar * SUB_PIXELS_PER_CELL_Y;
        let pixel_height = SUB_PIXELS_PER_CELL_Y;
        bars.push(BarRect {
            label: label.clone(),
            value,
            pixel_x: 0,
            pixel_y,
            pixel_width,
            pixel_height,
            series_key: label,
        });
    }

    Ok(BarLayout {
        plot_box,
        bars,
        canvas_cells_w,
        canvas_cells_h,
        label_margin,
        value_margin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn small_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new(
                "region",
                Series::Strings(
                    vec!["NA", "EMEA", "LATAM", "APAC"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new("revenue", Series::Numbers(vec![42.0, 72.0, 21.0, 26.0])),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_bar_per_row() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        assert_eq!(layout.bars.len(), 4);
        // Each bar spans both sub-pixel rows of its cell row → full block.
        for bar in &layout.bars {
            assert_eq!(bar.pixel_height, 2);
        }
    }

    #[test]
    fn longest_bar_uses_most_of_plot_width() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        let max_bar = layout.bars.iter().max_by_key(|b| b.pixel_width).unwrap();
        // The 72 (EMEA) bar should be the widest.
        assert_eq!(max_bar.label, "EMEA");
        // Should consume close to (but not exactly) the available plot width.
        assert!(max_bar.pixel_width > layout.plot_box.pixel_width / 2);
    }

    #[test]
    fn shortest_bar_is_proportional() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        let min_bar = layout.bars.iter().min_by_key(|b| b.pixel_width).unwrap();
        let max_bar = layout.bars.iter().max_by_key(|b| b.pixel_width).unwrap();
        // 21/72 ≈ 0.29
        let ratio = min_bar.pixel_width as f64 / max_bar.pixel_width as f64;
        assert!((ratio - (21.0 / 72.0)).abs() < 0.05);
    }

    #[test]
    fn label_margin_accommodates_longest_label() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        // "LATAM" is 5 chars + 2 cells of padding.
        assert_eq!(layout.label_margin, 7);
    }

    #[test]
    fn non_numeric_y_lists_alternatives() {
        let df = DataFrame::from_columns(vec![
            Column::new("region", Series::Strings(vec!["NA".into(), "EMEA".into()])),
            Column::new("revenue", Series::Numbers(vec![10.0, 20.0])),
        ])
        .unwrap();
        let err = layout_horizontal_bar(&df, "region", "region", None, 80, 12).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must be numeric"));
        assert!(
            msg.contains("revenue"),
            "error should list numeric alternatives: {msg}"
        );
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = DataFrame::from_columns(vec![
            Column::new("region", Series::Strings(vec!["NA".into(), "EMEA".into()])),
            Column::new("revenue", Series::Numbers(vec![10.0, 20.0])),
        ])
        .unwrap();
        let err = layout_horizontal_bar(&df, "regin", "revenue", None, 80, 12).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("regin"),
            "error should name the bad column: {msg}"
        );
        assert!(
            msg.contains("region"),
            "error should suggest the closest match: {msg}"
        );
    }
}
