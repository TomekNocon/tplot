use crate::dataframe::{Column, DataFrame, Series, comma_list};
use crate::layout::vertical_bar::{VerticalBarLayout, VerticalBarLayoutError, layout_vertical_bar};

#[derive(Debug, Clone)]
pub struct HistogramLayout {
    pub bars: VerticalBarLayout,
    pub bin_count: usize,
    pub bin_width: f64,
    pub data_min: f64,
    pub data_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum HistogramError {
    #[error("column `{name}` must be numeric (numeric columns: {numeric})")]
    NonNumeric { name: String, numeric: String },
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    Layout(#[from] VerticalBarLayoutError),
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

pub fn layout_histogram(
    df: &DataFrame,
    value_col: &str,
    bins: Option<usize>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<HistogramLayout, HistogramError> {
    let values: Vec<f64> = match df.column(value_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => {
            return Err(HistogramError::NonNumeric {
                name: value_col.to_string(),
                numeric: comma_list(df.numeric_columns()),
            });
        }
    };
    if values.is_empty() {
        return Err(HistogramError::Empty);
    }

    let n = values.len();
    let bin_count = bins.unwrap_or_else(|| sturges(n)).max(2);

    let data_min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let data_max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let range = (data_max - data_min).max(1e-9);
    let bin_width = range / bin_count as f64;

    // Count items per bin.
    let mut counts = vec![0u64; bin_count];
    for v in &values {
        let mut idx = ((v - data_min) / bin_width).floor() as usize;
        if idx >= bin_count {
            idx = bin_count - 1;
        } // pin upper boundary
        counts[idx] += 1;
    }

    // Build a synthetic DataFrame: one row per bin, label = "lo–hi", value = count.
    let labels: Vec<String> = (0..bin_count)
        .map(|i| {
            let lo = data_min + i as f64 * bin_width;
            let hi = data_min + (i + 1) as f64 * bin_width;
            format!("{lo:.0}–{hi:.0}")
        })
        .collect();
    let count_floats: Vec<f64> = counts.iter().map(|c| *c as f64).collect();

    let bar_df = DataFrame::from_columns(vec![
        Column::new("__bin__", Series::Strings(labels)),
        Column::new("__count__", Series::Numbers(count_floats)),
    ])?;

    let bars = layout_vertical_bar(
        &bar_df,
        "__bin__",
        "__count__",
        None,
        canvas_cells_w,
        canvas_cells_h,
    )?;

    Ok(HistogramLayout {
        bars,
        bin_count,
        bin_width,
        data_min,
        data_max,
    })
}

/// Sturges' rule: ceil(log2(N) + 1). Standard textbook default.
fn sturges(n: usize) -> usize {
    if n < 2 {
        return 1;
    }
    ((n as f64).log2() + 1.0).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn latency_df() -> DataFrame {
        // Synthetic latency distribution with a clear peak around 50ms.
        let values: Vec<f64> = vec![
            10.0, 22.0, 35.0, 41.0, 48.0, 49.0, 50.0, 50.0, 51.0, 52.0, 55.0, 58.0, 60.0, 65.0,
            80.0, 95.0, 110.0, 145.0, 220.0,
        ];
        DataFrame::from_columns(vec![Column::new("ms", Series::Numbers(values))]).unwrap()
    }

    #[test]
    fn auto_bin_count_uses_sturges() {
        // N=19 → bins = ceil(log2(19)+1) = ceil(4.25+1) = 6
        let layout = layout_histogram(&latency_df(), "ms", None, 80, 16).unwrap();
        assert_eq!(layout.bin_count, 6);
        assert_eq!(layout.bars.bars.len(), 6);
    }

    #[test]
    fn explicit_bin_count_honored() {
        let layout = layout_histogram(&latency_df(), "ms", Some(10), 80, 16).unwrap();
        assert_eq!(layout.bin_count, 10);
    }

    #[test]
    fn modal_bin_has_highest_count() {
        // The 40-60ms range has the most observations; whichever bin covers
        // that range should be the tallest (= highest count).
        let layout = layout_histogram(&latency_df(), "ms", Some(7), 80, 16).unwrap();
        let modal = layout
            .bars
            .bars
            .iter()
            .max_by_key(|b| b.pixel_height)
            .unwrap();
        assert!(
            modal.label.contains("4") || modal.label.contains("5") || modal.label.contains("6"),
            "modal bin label was {:?}",
            modal.label
        );
    }

    #[test]
    fn bin_labels_are_ranges() {
        let layout = layout_histogram(&latency_df(), "ms", Some(5), 80, 16).unwrap();
        for bar in &layout.bars.bars {
            // Labels look like "10–52" (range form).
            assert!(
                bar.label.contains('–') || bar.label.contains('-'),
                "label {:?} should be a range",
                bar.label
            );
        }
    }

    #[test]
    fn non_numeric_value_lists_alternatives() {
        let df = DataFrame::from_columns(vec![
            Column::new(
                "label",
                Series::Strings(vec!["A".into(), "B".into()]),
            ),
            Column::new("count", Series::Numbers(vec![1.0, 2.0])),
        ])
        .unwrap();
        let err = layout_histogram(&df, "label", None, 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must be numeric"));
        assert!(msg.contains("count"), "error should list numeric alternatives: {msg}");
    }

    #[test]
    fn missing_column_returns_did_you_mean() {
        let df = latency_df();
        let err = layout_histogram(&df, "mz", None, 80, 16).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("mz"), "error should name the bad column: {msg}");
        assert!(msg.contains("ms"), "error should suggest the closest match: {msg}");
    }
}
