//! Candlestick layout — wide-form OHLC input → per-row pixel coordinates for
//! wick (high → low) and body (open → close), inverted pixel-y space.

use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct Candle {
    pub label: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Center column of this candle, in sub-pixel coords.
    pub pixel_x: usize,
    pub wick_top_y: usize,
    pub wick_bottom_y: usize,
    pub body_top_y: usize,
    pub body_bottom_y: usize,
    pub is_up: bool,
}

#[derive(Debug, Clone)]
pub struct CandlestickLayout {
    pub plot_box: PlotBox,
    pub candles: Vec<Candle>,
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
pub enum CandlestickError {
    #[error("column `{0}` must be numeric")]
    NonNumeric(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}

const SUB_X_PER_CELL: usize = 1;
const SUB_Y_PER_CELL: usize = 2;
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_candlestick(
    df: &DataFrame,
    x_col: &str,
    open_col: &str,
    high_col: &str,
    low_col: &str,
    close_col: &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<CandlestickLayout, CandlestickError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let opens = numeric_or_err(df, open_col)?;
    let highs = numeric_or_err(df, high_col)?;
    let lows = numeric_or_err(df, low_col)?;
    let closes = numeric_or_err(df, close_col)?;

    let n = labels.len();
    if n == 0 {
        return Err(CandlestickError::Empty);
    }
    if opens.len() != n || highs.len() != n || lows.len() != n || closes.len() != n {
        return Err(CandlestickError::Empty);
    }

    // Global y-range with 5% padding.
    let y_min_raw = lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max_raw = highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let span_raw = (y_max_raw - y_min_raw).max(1e-9);
    let y_min = y_min_raw - span_raw * 0.05;
    let y_max = y_max_raw + span_raw * 0.05;
    let y_span = (y_max - y_min).max(1e-9);

    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_h = plot_cells_h * SUB_Y_PER_CELL;

    let max_bar_w = plot_cells_w / n;
    if max_bar_w == 0 {
        return Err(CandlestickError::Empty);
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

    let candles = (0..n)
        .map(|i| {
            let pixel_x = leading + i * group_w + bar_cell_width / 2;
            let open = opens[i];
            let close = closes[i];
            let is_up = close >= open;
            Candle {
                label: labels[i].clone(),
                open,
                high: highs[i],
                low: lows[i],
                close,
                pixel_x,
                wick_top_y: map_y(highs[i]),
                wick_bottom_y: map_y(lows[i]),
                body_top_y: map_y(open.max(close)),
                body_bottom_y: map_y(open.min(close)),
                is_up,
            }
        })
        .collect();

    Ok(CandlestickLayout {
        plot_box,
        candles,
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

fn numeric_or_err(df: &DataFrame, col: &str) -> Result<Vec<f64>, CandlestickError> {
    match df.column(col)?.series() {
        Series::Numbers(v) => Ok(v.clone()),
        Series::Strings(_) => Err(CandlestickError::NonNumeric(col.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn ohlc_df() -> DataFrame {
        // 4 days. Day 1 up, Day 2 down, Day 3 up, Day 4 doji (open == close).
        DataFrame::from_columns(vec![
            Column::new(
                "date",
                Series::Strings(
                    vec!["d1", "d2", "d3", "d4"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new("open", Series::Numbers(vec![100.0, 110.0, 105.0, 115.0])),
            Column::new("high", Series::Numbers(vec![112.0, 113.0, 118.0, 117.0])),
            Column::new("low", Series::Numbers(vec![98.0, 99.0, 104.0, 113.0])),
            Column::new("close", Series::Numbers(vec![110.0, 105.0, 115.0, 115.0])),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_candle_per_row() {
        let layout =
            layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16)
                .unwrap();
        assert_eq!(layout.candles.len(), 4);
    }

    #[test]
    fn up_day_marked_is_up() {
        let layout =
            layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16)
                .unwrap();
        // Day 1: open=100, close=110 → up
        assert!(layout.candles[0].is_up);
        // Day 2: open=110, close=105 → down
        assert!(!layout.candles[1].is_up);
        // Day 3: open=105, close=115 → up
        assert!(layout.candles[2].is_up);
        // Day 4: open=115, close=115 → "doji" (treated as up for color, configurable later)
        assert!(layout.candles[3].is_up);
    }

    #[test]
    fn wick_spans_high_to_low_inverted() {
        let layout =
            layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16)
                .unwrap();
        let c = &layout.candles[0]; // high=112, low=98
        // Inverted: high → small pixel_y, low → large pixel_y
        assert!(c.wick_top_y < c.wick_bottom_y);
    }

    #[test]
    fn body_within_wick_range() {
        let layout =
            layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16)
                .unwrap();
        for c in &layout.candles {
            assert!(
                c.body_top_y >= c.wick_top_y,
                "body top must be at or below wick top"
            );
            assert!(
                c.body_bottom_y <= c.wick_bottom_y,
                "body bottom must be at or above wick bottom"
            );
        }
    }
}
