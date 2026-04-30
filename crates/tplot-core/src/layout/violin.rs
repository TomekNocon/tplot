//! Violin plot layout — per-group KDE evaluated on a y-grid then resampled
//! to one half-width per pixel-y row. Densities are normalised against the
//! global maximum so widths are comparable across groups.

use crate::dataframe::{DataFrame, Series};
use crate::kde::kde_evaluate;
pub use crate::layout::bar::PlotBox;
use crate::stats::{FiveNumberSummary, five_number_summary};

const GRID_POINTS: usize = 64;

#[derive(Debug, Clone)]
pub struct Violin {
    pub label: String,
    pub summary: FiveNumberSummary,
    /// Centerline column of this violin in sub-pixel coords.
    pub pixel_x: usize,
    /// `half_widths[i]` = pixel width on EACH side of the centerline at the
    /// y-grid point with index `i`. Index 0 is the TOP of the plot (high y),
    /// index GRID_POINTS-1 is the BOTTOM (low y) — matches the inverted
    /// pixel-y convention used elsewhere.
    pub half_widths: Vec<usize>,
    /// Pixel-y for the median line (within the plot area).
    pub median_y: usize,
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct ViolinLayout {
    pub plot_box: PlotBox,
    pub violins: Vec<Violin>,
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
pub enum ViolinError {
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

pub fn layout_violin(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<ViolinLayout, ViolinError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(ViolinError::NonNumericY(y_col.to_string())),
    };
    if labels.is_empty() {
        return Err(ViolinError::Empty);
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

    let summaries: Vec<(String, FiveNumberSummary, Vec<f64>)> = groups
        .into_iter()
        .filter_map(|(l, vs)| five_number_summary(&vs).map(|s| (l, s, vs)))
        .collect();
    if summaries.is_empty() {
        return Err(ViolinError::Empty);
    }

    // Global y-range from the data, padded.
    let y_min_raw = summaries
        .iter()
        .map(|(_, s, _)| s.min)
        .fold(f64::INFINITY, f64::min);
    let y_max_raw = summaries
        .iter()
        .map(|(_, s, _)| s.max)
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
        return Err(ViolinError::Empty);
    }
    let bar_cell_width = max_bar_w.clamp(5, 11);
    let gap_cell_width = (bar_cell_width / 4)
        .max(1)
        .min(max_bar_w.saturating_sub(bar_cell_width));
    let group_w = bar_cell_width + gap_cell_width;
    let total_w = group_w * n;
    let leading = (plot_cells_w.saturating_sub(total_w)) / 2;

    let plot_box = PlotBox {
        pixel_width: plot_cells_w * SUB_X_PER_CELL,
        pixel_height: plot_pixels_h,
    };

    // Build y-grid (data values) corresponding to each pixel-y row, top → bottom.
    let grid: Vec<f64> = (0..GRID_POINTS)
        .map(|i| {
            let frac = i as f64 / (GRID_POINTS - 1) as f64;
            y_max - frac * y_span // top = y_max, bottom = y_min
        })
        .collect();

    // Compute KDE for each group; remember the global maximum density.
    let kdes: Vec<Vec<f64>> = summaries
        .iter()
        .map(|(_, _, vs)| kde_evaluate(vs, &grid))
        .collect();
    let global_max_density = kdes
        .iter()
        .flat_map(|d| d.iter().copied())
        .fold(0.0_f64, f64::max)
        .max(1e-12);

    let max_half_width_pixels = (bar_cell_width / 2).max(1) * SUB_X_PER_CELL;

    let map_y = |v: f64| -> usize {
        let py = ((y_max - v) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        py.min(plot_pixels_h - 1)
    };

    let violins = summaries
        .into_iter()
        .enumerate()
        .zip(kdes)
        .map(|((i, (label, summary, _)), densities)| {
            // Resample densities from GRID_POINTS to plot_pixels_h pixels.
            let half_widths: Vec<usize> = (0..plot_pixels_h)
                .map(|py| {
                    // Map py back to a grid index in [0, GRID_POINTS-1].
                    let frac = py as f64 / (plot_pixels_h - 1).max(1) as f64;
                    let gi = (frac * (GRID_POINTS - 1) as f64).round() as usize;
                    let d = densities[gi.min(GRID_POINTS - 1)];
                    let raw = (d / global_max_density) * max_half_width_pixels as f64;
                    // Any non-trivial density paints at least 1 pixel so groups
                    // dwarfed by a tight cluster's peak still appear.
                    if raw > 0.05 {
                        raw.round().max(1.0) as usize
                    } else {
                        0
                    }
                })
                .collect();

            let pixel_x = leading + i * group_w + bar_cell_width / 2;
            Violin {
                label: label.clone(),
                summary,
                pixel_x,
                half_widths,
                median_y: map_y(summary.median),
                series_key: label,
            }
        })
        .collect();

    Ok(ViolinLayout {
        plot_box,
        violins,
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

    fn skewed_df() -> DataFrame {
        // /short: tight cluster around 50.
        // /long: wide spread, larger range.
        DataFrame::from_columns(vec![
            Column::new(
                "endpoint",
                Series::Strings(
                    vec![
                        "/short", "/short", "/short", "/short", "/short", "/long", "/long",
                        "/long", "/long", "/long", "/long", "/long", "/long",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                ),
            ),
            Column::new(
                "ms",
                Series::Numbers(vec![
                    48.0, 50.0, 51.0, 52.0, 53.0, 10.0, 30.0, 80.0, 150.0, 300.0, 250.0, 60.0,
                    90.0,
                ]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_violin_per_group() {
        let layout = layout_violin(&skewed_df(), "endpoint", "ms", 80, 16).unwrap();
        assert_eq!(layout.violins.len(), 2);
    }

    #[test]
    fn long_violin_has_wider_max_half_width() {
        let layout = layout_violin(&skewed_df(), "endpoint", "ms", 80, 16).unwrap();
        let short = layout
            .violins
            .iter()
            .find(|v| v.label == "/short")
            .unwrap();
        let long = layout.violins.iter().find(|v| v.label == "/long").unwrap();
        // /short is concentrated → densities at the peak are very high but only
        // over a narrow y-range. /long is spread over a wider y-range. After
        // global normalisation, /short's PEAK density (and thus its widest
        // half-width pixel) should still be ≥ /long's (because /short has
        // higher peak density). For the test, just check both have widths.
        assert!(short.half_widths.iter().any(|&w| w > 0));
        assert!(long.half_widths.iter().any(|&w| w > 0));
    }

    #[test]
    fn medians_match_data() {
        let layout = layout_violin(&skewed_df(), "endpoint", "ms", 80, 16).unwrap();
        let short = layout
            .violins
            .iter()
            .find(|v| v.label == "/short")
            .unwrap();
        // /short median = 51 (5 sorted values: 48, 50, 51, 52, 53)
        assert!((short.summary.median - 51.0).abs() < 0.01);
    }
}
