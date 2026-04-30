//! Ridgeline (joy-plot) layout — group data by category, evaluate KDE on a
//! shared x-grid for each group, and stack the resulting curves vertically with
//! controlled overlap so each ridge sits a fixed slot below the previous.
//!
//! Per-group normalization (each ridge's max density = full peak height) keeps
//! every ridge visible regardless of how peaky the others are. Same convention
//! as violin's polished form.

use crate::dataframe::{DataFrame, Series};
use crate::kde::kde_evaluate;
pub use crate::layout::bar::PlotBox;
use crate::stats::{FiveNumberSummary, five_number_summary};

#[derive(Debug, Clone)]
pub struct Ridge {
    pub label: String,
    pub summary: FiveNumberSummary,
    /// `heights[px]` = pixel height of the curve at column `px`, normalized to
    /// each ridge's own peak (so every ridge has the same max height in pixels).
    pub heights: Vec<usize>,
    /// Pixel-y of the ridge's baseline (where heights = 0).
    pub baseline_y: usize,
    /// Maximum vertical extent of this ridge in pixels (the "peak" allotment).
    pub max_height: usize,
    /// Sub-pixel x of the median value (for an optional median tick mark).
    pub median_x: usize,
    pub series_key: String,
}

#[derive(Debug, Clone)]
pub struct RidgelineLayout {
    pub plot_box: PlotBox,
    pub ridges: Vec<Ridge>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    pub x_min: f64,
    pub x_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum RidgelineError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X_PER_CELL: usize = 1;
const SUB_Y_PER_CELL: usize = 2;
/// Each ridge's peak height as a fraction of the per-ridge slot.
/// Lower = more overlap; higher = more separation. 1.5 means peak overshoots
/// the next ridge by 50% (classic joy-plot stacking).
const PEAK_OVERLAP: f64 = 1.5;

pub fn layout_ridgeline(
    df: &DataFrame,
    value_col: &str,
    group_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<RidgelineLayout, RidgelineError> {
    let values: Vec<f64> = match df.column(value_col)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(RidgelineError::NonNumericX(value_col.to_string())),
    };
    let groups: Vec<String> = match df.column(group_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    if values.is_empty() {
        return Err(RidgelineError::Empty);
    }

    // Group values by group, preserving first-seen order.
    let mut groups_data: Vec<(String, Vec<f64>)> = Vec::new();
    for (g, v) in groups.iter().zip(values.iter()) {
        if let Some(slot) = groups_data.iter_mut().find(|(name, _)| name == g) {
            slot.1.push(*v);
        } else {
            groups_data.push((g.clone(), vec![*v]));
        }
    }
    let summaries: Vec<(String, FiveNumberSummary, Vec<f64>)> = groups_data
        .into_iter()
        .filter_map(|(g, vs)| five_number_summary(&vs).map(|s| (g, s, vs)))
        .collect();
    if summaries.is_empty() {
        return Err(RidgelineError::Empty);
    }

    // Global x-range (data values), padded.
    let x_min_raw = summaries
        .iter()
        .map(|(_, s, _)| s.min)
        .fold(f64::INFINITY, f64::min);
    let x_max_raw = summaries
        .iter()
        .map(|(_, s, _)| s.max)
        .fold(f64::NEG_INFINITY, f64::max);
    let span_raw = (x_max_raw - x_min_raw).max(1e-9);
    let x_min = x_min_raw - span_raw * 0.05;
    let x_max = x_max_raw + span_raw * 0.05;
    let x_span = (x_max - x_min).max(1e-9);

    let n = summaries.len();
    let left_margin = 8; // group labels on the left
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(n.max(2));
    let plot_pixels_w = plot_cells_w * SUB_X_PER_CELL;
    let plot_pixels_h = plot_cells_h * SUB_Y_PER_CELL;

    // Each ridge gets a "slot" of pixel rows; slots step by slot_h, but each
    // ridge's peak can extend up to PEAK_OVERLAP × slot_h pixels above its
    // baseline. The last ridge's peak can stick up above the plot top — fine,
    // it gets clamped by the rasterizer.
    let slot_h = (plot_pixels_h / n).max(2);
    let max_peak = ((slot_h as f64) * PEAK_OVERLAP).round() as usize;

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    // X-grid: one data-x value per pixel column.
    let grid: Vec<f64> = (0..plot_pixels_w)
        .map(|px| {
            let frac = if plot_pixels_w <= 1 {
                0.0
            } else {
                px as f64 / (plot_pixels_w - 1) as f64
            };
            x_min + frac * x_span
        })
        .collect();

    let map_x = |v: f64| -> usize {
        let px = ((v - x_min) / x_span * (plot_pixels_w - 1) as f64).round() as usize;
        px.min(plot_pixels_w - 1)
    };

    let ridges: Vec<Ridge> = summaries
        .into_iter()
        .enumerate()
        .map(|(i, (label, summary, vs))| {
            let densities = kde_evaluate(&vs, &grid);
            let local_max = densities
                .iter()
                .copied()
                .fold(0.0_f64, f64::max)
                .max(1e-12);

            // Heights (pixels) per column, normalized to this ridge's own peak.
            let heights: Vec<usize> = densities
                .iter()
                .map(|&d| {
                    let raw = (d / local_max) * max_peak as f64;
                    if raw > 0.05 {
                        raw.round().max(1.0) as usize
                    } else {
                        0
                    }
                })
                .collect();

            // Baseline y for this ridge: stack from the top down.
            // Earlier ridges sit higher (smaller baseline_y).
            let baseline_y = ((i + 1) * slot_h).min(plot_pixels_h - 1);

            Ridge {
                label: label.clone(),
                summary,
                heights,
                baseline_y,
                max_height: max_peak,
                median_x: map_x(summary.median),
                series_key: label,
            }
        })
        .collect();

    Ok(RidgelineLayout {
        plot_box,
        ridges,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        x_min,
        x_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn months_df() -> DataFrame {
        // Three months of latencies — different distributions.
        DataFrame::from_columns(vec![
            Column::new(
                "month",
                Series::Strings(
                    vec![
                        "jan", "jan", "jan", "jan", "feb", "feb", "feb", "feb", "feb", "feb",
                        "mar", "mar", "mar", "mar", "mar", "mar", "mar",
                    ]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                ),
            ),
            Column::new(
                "ms",
                Series::Numbers(vec![
                    40.0, 50.0, 55.0, 60.0, 30.0, 45.0, 80.0, 100.0, 130.0, 160.0, 20.0, 25.0,
                    28.0, 30.0, 32.0, 35.0, 40.0,
                ]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_ridge_per_group_in_first_seen_order() {
        let layout = layout_ridgeline(&months_df(), "ms", "month", 80, 16).unwrap();
        assert_eq!(layout.ridges.len(), 3);
        assert_eq!(layout.ridges[0].label, "jan");
        assert_eq!(layout.ridges[1].label, "feb");
        assert_eq!(layout.ridges[2].label, "mar");
    }

    #[test]
    fn each_ridge_has_one_height_per_pixel_column() {
        let layout = layout_ridgeline(&months_df(), "ms", "month", 80, 16).unwrap();
        let pw = layout.plot_box.pixel_width;
        for r in &layout.ridges {
            assert_eq!(r.heights.len(), pw, "ridge `{}` has wrong height count", r.label);
        }
    }

    #[test]
    fn ridges_stack_with_increasing_baseline_y() {
        let layout = layout_ridgeline(&months_df(), "ms", "month", 80, 16).unwrap();
        // Earlier ridges (top of plot) have smaller baseline_y; later ones (bottom)
        // have larger baseline_y.
        for w in layout.ridges.windows(2) {
            assert!(
                w[0].baseline_y < w[1].baseline_y,
                "expected ascending baseline_y between ridges"
            );
        }
    }

    #[test]
    fn medians_match_data() {
        let layout = layout_ridgeline(&months_df(), "ms", "month", 80, 16).unwrap();
        // jan median = (50 + 55) / 2 = 52.5
        let jan = layout.ridges.iter().find(|r| r.label == "jan").unwrap();
        assert!((jan.summary.median - 52.5).abs() < 0.5);
    }
}
