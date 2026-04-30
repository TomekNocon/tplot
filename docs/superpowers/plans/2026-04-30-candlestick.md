# TerminalPlot — Plan 9: Candlestick Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add `tplot candle` to the binary — vertical OHLC (open/high/low/close) candlesticks with the standard convention: a thin vertical "wick" line covering the high-to-low range, and a filled "body" rectangle covering the open-to-close range. Body is colored green when close ≥ open (up day) and red when close < open (down day).

**Architecture:** Reuses half-blocks renderer + Bresenham line drawing. New: candlestick layout (wide-form OHLC input → per-x pixel coordinates) and rasterizer (paints wick + body per candle). No story-pass — candlesticks have a fixed visual convention; storytelling doesn't apply meaningfully.

**Tech Stack:** Same as prior plans. No new dependencies.

**Inherited context (Plans 1–7b):**
- 9 chart types working; 256 tests passing.
- Half-blocks renderer handles per-cell fg/bg colors.
- Conventional candle colors: up = green-ish, down = red-ish. Use deuteranopia-aware shades that work on both light/dark themes.

---

## File Structure

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add Candlestick variant
├── tplot-core/src/
│   ├── layout/
│   │   ├── mod.rs                  EXTEND
│   │   └── candlestick.rs          ★ NEW: per-x OHLC layout
│   └── rasterize/
│       ├── mod.rs                  EXTEND
│       └── candlestick.rs          ★ NEW: paint wick + body per candle
├── tplot/src/
│   ├── cli.rs                      EXTEND: CandleArgs (--open --high --low --close)
│   ├── commands/
│   │   ├── mod.rs                  EXTEND
│   │   ├── candlestick.rs          ★ NEW: pipeline
│   │   └── json.rs                 EXTEND: dispatch
│   └── main.rs                     EXTEND
└── tplot/tests/
    └── e2e_candlestick.rs          ★ NEW: snapshot tests
```

---

## Task 1: Add Candlestick to ChartKind

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`

- [ ] **Step 1: Write failing test (within existing tests module)**

```rust
// crates/tplot-protocol/src/chart.rs (within existing tests module)
#[test]
fn candlestick_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Candlestick {
            open:  "open".into(),
            high:  "high".into(),
            low:   "low".into(),
            close: "close".into(),
        },
        x: Axis::Column("date".into()),
        y: Axis::Column("close".into()),  // y unused for candle but kept for ChartSpec uniformity
        group: None,
        title: None,
        story: StoryConfig::default(),
    };
    let json = serde_json::to_string(&spec).unwrap();
    assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
}
```

- [ ] **Step 2: Add the variant**

```rust
// crates/tplot-protocol/src/chart.rs — modify ChartKind enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar { orientation: BarOrientation },
    Histogram { #[serde(default)] bins: Option<usize> },
    Line,
    Scatter,
    Sparkline,
    Heatmap { value: String },
    BoxPlot,
    StackedArea,
    /// OHLC candlestick chart. Names of the numeric columns for each value.
    Candlestick {
        open:  String,
        high:  String,
        low:   String,
        close: String,
    },
}
```

Add a placeholder arm to `crates/tplot/src/commands/json.rs` for `ChartKind::Candlestick { .. }` returning `Err(anyhow!("candlestick JSON dispatch lands in plan 9 task 6"))` to keep the workspace compiling.

- [ ] **Step 3: Run + verify build**

```bash
cargo test -p tplot-protocol --lib chart
cargo build --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot-protocol/ crates/tplot/src/commands/json.rs
git commit -m "Add Candlestick variant to ChartKind"
```

---

## Task 2: Candlestick layout

**Files:**
- Create: `crates/tplot-core/src/layout/candlestick.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Layout strategy:
- 5 columns required: x (categorical or numeric date), open, high, low, close (numeric).
- Per row, compute `(pixel_x, wick_top_y, wick_bottom_y, body_top_y, body_bottom_y, is_up)`.
- y-axis spans the global low/high across all rows (with a small 5% pad).
- Each candle is a column of `bar_cell_width` cells; gap between candles is `gap_cell_width`. Same lookup as vertical bar.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/candlestick.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn ohlc_df() -> DataFrame {
        // 4 days. Day 1 up, Day 2 down, Day 3 up, Day 4 doji (open == close).
        DataFrame::from_columns(vec![
            Column::new("date", Series::Strings(
                vec!["d1","d2","d3","d4"].into_iter().map(String::from).collect())),
            Column::new("open",  Series::Numbers(vec![100.0, 110.0, 105.0, 115.0])),
            Column::new("high",  Series::Numbers(vec![112.0, 113.0, 118.0, 117.0])),
            Column::new("low",   Series::Numbers(vec![ 98.0,  99.0, 104.0, 113.0])),
            Column::new("close", Series::Numbers(vec![110.0, 105.0, 115.0, 115.0])),
        ]).unwrap()
    }

    #[test]
    fn produces_one_candle_per_row() {
        let layout = layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16).unwrap();
        assert_eq!(layout.candles.len(), 4);
    }

    #[test]
    fn up_day_marked_is_up() {
        let layout = layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16).unwrap();
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
        let layout = layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16).unwrap();
        let c = &layout.candles[0];  // high=112, low=98
        // Inverted: high → small pixel_y, low → large pixel_y
        assert!(c.wick_top_y < c.wick_bottom_y);
    }

    #[test]
    fn body_within_wick_range() {
        let layout = layout_candlestick(&ohlc_df(), "date", "open", "high", "low", "close", 80, 16).unwrap();
        for c in &layout.candles {
            assert!(c.body_top_y    >= c.wick_top_y, "body top must be at or below wick top");
            assert!(c.body_bottom_y <= c.wick_bottom_y, "body bottom must be at or above wick bottom");
        }
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod candlestick;
pub use candlestick::{layout_candlestick, CandlestickLayout, Candle, CandlestickError};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::candlestick
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/candlestick.rs (above tests)
use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct Candle {
    pub label:           String,
    pub open:            f64,
    pub high:            f64,
    pub low:             f64,
    pub close:           f64,
    /// Center column of this candle, in sub-pixel coords.
    pub pixel_x:         usize,
    pub wick_top_y:      usize,
    pub wick_bottom_y:   usize,
    pub body_top_y:      usize,
    pub body_bottom_y:   usize,
    pub is_up:           bool,
}

#[derive(Debug, Clone)]
pub struct CandlestickLayout {
    pub plot_box:         PlotBox,
    pub candles:          Vec<Candle>,
    pub canvas_cells_w:   usize,
    pub canvas_cells_h:   usize,
    pub left_margin:      usize,
    pub bottom_reserve:   usize,
    pub bar_cell_width:   usize,
    pub gap_cell_width:   usize,
    pub y_min:            f64,
    pub y_max:            f64,
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
    df:             &DataFrame,
    x_col:          &str,
    open_col:       &str,
    high_col:       &str,
    low_col:        &str,
    close_col:      &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<CandlestickLayout, CandlestickError> {
    let labels: Vec<String> = match df.column(x_col)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let opens  = numeric_or_err(df, open_col)?;
    let highs  = numeric_or_err(df, high_col)?;
    let lows   = numeric_or_err(df, low_col)?;
    let closes = numeric_or_err(df, close_col)?;

    let n = labels.len();
    if n == 0 { return Err(CandlestickError::Empty); }
    if opens.len() != n || highs.len() != n || lows.len() != n || closes.len() != n {
        return Err(CandlestickError::Empty);
    }

    // Global y-range with 5% padding.
    let y_min_raw = lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max_raw = highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let span_raw  = (y_max_raw - y_min_raw).max(1e-9);
    let y_min     = y_min_raw - span_raw * 0.05;
    let y_max     = y_max_raw + span_raw * 0.05;
    let y_span    = (y_max - y_min).max(1e-9);

    let left_margin    = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w   = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h   = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_h  = plot_cells_h * SUB_Y_PER_CELL;

    let max_bar_w = plot_cells_w / n;
    if max_bar_w == 0 { return Err(CandlestickError::Empty); }
    let bar_cell_width = max_bar_w.clamp(3, 7);
    let gap_cell_width = (bar_cell_width / 3).max(1).min(max_bar_w.saturating_sub(bar_cell_width));
    let group_w        = bar_cell_width + gap_cell_width;
    let total_w        = group_w * n;
    let leading        = (plot_cells_w.saturating_sub(total_w)) / 2;

    let plot_box = PlotBox {
        pixel_width:  plot_cells_w * SUB_X_PER_CELL,
        pixel_height: plot_pixels_h,
    };

    let map_y = |v: f64| -> usize {
        let py = ((y_max - v) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        py.min(plot_pixels_h - 1)
    };

    let candles = (0..n).map(|i| {
        let pixel_x = leading + i * group_w + bar_cell_width / 2;
        let open  = opens[i];
        let close = closes[i];
        let is_up = close >= open;
        Candle {
            label:           labels[i].clone(),
            open, high: highs[i], low: lows[i], close,
            pixel_x,
            wick_top_y:      map_y(highs[i]),
            wick_bottom_y:   map_y(lows[i]),
            body_top_y:      map_y(open.max(close)),
            body_bottom_y:   map_y(open.min(close)),
            is_up,
        }
    }).collect();

    Ok(CandlestickLayout {
        plot_box, candles,
        canvas_cells_w, canvas_cells_h,
        left_margin, bottom_reserve,
        bar_cell_width, gap_cell_width,
        y_min, y_max,
    })
}

fn numeric_or_err(df: &DataFrame, col: &str) -> Result<Vec<f64>, CandlestickError> {
    match df.column(col)?.series() {
        Series::Numbers(v) => Ok(v.clone()),
        Series::Strings(_) => Err(CandlestickError::NonNumeric(col.to_string())),
    }
}
```

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add candlestick layout with per-x OHLC pixel placement"
```

---

## Task 3: Candlestick rasterizer

**Files:**
- Create: `crates/tplot-core/src/rasterize/candlestick.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

For each candle:
1. Wick: 1-px-wide vertical line at `pixel_x` from `wick_bottom_y` to `wick_top_y`.
2. Body: filled rectangle of width `bar_cell_width` centered on `pixel_x`, from `body_top_y` to `body_bottom_y`. Color = `up_color` if `is_up` else `down_color`.

Default colors (deuteranopia-friendly): up = `#3fb950` (green), down = `#f85149` (red). Both work on dark backgrounds; on light theme they shift slightly darker (handled by caller passing theme-adjusted colors).

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/candlestick.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Candle, CandlestickLayout, PlotBox};
    use tplot_protocol::RgbColor;

    const GREEN: RgbColor = RgbColor { r: 0x3f, g: 0xb9, b: 0x50 };
    const RED:   RgbColor = RgbColor { r: 0xf8, g: 0x51, b: 0x49 };

    fn fake_layout() -> CandlestickLayout {
        CandlestickLayout {
            plot_box: PlotBox { pixel_width: 20, pixel_height: 20 },
            candles: vec![
                Candle {
                    label: "d1".into(),
                    open: 100.0, high: 112.0, low: 98.0, close: 110.0,
                    pixel_x: 5,
                    wick_top_y: 0,    // pixel for high
                    wick_bottom_y: 19,// pixel for low
                    body_top_y: 5,    // pixel for max(open, close) = 110
                    body_bottom_y: 14,// pixel for min(open, close) = 100
                    is_up: true,
                },
                Candle {
                    label: "d2".into(),
                    open: 110.0, high: 113.0, low: 99.0, close: 105.0,
                    pixel_x: 15,
                    wick_top_y: 1,
                    wick_bottom_y: 18,
                    body_top_y: 6,
                    body_bottom_y: 12,
                    is_up: false,
                },
            ],
            canvas_cells_w: 30, canvas_cells_h: 12,
            left_margin: 6, bottom_reserve: 3,
            bar_cell_width: 5, gap_cell_width: 2,
            y_min: 95.0, y_max: 115.0,
        }
    }

    #[test]
    fn up_candle_body_painted_in_green() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Up candle body spans (3..=7, 5..=14).
        assert_eq!(buf.get(5, 7), Some(GREEN));
        assert_eq!(buf.get(3, 10), Some(GREEN));
    }

    #[test]
    fn down_candle_body_painted_in_red() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Down candle body spans (13..=17, 6..=12).
        assert_eq!(buf.get(15, 9), Some(RED));
        assert_eq!(buf.get(13, 12), Some(RED));
    }

    #[test]
    fn wick_extends_above_and_below_body() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        rasterize_candlestick(&fake_layout(), GREEN, RED, &mut buf);
        // Wick of up candle: pixel_x=5, from y=0 to y=19.
        // Above body (y < 5) and below body (y > 14) should be GREEN at x=5.
        assert_eq!(buf.get(5, 0),  Some(GREEN));
        assert_eq!(buf.get(5, 19), Some(GREEN));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod candlestick;
pub use candlestick::rasterize_candlestick;
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/candlestick.rs (above tests)
use crate::layout::CandlestickLayout;
use crate::PixelBuffer;
use tplot_protocol::RgbColor;

pub fn rasterize_candlestick(
    layout:     &CandlestickLayout,
    up_color:   RgbColor,
    down_color: RgbColor,
    buf:        &mut PixelBuffer,
) {
    for c in &layout.candles {
        let color = if c.is_up { up_color } else { down_color };

        // 1. Wick: vertical 1-pixel line at pixel_x.
        buf.draw_line(c.pixel_x, c.wick_top_y, c.pixel_x, c.wick_bottom_y, color);

        // 2. Body: filled rectangle centered on pixel_x.
        let half = layout.bar_cell_width / 2;
        let x0 = c.pixel_x.saturating_sub(half);
        let x1 = c.pixel_x + half;
        buf.fill_rect(x0, c.body_top_y, x1, c.body_bottom_y, color);
    }
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot-core --lib rasterize::candlestick
git add crates/tplot-core/
git commit -m "Add candlestick rasterizer with wick + colored body"
```

---

## Task 4: CLI subcommand

**Files:**
- Modify: `crates/tplot/src/cli.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_candle_subcommand() {
    let args = Cli::parse_from([
        "tplot", "candle", "stocks.csv",
        "-x", "date",
        "--open",  "o",
        "--high",  "h",
        "--low",   "l",
        "--close", "c",
    ]);
    match args.command {
        Command::Candle(c) => {
            assert_eq!(c.input, "stocks.csv");
            assert_eq!(c.x, "date");
            assert_eq!(c.open,  "o");
            assert_eq!(c.high,  "h");
            assert_eq!(c.low,   "l");
            assert_eq!(c.close, "c");
        }
        _ => panic!("expected Candle"),
    }
}
```

- [ ] **Step 2: Add the variant**

```rust
// crates/tplot/src/cli.rs — modify Command enum, add CandleArgs
#[derive(Subcommand, Debug)]
pub enum Command {
    Bar(BarArgs),
    Hist(HistArgs),
    Line(LineArgs),
    Scatter(ScatterArgs),
    Spark(SparkArgs),
    Heatmap(HeatmapArgs),
    Box(BoxArgs),
    Area(AreaArgs),
    /// OHLC candlestick chart. Requires four numeric columns: open, high, low, close.
    Candle(CandleArgs),
    Doctor,
    Json,
}

#[derive(Args, Debug)]
pub struct CandleArgs {
    pub input: String,
    /// X-axis column (typically date or index).
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column with opening price.
    #[arg(long)]
    pub open:  String,
    /// Numeric column with intraday high.
    #[arg(long)]
    pub high:  String,
    /// Numeric column with intraday low.
    #[arg(long)]
    pub low:   String,
    /// Numeric column with closing price.
    #[arg(long)]
    pub close: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test -p tplot --lib cli
git add crates/tplot/
git commit -m "Add candle subcommand to CLI argument parser"
```

---

## Task 5: Candlestick pipeline

**Files:**
- Create: `crates/tplot/src/commands/candlestick.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/candlestick.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_candlestick_with_up_and_down_days() {
        let csv = "date,o,h,l,c\nd1,100,112,98,110\nd2,110,113,99,105\nd3,105,118,104,115\nd4,115,117,113,115\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = CandleOptions {
            x: "date".into(),
            open: "o".into(), high: "h".into(), low: "l".into(), close: "c".into(),
            graphics: "none".into(),
            width: Some(80), height: 16,
        };
        let out = render_candlestick(&df, &opts).unwrap();
        // d1 (up: 100→110): green color escape
        assert!(out.contains("\x1b[38;2;63;185;80m"), "missing green up-color");
        // d2 (down: 110→105): red color escape
        assert!(out.contains("\x1b[38;2;248;81;73m"), "missing red down-color");
        // Day labels appear under the chart.
        assert!(out.contains("d1"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod candlestick;
pub use candlestick::{render_candlestick, CandleOptions};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/candlestick.rs (above tests)
use crate::pipeline::{detected_terminal_size, require_minimum_width, resolve_graphics};
use anyhow::{anyhow, Result};
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_candlestick;
use tplot_core::rasterize::rasterize_candlestick;
use tplot_core::PixelBuffer;
use tplot_render::render_halfblocks;
use tplot_protocol::{Capabilities, GraphicsProtocol, RgbColor};

const UP_COLOR:   RgbColor = RgbColor { r: 0x3f, g: 0xb9, b: 0x50 };
const DOWN_COLOR: RgbColor = RgbColor { r: 0xf8, g: 0x51, b: 0x49 };

#[derive(Debug, Clone)]
pub struct CandleOptions {
    pub x:        String,
    pub open:     String,
    pub high:     String,
    pub low:      String,
    pub close:    String,
    pub graphics: String,
    pub width:    Option<usize>,
    pub height:   usize,
}

pub fn render_candlestick(df: &DataFrame, opts: &CandleOptions) -> Result<String> {
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_candlestick(
        df, &opts.x, &opts.open, &opts.high, &opts.low, &opts.close,
        canvas_w, canvas_h,
    ).map_err(|e| anyhow!(e.to_string()))?;

    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_candlestick(&layout, UP_COLOR, DOWN_COLOR, &mut buf);

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let protocol = resolve_graphics(&opts.graphics, caps);

    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path: y-axis labels + chart + x-axis labels.
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();
    let n_rows = body_lines.len();

    let mut out = String::new();
    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", layout.y_max)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", (layout.y_min + layout.y_max) / 2.0)
        } else if i + 1 == n_rows {
            format!("{:>5.0}", layout.y_min)
        } else {
            " ".repeat(5)
        };
        out.push_str(&y_label);
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }

    // X-axis labels — one per candle, centered.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout.candles.first()
        .map(|c| c.pixel_x.saturating_sub(layout.bar_cell_width / 2)).unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    let group_w = layout.bar_cell_width + layout.gap_cell_width;
    for (idx, c) in layout.candles.iter().enumerate() {
        let trimmed: String = c.label.chars().take(group_w).collect();
        let pad_left  = layout.bar_cell_width.saturating_sub(trimmed.chars().count()) / 2;
        let pad_right = layout.bar_cell_width.saturating_sub(trimmed.chars().count() + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.candles.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    Ok(out)
}

#[allow(dead_code)]
fn _typecheck(_: usize) -> Option<usize> { detected_terminal_size(None).0.into() }
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot --lib commands::candlestick
git add crates/tplot/
git commit -m "Add candlestick pipeline with up/down body coloring"
```

---

## Task 6: Wire main.rs and JSON dispatch

**Files:**
- Modify: `crates/tplot/src/main.rs`
- Modify: `crates/tplot/src/commands/json.rs`

- [ ] **Step 1: Wire main.rs**

```rust
// crates/tplot/src/main.rs — add Candle match arm
Command::Candle(c) => {
    let df = pipeline::read_dataframe(&c.input)?;
    let (_, height) = pipeline::detected_terminal_size(c.common.width);
    let out = commands::render_candlestick(&df, &commands::CandleOptions {
        x: c.x,
        open: c.open, high: c.high, low: c.low, close: c.close,
        graphics: c.common.graphics,
        width: c.common.width, height,
    })?;
    print!("{out}");
    Ok(())
}
```

- [ ] **Step 2: Replace placeholder in json.rs**

```rust
// crates/tplot/src/commands/json.rs — replace the Candlestick placeholder arm
ChartKind::Candlestick { open, high, low, close } => {
    let x = match spec.x {
        Axis::Column(c) => c,
        _ => return Err(anyhow!("inline x axis not supported in v1")),
    };
    let opts = CandleOptions {
        x,
        open, high, low, close,
        graphics: "none".into(),
        width:    Some(canvas_w),
        height:   canvas_h,
    };
    crate::commands::render_candlestick(&parsed.dataframe, &opts)
}
```

Add: `use crate::commands::CandleOptions;`

- [ ] **Step 3: Run + smoke test**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

echo "date,o,h,l,c
d1,100,112,98,110
d2,110,113,99,105
d3,105,118,104,115
d4,115,117,113,115
d5,115,121,110,118
d6,118,119,114,116
d7,116,125,116,124
d8,124,127,120,121" | cargo run -p tplot -- candle - -x date --open o --high h --low l --close c
```

Expected: 8 candles side-by-side, up days (d1, d3, d4, d5, d7) in green, down days (d2, d6, d8) in red, wicks extending above and below each body. Day labels under the chart.

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Wire candlestick into main and JSON dispatch"
```

---

## Task 7: Snapshot tests + README + lint pass

**Files:**
- Create: `crates/tplot/tests/e2e_candlestick.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the snapshot test**

```rust
// crates/tplot/tests/e2e_candlestick.rs
use std::io::Write;
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap().to_path_buf()
}

fn run_with_stdin(args: &[&str], input: &str) -> String {
    let mut child = Command::new(binary_path())
        .args(args).current_dir(workspace_root())
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().expect("tplot binary failed");
    child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().expect("wait failed");
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c2 in chars.by_ref() { if c2.is_ascii_alphabetic() { break; } }
        } else { out.push(c); }
    }
    out
}

#[test]
fn candlestick_8_days_snapshot() {
    let csv = "date,o,h,l,c\n\
        d1,100,112,98,110\nd2,110,113,99,105\nd3,105,118,104,115\nd4,115,117,113,115\n\
        d5,115,121,110,118\nd6,118,119,114,116\nd7,116,125,116,124\nd8,124,127,120,121\n";
    let out = run_with_stdin(&[
        "candle", "-",
        "-x", "date",
        "--open", "o", "--high", "h", "--low", "l", "--close", "c",
        "--width", "80",
    ], csv);
    insta::assert_snapshot!("candlestick_8_days", strip_ansi(&out));
}
```

- [ ] **Step 2: Run, accept**

```bash
cargo test -p tplot --test e2e_candlestick
cargo insta accept
cargo test -p tplot --test e2e_candlestick
```

- [ ] **Step 3: Update README** — add candlestick to chart-types list and quickstart:

```markdown
```bash
# OHLC candlestick chart from a stocks-style CSV
echo "date,o,h,l,c
d1,100,112,98,110
d2,110,113,99,105
d3,105,118,104,115
d4,115,117,113,115" | tplot candle - -x date --open o --high h --low l --close c
```
```

- [ ] **Step 4: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/tplot/tests/ README.md
git add -A
git commit -m "Add e2e snapshot test for candlestick, update README, lint pass"
```

---

## Done — Plan 9 deliverable

```bash
echo "date,o,h,l,c
d1,100,112,98,110
d2,110,113,99,105
d3,105,118,104,115
d4,115,117,113,115" | tplot candle - -x date --open o --high h --low l --close c
```

Up days in green, down days in red, wicks above and below each body.

**10 chart types.**

## Self-review notes

- ChartKind::Candlestick (Task 1) ✓
- Layout (Task 2) ✓
- Rasterizer (Task 3) ✓
- CLI (Task 4) ✓
- Pipeline (Task 5) ✓
- Main + JSON wiring (Task 6) ✓
- Snapshot tests + README + lint (Task 7) ✓
