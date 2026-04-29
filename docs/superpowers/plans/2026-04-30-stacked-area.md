# TerminalPlot — Plan 5.5: Stacked Area Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tplot area` to the binary — multi-series cumulative-fill chart where each series stacks on top of the previous, showing how a total decomposes over time. Story-pass picks the series with the largest total contribution as focal.

**Architecture:** Reuses the half-blocks renderer (per-cell fg/bg colors) and Bresenham for outlines. New: per-pixel-column interpolation that walks the buffer width and fills each pixel column from the cumulative baseline up to the series's cumulative top. The result is smooth between data points without any polygon math.

**Tech Stack:** Same as prior plans. No new dependencies.

**Inherited context (Plans 1–5):**
- 8 chart types working; 160 tests passing.
- Subagents have caught a number of plan-bugs in earlier rounds (test data near boundaries, off-by-one math, missing trait derives, even-length median). Stay alert.

---

## File Structure

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add StackedArea variant
├── tplot-core/src/
│   ├── layout/
│   │   ├── mod.rs                  EXTEND
│   │   └── stacked_area.rs         ★ NEW
│   └── rasterize/
│       ├── mod.rs                  EXTEND
│       └── stacked_area.rs         ★ NEW
├── tplot-story/src/
│   ├── focal.rs                    EXTEND: pick_focal_by_total (reusable)
│   ├── takeaway.rs                 EXTEND: stacked_area_takeaway
│   └── lib.rs                      EXTEND: run_stacked_area_story_pass
├── tplot/src/
│   ├── cli.rs                      EXTEND: AreaArgs
│   ├── commands/
│   │   ├── mod.rs                  EXTEND
│   │   ├── stacked_area.rs         ★ NEW
│   │   └── json.rs                 EXTEND
│   └── main.rs                     EXTEND
└── tplot/tests/
    └── e2e_stacked_area.rs         ★ NEW
```

---

## Task 1: Add StackedArea to ChartKind

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-protocol/src/chart.rs (within existing tests module)
#[test]
fn stacked_area_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::StackedArea,
        x: Axis::Column("month".into()),
        y: Axis::Column("revenue".into()),
        group: Some("region".into()),
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
    /// Stacked area chart. Long-form input grouped by `group`; each group
    /// stacks on top of the previous (in first-seen order). Y is the cumulative
    /// total across all groups.
    StackedArea,
}
```

Add a placeholder arm to `crates/tplot/src/commands/json.rs` for `ChartKind::StackedArea` returning `Err(anyhow!("stacked-area JSON dispatch lands in plan 5.5 task 7"))`.

- [ ] **Step 3: Run + verify workspace builds**

```bash
cargo test -p tplot-protocol --lib chart
cargo build --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot-protocol/ crates/tplot/src/commands/json.rs
git commit -m "Add StackedArea variant to ChartKind"
```

---

## Task 2: Stacked area layout

**Files:**
- Create: `crates/tplot-core/src/layout/stacked_area.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Layout produces:
- Sorted unique x values (numeric).
- For each series (in first-seen order): a vector of `(x_value, value)` pairs aligned to the x grid (missing entries = 0).
- Per-x-position cumulative tops in pixel coordinates: `cumulative_pixel_y[series_idx][x_idx]` (inverted: high data = low pixel).
- The TOTAL (top of stack) at each x position determines y_max.

```rust
// crates/tplot-core/src/layout/stacked_area.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn revenue_df() -> DataFrame {
        // 3 months × 2 regions. NA grows; EMEA stays flat-ish.
        DataFrame::from_columns(vec![
            Column::new("month", Series::Numbers(vec![1.0,1.0, 2.0,2.0, 3.0,3.0])),
            Column::new("rev",   Series::Numbers(vec![10.0,5.0, 20.0,5.0, 40.0,6.0])),
            Column::new("g",     Series::Strings(
                vec!["NA","EMEA","NA","EMEA","NA","EMEA"]
                    .into_iter().map(String::from).collect())),
        ]).unwrap()
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
        let na   = layout.series.iter().find(|s| s.key == "NA").unwrap();
        let emea = layout.series.iter().find(|s| s.key == "EMEA").unwrap();
        assert!((na.total - 70.0).abs() < 1e-6);
        assert!((emea.total - 16.0).abs() < 1e-6);
    }
}
```

- [ ] **Step 1: Write the tests above.**

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod stacked_area;
pub use stacked_area::{layout_stacked_area, StackedAreaLayout, StackedAreaSeries, StackedAreaError};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/stacked_area.rs (above tests)
use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct StackedAreaSeries {
    pub key:   String,
    /// Aligned to the layout's `x_values` grid; missing entries = 0.
    pub y_values: Vec<f64>,
    /// Sum across all x positions (used for the focal heuristic).
    pub total: f64,
}

#[derive(Debug, Clone)]
pub struct StackedAreaLayout {
    pub plot_box:        PlotBox,
    pub series:          Vec<StackedAreaSeries>,
    /// Sorted unique x values (data space).
    pub x_values:        Vec<f64>,
    /// Number of unique x positions.
    pub x_count:         usize,
    /// Total stack height per x position (sum of all series at that x).
    pub totals_at_x:     Vec<f64>,
    pub canvas_cells_w:  usize,
    pub canvas_cells_h:  usize,
    pub left_margin:     usize,
    pub bottom_reserve:  usize,
    pub x_min:           f64,
    pub x_max:           f64,
    pub y_max:           f64,
}

#[derive(Debug, thiserror::Error)]
pub enum StackedAreaError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
}

const SUB_X_PER_CELL: usize = 1;
const SUB_Y_PER_CELL: usize = 2;
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_stacked_area(
    df:             &DataFrame,
    x_col:          &str,
    y_col:          &str,
    group_col:      &str,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<StackedAreaLayout, StackedAreaError> {
    let xs: Vec<f64> = match df.column(x_col).map_err(|_| StackedAreaError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(StackedAreaError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df.column(y_col).map_err(|_| StackedAreaError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(StackedAreaError::NonNumericY(y_col.to_string())),
    };
    let groups: Vec<String> = match df.column(group_col).map_err(|_| StackedAreaError::Empty)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    if xs.is_empty() { return Err(StackedAreaError::Empty); }

    // Discover unique sorted x values.
    let mut x_values: Vec<f64> = xs.clone();
    x_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    x_values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let x_count = x_values.len();

    // Discover unique series keys in first-seen order.
    let mut series_keys: Vec<String> = Vec::new();
    for g in &groups {
        if !series_keys.iter().any(|k| k == g) { series_keys.push(g.clone()); }
    }

    // Build per-series y_values aligned to the x_values grid (sum on duplicates).
    let mut series: Vec<StackedAreaSeries> = series_keys.into_iter().map(|key| {
        StackedAreaSeries { key, y_values: vec![0.0; x_count], total: 0.0 }
    }).collect();

    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let xi = x_values.iter().position(|v| (v - x).abs() < 1e-9).unwrap();
        let s  = series.iter_mut().find(|s| s.key == *g).unwrap();
        s.y_values[xi] += y;
        s.total += y;
    }

    let totals_at_x: Vec<f64> = (0..x_count).map(|xi| {
        series.iter().map(|s| s.y_values[xi]).sum()
    }).collect();

    let x_min  = *x_values.first().unwrap();
    let x_max  = *x_values.last().unwrap();
    let y_max  = totals_at_x.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)).max(1e-9);

    let left_margin    = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w   = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h   = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w  = plot_cells_w * SUB_X_PER_CELL;
    let plot_pixels_h  = plot_cells_h * SUB_Y_PER_CELL;

    let plot_box = PlotBox { pixel_width: plot_pixels_w, pixel_height: plot_pixels_h };

    Ok(StackedAreaLayout {
        plot_box, series, x_values, x_count, totals_at_x,
        canvas_cells_w, canvas_cells_h,
        left_margin, bottom_reserve,
        x_min, x_max, y_max,
    })
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::stacked_area
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add stacked area layout with per-series totals and per-x cumulative"
```

---

## Task 3: Stacked area rasterizer

**Files:**
- Create: `crates/tplot-core/src/rasterize/stacked_area.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

The rasterizer walks each pixel-column `px` in the plot buffer. For each `px`:
1. Convert `px` back to a data-x value: `x = x_min + (px / (plot_pixels_w - 1)) * (x_max - x_min)`.
2. Find the surrounding data x-indices (`i_lo`, `i_hi`) and linear interpolation factor `t`.
3. For each series in stack order, compute the interpolated cumulative top `cum_top` (in data-y space) by summing across all series ≤ this one.
4. Convert `cum_top` and `cum_bottom` to pixel-y values (inverted).
5. Fill from `cum_bottom_py` UP to `cum_top_py` at this x with the series's color.

Walking once per pixel column gives a smooth-looking fill without explicit polygon math.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/stacked_area.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, StackedAreaLayout, StackedAreaSeries};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const RED:  RgbColor = RgbColor { r: 0xff, g: 0x00, b: 0x00 };
    const BLUE: RgbColor = RgbColor { r: 0x00, g: 0x00, b: 0xff };

    fn fake_layout() -> StackedAreaLayout {
        StackedAreaLayout {
            plot_box: PlotBox { pixel_width: 10, pixel_height: 10 },
            series: vec![
                StackedAreaSeries { key: "A".into(), y_values: vec![10.0, 10.0, 10.0], total: 30.0 },
                StackedAreaSeries { key: "B".into(), y_values: vec![5.0,  5.0,  5.0],  total: 15.0 },
            ],
            x_values: vec![1.0, 2.0, 3.0],
            x_count: 3,
            totals_at_x: vec![15.0, 15.0, 15.0],
            canvas_cells_w: 30, canvas_cells_h: 8,
            left_margin: 6, bottom_reserve: 3,
            x_min: 1.0, x_max: 3.0, y_max: 15.0,
        }
    }

    #[test]
    fn paints_two_layers_in_their_colors() {
        let mut buf = crate::PixelBuffer::new(10, 10);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), RED);
        palette.insert("B".into(), BLUE);
        rasterize_stacked_area(&fake_layout(), &palette, &mut buf);

        // With totals all = 15, A occupies 10/15 = 2/3 of the height (top portion of the buffer).
        // B occupies the remaining 1/3 (bottom portion).
        // Pixel (5, 0) = top of buffer = top of stack. With y inverted, that's series B (cum_top=15).
        // Pixel (5, 9) = bottom of buffer = baseline (cum=0). That's INSIDE series A.
        assert_eq!(buf.get(5, 9), Some(RED), "bottom should be A");
        assert_eq!(buf.get(5, 0), Some(BLUE), "top should be B");
    }

    #[test]
    fn empty_buffer_when_no_series() {
        let layout = StackedAreaLayout {
            series: vec![],
            ..fake_layout()
        };
        let mut buf = crate::PixelBuffer::new(10, 10);
        let palette: HashMap<String, RgbColor> = HashMap::new();
        rasterize_stacked_area(&layout, &palette, &mut buf);
        assert_eq!(buf.get(0, 0), None);
        assert_eq!(buf.get(5, 5), None);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod stacked_area;
pub use stacked_area::rasterize_stacked_area;
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/stacked_area.rs (above tests)
use crate::layout::StackedAreaLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_stacked_area(
    layout:  &StackedAreaLayout,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    let pw = layout.plot_box.pixel_width;
    let ph = layout.plot_box.pixel_height;
    if layout.series.is_empty() || layout.x_count == 0 { return; }
    let n = layout.x_count;
    let x_span = (layout.x_max - layout.x_min).max(1e-9);
    let y_max = layout.y_max.max(1e-9);

    for px in 0..pw {
        // Convert pixel-x to data-x via linear interpolation.
        let frac_x = if pw <= 1 { 0.0 } else { px as f64 / (pw - 1) as f64 };
        let data_x = layout.x_min + frac_x * x_span;

        // Find surrounding data-x indices.
        let (i_lo, i_hi, t) = bracket(&layout.x_values, data_x);

        // Compute interpolated cumulative tops (data space) for each series.
        let mut cum_data: f64 = 0.0;
        for s in &layout.series {
            let v_lo = s.y_values[i_lo];
            let v_hi = s.y_values[i_hi];
            let v    = v_lo + (v_hi - v_lo) * t;
            let cum_top_data    = cum_data + v;
            let cum_bottom_data = cum_data;

            // Map data-y to pixel-y (inverted: high data → low pixel y).
            let py_top    = ((y_max - cum_top_data)    / y_max * (ph - 1) as f64).round() as usize;
            let py_bottom = ((y_max - cum_bottom_data) / y_max * (ph - 1) as f64).round() as usize;

            let lo = py_top.min(py_bottom);
            let hi = py_top.max(py_bottom).min(ph - 1);

            let color = palette.get(&s.key).copied()
                .unwrap_or(RgbColor { r: 0x76, g: 0x76, b: 0x76 });

            for py in lo..=hi {
                buf.set(px, py, color);
            }

            cum_data = cum_top_data;
        }
    }
}

/// Linear-interpolation bracket: given a sorted `xs` and a target `x`, return
/// (lo_idx, hi_idx, t) such that `x ≈ xs[lo] + (xs[hi] - xs[lo]) * t`.
fn bracket(xs: &[f64], x: f64) -> (usize, usize, f64) {
    let n = xs.len();
    if n == 1 { return (0, 0, 0.0); }
    if x <= xs[0]      { return (0, 0, 0.0); }
    if x >= xs[n - 1]  { return (n - 1, n - 1, 0.0); }
    for i in 0..n - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let span = (xs[i + 1] - xs[i]).max(1e-9);
            return (i, i + 1, (x - xs[i]) / span);
        }
    }
    (n - 1, n - 1, 0.0)
}
```

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add stacked area rasterizer with per-pixel-column interpolation"
```

---

## Task 4: Story-pass — focal-by-total

**Files:**
- Modify: `crates/tplot-story/src/focal.rs`
- Modify: `crates/tplot-story/src/takeaway.rs`
- Modify: `crates/tplot-story/src/lib.rs`

For stacked area, the "interesting" series is typically the one contributing the most area — i.e., the largest cumulative total across the time range.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-story/src/focal.rs (within existing tests module)
#[test]
fn picks_largest_total_series() {
    let totals = vec![
        SeriesTotal { key: "NA".into(),    total: 70.0 },
        SeriesTotal { key: "EMEA".into(),  total: 16.0 },
        SeriesTotal { key: "LATAM".into(), total: 8.0 },
    ];
    let r = pick_focal_by_total(&totals);
    assert_eq!(r.choice, FocalChoice::Series("NA".into()));
}

#[test]
fn admits_no_focal_when_totals_uniform() {
    let totals = vec![
        SeriesTotal { key: "a".into(), total: 50.0 },
        SeriesTotal { key: "b".into(), total: 51.0 },
        SeriesTotal { key: "c".into(), total: 49.0 },
        SeriesTotal { key: "d".into(), total: 50.5 },
    ];
    let r = pick_focal_by_total(&totals);
    assert_eq!(r.choice, FocalChoice::None);
}
```

```rust
// crates/tplot-story/src/takeaway.rs (within existing tests module)
#[test]
fn stacked_area_takeaway_quantifies_share() {
    let t = stacked_area_takeaway(Some("NA"), 70.0, 94.0);
    assert!(t.contains("NA"));
    assert!(t.contains("74%") || t.contains("75%") || t.contains("70"));
}

#[test]
fn stacked_area_takeaway_neutral_for_no_focal() {
    let t = stacked_area_takeaway(None, 0.0, 0.0);
    assert!(t.to_lowercase().contains("similar") || t.to_lowercase().contains("no series"));
}
```

- [ ] **Step 2: Implement focal-by-total**

```rust
// crates/tplot-story/src/focal.rs (append)
#[derive(Debug, Clone)]
pub struct SeriesTotal {
    pub key:   String,
    pub total: f64,
}

pub fn pick_focal_by_total(totals: &[SeriesTotal]) -> FocalResult {
    if totals.is_empty() {
        return FocalResult { choice: FocalChoice::None, trust_score: 0.0, reason: "empty" };
    }
    let mut sorted: Vec<f64> = totals.iter().map(|t| t.total).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    let max_idx = totals.iter().enumerate()
        .max_by(|a, b| a.1.total.partial_cmp(&b.1.total).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();
    let trust = if median.abs() < 1e-9 {
        if totals[max_idx].total > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        totals[max_idx].total / median
    };
    if trust >= 1.5 {
        FocalResult {
            choice: FocalChoice::Series(totals[max_idx].key.clone()),
            trust_score: trust,
            reason: "total",
        }
    } else {
        FocalResult { choice: FocalChoice::None, trust_score: trust, reason: "uniform-totals" }
    }
}
```

- [ ] **Step 3: Implement takeaway**

```rust
// crates/tplot-story/src/takeaway.rs (append)
pub fn stacked_area_takeaway(focal: Option<&str>, focal_total: f64, grand_total: f64) -> String {
    match focal {
        None => "Series contributed similar amounts — no clear leader.".to_string(),
        Some(name) if grand_total <= 0.0 => format!("{name} carried the bulk of the total."),
        Some(name) => {
            let pct = (focal_total / grand_total * 100.0).round() as i64;
            format!("{name} contributed the most — {pct}% of the cumulative total ({focal_total:.0} of {grand_total:.0}).")
        }
    }
}
```

- [ ] **Step 4: Add story-pass entry point**

```rust
// crates/tplot-story/src/lib.rs (append)
use crate::focal::{pick_focal_by_total, SeriesTotal};

pub fn run_stacked_area_story_pass(
    totals:  &[SeriesTotal],
    config:  &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = totals.iter()
            .map(|t| (t.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None, palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }
    let focal_choice = match &config.focus {
        FocusMode::Auto         => pick_focal_by_total(totals),
        FocusMode::Series(name) => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None         => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };
    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None      => None,
    };
    let keys: Vec<&str> = totals.iter().map(|t| t.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);
    StoryAnnotated { focal: focal_name.map(String::from), palette_map, takeaway: None }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-story --lib
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add stacked area story-pass with focal-by-total detection"
```

---

## Task 5: CLI subcommand

**Files:**
- Modify: `crates/tplot/src/cli.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_area_subcommand() {
    let args = Cli::parse_from([
        "tplot", "area", "metrics.csv",
        "-x", "month", "-y", "revenue", "--group", "region",
    ]);
    match args.command {
        Command::Area(a) => {
            assert_eq!(a.input, "metrics.csv");
            assert_eq!(a.x, "month");
            assert_eq!(a.y, "revenue");
            assert_eq!(a.group, "region");
        }
        _ => panic!("expected Area"),
    }
}
```

- [ ] **Step 2: Add the variant**

```rust
// crates/tplot/src/cli.rs — modify Command enum, add AreaArgs

#[derive(Subcommand, Debug)]
pub enum Command {
    Bar(BarArgs),
    Hist(HistArgs),
    Line(LineArgs),
    Scatter(ScatterArgs),
    Spark(SparkArgs),
    Heatmap(HeatmapArgs),
    Box(BoxArgs),
    /// Render a stacked-area chart. Each group fills from the cumulative
    /// baseline up to its cumulative top, in first-seen order.
    Area(AreaArgs),
    Json,
}

#[derive(Args, Debug)]
pub struct AreaArgs {
    pub input: String,
    /// Numeric x-axis column (typically time).
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric y-axis column (the values to stack).
    #[arg(short = 'y')]
    pub y: String,
    /// Required: column whose unique values form the stacked series.
    #[arg(long)]
    pub group: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}
```

- [ ] **Step 3: Run (expected pass)**

```bash
cargo test -p tplot --lib cli
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Add area subcommand to CLI argument parser"
```

---

## Task 6: Stacked area pipeline

**Files:**
- Create: `crates/tplot/src/commands/stacked_area.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/stacked_area.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_stacked_area_with_focal_largest_total() {
        let csv = "month,rev,g\n\
            1,10,NA\n1,5,EMEA\n\
            2,20,NA\n2,5,EMEA\n\
            3,40,NA\n3,6,EMEA\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = AreaOptions {
            x: "month".into(), y: "rev".into(), group: "g".into(),
            focus: None, annotate: None,
            neutral: false, no_takeaway: false,
            width: Some(80), height: 16,
            palette_name: "signature".into(),
        };
        let out = render_stacked_area(&df, &opts).unwrap();
        // NA total = 70, EMEA total = 16 → NA focal in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        assert!(out.contains("NA"));
        // Takeaway phrasing.
        assert!(out.to_lowercase().contains("contributed") || out.to_lowercase().contains("most"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod stacked_area;
pub use stacked_area::{render_stacked_area, AreaOptions};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/stacked_area.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_stacked_area;
use tplot_core::rasterize::rasterize_stacked_area;
use tplot_core::PixelBuffer;
use tplot_render::render_halfblocks;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_story::{run_stacked_area_story_pass, focal::SeriesTotal, stacked_area_takeaway};

#[derive(Debug, Clone)]
pub struct AreaOptions {
    pub x:            String,
    pub y:            String,
    pub group:        String,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub width:        Option<usize>,
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_stacked_area(df: &DataFrame, opts: &AreaOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;

    let layout = layout_stacked_area(df, &opts.x, &opts.y, &opts.group, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass over per-series totals.
    let totals: Vec<SeriesTotal> = layout.series.iter()
        .map(|s| SeriesTotal { key: s.key.clone(), total: s.total })
        .collect();
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let story = run_stacked_area_story_pass(&totals, &story_cfg, palette);

    // Rasterize.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_stacked_area(&layout, &story.palette_map, &mut buf);

    // Render.
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // Compose: y-axis labels, plot, x-axis labels, takeaway.
    let mut out = String::new();
    let n_rows = body_lines.len();
    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", layout.y_max)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", layout.y_max / 2.0)
        } else if i + 1 == n_rows {
            format!("{:>5}", "0")
        } else {
            " ".repeat(5)
        };
        out.push_str(&y_label);
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }

    // X-axis labels: just min and max under the plot.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width;
    let pad = plot_cells.saturating_sub(label_l.len() + label_r.len());
    x_axis.push_str(&label_l);
    x_axis.push_str(&" ".repeat(pad));
    x_axis.push_str(&label_r);
    out.push_str(&x_axis);
    out.push('\n');

    // Legend: list series in stack order with their focal/context colors.
    let legend: String = layout.series.iter().map(|s| {
        let focal = story.focal.as_deref() == Some(&s.key);
        let marker = if focal { "■" } else { "·" };
        format!(" {marker} {}", s.key)
    }).collect::<Vec<_>>().join("  ");
    out.push_str(&" ".repeat(layout.left_margin));
    out.push_str(&legend);
    out.push('\n');

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let s = layout.series.iter().find(|s| s.key == *name);
            let grand: f64 = layout.series.iter().map(|s| s.total).sum();
            let focal_total = s.map(|s| s.total).unwrap_or(0.0);
            stacked_area_takeaway(Some(name), focal_total, grand)
        } else {
            stacked_area_takeaway(None, 0.0, 0.0)
        };
        out.push_str(&takeaway);
        out.push('\n');
    }

    Ok(out)
}
```

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Add stacked area pipeline with legend and focal-by-total takeaway"
```

---

## Task 7: Wire main.rs and JSON dispatch

**Files:**
- Modify: `crates/tplot/src/main.rs`
- Modify: `crates/tplot/src/commands/json.rs`

- [ ] **Step 1: Wire main.rs**

```rust
// crates/tplot/src/main.rs — add Area arm
Command::Area(a) => {
    let df = pipeline::read_dataframe(&a.input)?;
    let (_, height) = pipeline::detected_terminal_size(a.common.width);
    let out = commands::render_stacked_area(&df, &commands::AreaOptions {
        x: a.x, y: a.y, group: a.group,
        focus: a.common.focus, annotate: a.common.annotate,
        neutral: a.common.neutral, no_takeaway: a.common.no_takeaway,
        width: a.common.width, height,
        palette_name: a.common.palette,
    })?;
    print!("{out}");
    Ok(())
}
```

- [ ] **Step 2: Replace placeholder in json.rs**

```rust
// crates/tplot/src/commands/json.rs — replace the StackedArea placeholder arm
ChartKind::StackedArea => {
    let x = match spec.x {
        Axis::Column(c) => c,
        _ => return Err(anyhow!("inline x axis not supported in v1")),
    };
    let y = match spec.y {
        Axis::Column(c) => c,
        _ => return Err(anyhow!("inline y axis not supported in v1")),
    };
    let group = spec.group.ok_or_else(|| anyhow!(
        "stacked area requires a `group` column to define series"
    ))?;
    let opts = AreaOptions {
        x, y, group,
        focus: match spec.story.focus {
            tplot_protocol::FocusMode::Series(s) => Some(s),
            _ => None,
        },
        annotate:     spec.story.annotation,
        neutral:      !spec.story.enabled,
        no_takeaway:  !spec.story.takeaway,
        width:        Some(canvas_w),
        height:       canvas_h,
        palette_name: "signature".into(),
    };
    crate::commands::render_stacked_area(&parsed.dataframe, &opts)
}
```

Add: `use crate::commands::AreaOptions;`

- [ ] **Step 3: Run + smoke test**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Smoke test
echo "month,rev,region
1,10,NA
1,5,EMEA
1,3,APAC
2,15,NA
2,7,EMEA
2,5,APAC
3,30,NA
3,9,EMEA
3,6,APAC
4,40,NA
4,12,EMEA
4,8,APAC
5,55,NA
5,15,EMEA
5,10,APAC
6,70,NA
6,18,EMEA
6,11,APAC" | cargo run -p tplot -- area - -x month -y rev --group region
```

Expected: a smoothly-rising stacked area with NA highlighted in burnt orange (focal — largest total ~220), EMEA and APAC in gray. Legend at the bottom and takeaway naming NA's contribution percentage.

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Wire stacked area into main and JSON dispatch"
```

---

## Task 8: End-to-end snapshot tests + README + lint pass

**Files:**
- Create: `crates/tplot/tests/e2e_stacked_area.rs`
- Modify: `README.md`

- [ ] **Step 1: Write the snapshot tests**

```rust
// crates/tplot/tests/e2e_stacked_area.rs
use std::io::Write;
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(2).unwrap().to_path_buf()
}

fn run_with_stdin(args: &[&str], input: &str) -> String {
    let mut child = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("tplot binary failed to launch");
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
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn stacked_area_three_series_snapshot() {
    let csv = "month,rev,region\n\
        1,10,NA\n1,5,EMEA\n1,3,APAC\n\
        2,15,NA\n2,7,EMEA\n2,5,APAC\n\
        3,30,NA\n3,9,EMEA\n3,6,APAC\n\
        4,40,NA\n4,12,EMEA\n4,8,APAC\n\
        5,55,NA\n5,15,EMEA\n5,10,APAC\n\
        6,70,NA\n6,18,EMEA\n6,11,APAC\n";
    let out = run_with_stdin(&[
        "area", "-", "-x", "month", "-y", "rev", "--group", "region",
        "--width", "80",
    ], csv);
    insta::assert_snapshot!("stacked_area_three_series", strip_ansi(&out));
}

#[test]
fn stacked_area_neutral_snapshot() {
    let csv = "month,rev,region\n\
        1,10,NA\n1,5,EMEA\n\
        2,20,NA\n2,5,EMEA\n\
        3,40,NA\n3,6,EMEA\n";
    let out = run_with_stdin(&[
        "area", "-", "-x", "month", "-y", "rev", "--group", "region",
        "--neutral", "--width", "80",
    ], csv);
    insta::assert_snapshot!("stacked_area_neutral", strip_ansi(&out));
}
```

- [ ] **Step 2: Run, inspect, accept**

```bash
cargo test -p tplot --test e2e_stacked_area
cargo insta accept
cargo test -p tplot --test e2e_stacked_area
```

- [ ] **Step 3: Update README** to "Plans 1+2+3+4+4.5+5+5.5". Add quickstart:

````markdown
```bash
# Stacked area — cumulative breakdown over time
echo "month,rev,region
1,10,NA
1,5,EMEA
2,20,NA
2,7,EMEA
3,40,NA
3,12,EMEA" | tplot area - -x month -y rev --group region
```
````

- [ ] **Step 4: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add crates/tplot/tests/ README.md
git add -A   # if fmt produced changes
git commit -m "Add e2e snapshot tests for stacked area, update README, lint pass"
```

---

## Done — Plan 5.5 deliverable

```bash
echo "month,rev,region
1,10,NA
1,5,EMEA
1,3,APAC
2,20,NA
2,7,EMEA
2,5,APAC
3,40,NA
3,9,EMEA
3,6,APAC" | tplot area - -x month -y rev --group region
```

That's the v1 MVP chart set complete. **9 chart types** working: horizontal bar, vertical bar, histogram, line, scatter, sparkline, heatmap, box plot, stacked area.

**Plans 1–5.5 deliverables:**
- Three renderers: half-blocks, vertical-block elements, Braille
- Three palettes (signature/editorial/colorblind-safe) and three heat ramps (inferno/viridis/coolwarm)
- Story-pass with chart-specific focal heuristics: max-value, modal-bin, delta-trend, point-count, hottest-cell, widest-IQR, **largest-total**

**Still missing for v1**:
- `--graphics` image-protocol output
- OSC capability probing
- Distribution / Python wrapper

## Self-review notes

- ChartKind::StackedArea (Task 1) ✓
- Stacked area layout (Task 2) ✓
- Stacked area rasterizer with per-pixel-column interpolation (Task 3) ✓
- Story-pass focal-by-total (Task 4) ✓
- CLI subcommand (Task 5) ✓
- Pipeline (Task 6) ✓
- Main + JSON wiring (Task 7) ✓
- Snapshot tests + README + lint (Task 8) ✓
