# TerminalPlot — Plan 11.5: Ridgeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add `tplot ridge` to the binary — a "joy plot" style chart where each category's KDE is drawn as a filled curve on its own horizontal row, stacked vertically with slight overlap. Story-pass picks the group with the widest IQR as focal (same as violin / box plot).

**Architecture:** Reuses `tplot-core::kde` (built in Plan 11), `FiveNumberSummary` from `tplot-core::stats`, and the box plot's focal-by-IQR story-pass. New: ridgeline layout (per-row KDE on x-grid, stacked with overlap) and rasterizer (fill area under each curve, upper rows on top).

**Tech Stack:** Same as prior plans. No new dependencies.

**Inherited context (Plans 1–11):**
- 12 chart types working; ~294 tests passing.
- KDE utility lives in `tplot-core::kde`. `kde_evaluate(data, grid)` returns densities at each grid point.
- `run_boxplot_story_pass_with_theme` returns `StoryAnnotated` with `focal: Option<String>` and `palette_map`.

---

## File Structure

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add Ridgeline variant
├── tplot-core/src/
│   ├── layout/
│   │   ├── mod.rs                  EXTEND
│   │   └── ridgeline.rs            ★ NEW
│   └── rasterize/
│       ├── mod.rs                  EXTEND
│       └── ridgeline.rs            ★ NEW: paint stacked filled curves
├── tplot/src/
│   ├── cli.rs                      EXTEND: RidgeArgs (-x value, --group categories)
│   ├── commands/
│   │   ├── mod.rs                  EXTEND
│   │   ├── ridgeline.rs            ★ NEW: pipeline
│   │   └── json.rs                 EXTEND: dispatch
│   └── main.rs                     EXTEND
└── tplot/tests/
    └── e2e_ridgeline.rs            ★ NEW: snapshot tests
```

---

## Task 1: Add Ridgeline to ChartKind

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-protocol/src/chart.rs (within existing tests module)
#[test]
fn ridgeline_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Ridgeline,
        x: Axis::Column("value".into()),
        y: Axis::Column("__count__".into()),  // unused — kept for ChartSpec uniformity
        group: Some("category".into()),
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
    Candlestick { open: String, high: String, low: String, close: String },
    Treemap,
    Violin,
    /// Ridgeline (joy-plot) chart — stacked KDEs per group along a categorical
    /// y-axis, each ridge a filled curve over a shared x-axis.
    Ridgeline,
}
```

Add a placeholder arm in `crates/tplot/src/commands/json.rs` for `ChartKind::Ridgeline` returning `Err(anyhow!("ridgeline JSON dispatch lands in plan 11.5 task 6"))`.

- [ ] **Step 3: Run + verify build**

```bash
cargo test -p tplot-protocol --lib chart
cargo build --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot-protocol/ crates/tplot/src/commands/json.rs
git commit -m "Add Ridgeline variant to ChartKind"
```

---

## Task 2: Ridgeline layout

**Files:**
- Create: `crates/tplot-core/src/layout/ridgeline.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Layout strategy:
- Group by `group_col`, collect `value_col` per group. Preserve first-seen order.
- Determine global x-range (data values), pad ±5%.
- Build an x-grid (along the SHARED x-axis) — one density value per pixel column.
- Per group:
  - Compute KDE on the x-grid → density array.
  - Compute the 5-number summary (for focal-by-IQR + median markers).
- Per-group normalize each KDE: `density / group_max → [0, 1]` so each ridge has the same maximum height.
- Place each ridge at a y-position. Ridges OVERLAP by ~30% (each new ridge starts 70% of the previous ridge's height down from the previous baseline). The classic joy-plot look.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/ridgeline.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn months_df() -> DataFrame {
        // Three months of latencies — different distributions.
        DataFrame::from_columns(vec![
            Column::new("month", Series::Strings(
                vec!["jan","jan","jan","jan",
                     "feb","feb","feb","feb","feb","feb",
                     "mar","mar","mar","mar","mar","mar","mar"]
                .into_iter().map(String::from).collect())),
            Column::new("ms", Series::Numbers(vec![
                40.0, 50.0, 55.0, 60.0,
                30.0, 45.0, 80.0, 100.0, 130.0, 160.0,
                20.0, 25.0, 28.0, 30.0, 32.0, 35.0, 40.0,
            ])),
        ]).unwrap()
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
            assert!(w[0].baseline_y < w[1].baseline_y,
                "expected ascending baseline_y between ridges");
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
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod ridgeline;
pub use ridgeline::{layout_ridgeline, RidgelineLayout, Ridge, RidgelineError};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/ridgeline.rs (above tests)
use crate::dataframe::{DataFrame, Series};
use crate::kde::kde_evaluate;
use crate::stats::{five_number_summary, FiveNumberSummary};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct Ridge {
    pub label:       String,
    pub summary:     FiveNumberSummary,
    /// `heights[px]` = pixel height of the curve at column `px`, normalized to
    /// each ridge's own peak (so every ridge has the same max height in pixels).
    pub heights:     Vec<usize>,
    /// Pixel-y of the ridge's baseline (where heights = 0).
    pub baseline_y:  usize,
    /// Maximum vertical extent of this ridge in pixels (the "peak" allotment).
    pub max_height:  usize,
    /// Sub-pixel x of the median value (for an optional median tick mark).
    pub median_x:    usize,
    pub series_key:  String,
}

#[derive(Debug, Clone)]
pub struct RidgelineLayout {
    pub plot_box:        PlotBox,
    pub ridges:          Vec<Ridge>,
    pub canvas_cells_w:  usize,
    pub canvas_cells_h:  usize,
    pub left_margin:     usize,
    pub bottom_reserve:  usize,
    pub x_min:           f64,
    pub x_max:           f64,
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
    df:             &DataFrame,
    value_col:      &str,
    group_col:      &str,
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
    if values.is_empty() { return Err(RidgelineError::Empty); }

    // Group values by group, preserving first-seen order.
    let mut groups_data: Vec<(String, Vec<f64>)> = Vec::new();
    for (g, v) in groups.iter().zip(values.iter()) {
        if let Some(slot) = groups_data.iter_mut().find(|(name, _)| name == g) {
            slot.1.push(*v);
        } else {
            groups_data.push((g.clone(), vec![*v]));
        }
    }
    let summaries: Vec<(String, FiveNumberSummary, Vec<f64>)> = groups_data.into_iter()
        .filter_map(|(g, vs)| five_number_summary(&vs).map(|s| (g, s, vs)))
        .collect();
    if summaries.is_empty() { return Err(RidgelineError::Empty); }

    // Global x-range (data values), padded.
    let x_min_raw = summaries.iter().map(|(_, s, _)| s.min).fold(f64::INFINITY, f64::min);
    let x_max_raw = summaries.iter().map(|(_, s, _)| s.max).fold(f64::NEG_INFINITY, f64::max);
    let span_raw  = (x_max_raw - x_min_raw).max(1e-9);
    let x_min     = x_min_raw - span_raw * 0.05;
    let x_max     = x_max_raw + span_raw * 0.05;
    let x_span    = (x_max - x_min).max(1e-9);

    let n              = summaries.len();
    let left_margin    = 8;     // group labels on the left
    let bottom_reserve = 3;
    let plot_cells_w   = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h   = canvas_cells_h.saturating_sub(bottom_reserve).max(n.max(2));
    let plot_pixels_w  = plot_cells_w * SUB_X_PER_CELL;
    let plot_pixels_h  = plot_cells_h * SUB_Y_PER_CELL;

    // Each ridge gets a "slot" of pixel rows; slots step by slot_h, but each
    // ridge's peak can extend up to PEAK_OVERLAP × slot_h pixels above its
    // baseline. The last ridge's peak can stick up above the plot top — fine,
    // it gets clamped by the rasterizer.
    let slot_h = (plot_pixels_h / n).max(2);
    let max_peak = ((slot_h as f64) * PEAK_OVERLAP).round() as usize;

    let plot_box = PlotBox { pixel_width: plot_pixels_w, pixel_height: plot_pixels_h };

    // X-grid: one data-x value per pixel column.
    let grid: Vec<f64> = (0..plot_pixels_w).map(|px| {
        let frac = if plot_pixels_w <= 1 { 0.0 } else { px as f64 / (plot_pixels_w - 1) as f64 };
        x_min + frac * x_span
    }).collect();

    let map_x = |v: f64| -> usize {
        let px = ((v - x_min) / x_span * (plot_pixels_w - 1) as f64).round() as usize;
        px.min(plot_pixels_w - 1)
    };

    let ridges: Vec<Ridge> = summaries.into_iter().enumerate().map(|(i, (label, summary, vs))| {
        let densities = kde_evaluate(&vs, &grid);
        let local_max = densities.iter().copied().fold(0.0_f64, f64::max).max(1e-12);

        // Heights (pixels) per column, normalized to this ridge's own peak.
        let heights: Vec<usize> = densities.iter().map(|&d| {
            let raw = (d / local_max) * max_peak as f64;
            if raw > 0.05 { raw.round().max(1.0) as usize } else { 0 }
        }).collect();

        // Baseline y for this ridge: stack from the bottom up.
        // Earlier ridges sit higher (smaller baseline_y).
        let baseline_y = (i + 1) * slot_h;
        let baseline_y = baseline_y.min(plot_pixels_h - 1);

        Ridge {
            label: label.clone(),
            summary,
            heights,
            baseline_y,
            max_height: max_peak,
            median_x: map_x(summary.median),
            series_key: label,
        }
    }).collect();

    Ok(RidgelineLayout {
        plot_box, ridges,
        canvas_cells_w, canvas_cells_h,
        left_margin, bottom_reserve,
        x_min, x_max,
    })
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot-core --lib layout::ridgeline
git add crates/tplot-core/
git commit -m "Add ridgeline layout with stacked KDEs and per-group normalization"
```

---

## Task 3: Ridgeline rasterizer

**Files:**
- Create: `crates/tplot-core/src/rasterize/ridgeline.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

For each ridge, walk its `heights` array. At column `px`, fill from `baseline_y - heights[px]` up to `baseline_y` with the ridge's color. Optional: a small tick mark at `median_x` along the baseline.

Painting order matters because ridges overlap: paint TOP TO BOTTOM (last in vec ↦ painted first). That way, later ridges (which sit lower) overwrite the bottoms of earlier ridges where they overlap, producing the classic joy-plot stacking.

Wait — actually it's the opposite. Later ridges have LARGER baseline_y (lower in the buffer). Earlier ridges' peaks may extend above their slot but their baselines are higher up. The newer (lower) ridges should APPEAR on top, hiding the bottoms of older (upper) ridges where they overlap.

So paint OLDEST FIRST (smallest baseline_y first), so later ridges overwrite. That's the natural order in the vec.

Plus: focal-on-top — focal ridge painted LAST.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/ridgeline.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Ridge, RidgelineLayout, PlotBox};
    use crate::stats::FiveNumberSummary;
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };
    const GRAY:   RgbColor = RgbColor { r: 0x76, g: 0x76, b: 0x76 };

    fn fake_layout() -> RidgelineLayout {
        // Two ridges in a 20×20 buffer. First (jan) baseline at y=10, second
        // (feb) at y=18. Both have a triangular peak shape.
        let triangle = |center: usize, n: usize, peak: usize| -> Vec<usize> {
            (0..n).map(|i| {
                let dist = if i > center { i - center } else { center - i };
                if dist >= peak { 0 } else { peak - dist }
            }).collect()
        };
        RidgelineLayout {
            plot_box: PlotBox { pixel_width: 20, pixel_height: 20 },
            ridges: vec![
                Ridge {
                    label: "jan".into(),
                    summary: FiveNumberSummary { min: 0.0, q1: 25.0, median: 50.0, q3: 75.0, max: 100.0 },
                    heights: triangle(5, 20, 6),
                    baseline_y: 10,
                    max_height: 6,
                    median_x: 5,
                    series_key: "jan".into(),
                },
                Ridge {
                    label: "feb".into(),
                    summary: FiveNumberSummary { min: 0.0, q1: 25.0, median: 50.0, q3: 75.0, max: 100.0 },
                    heights: triangle(15, 20, 6),
                    baseline_y: 18,
                    max_height: 6,
                    median_x: 15,
                    series_key: "feb".into(),
                },
            ],
            canvas_cells_w: 30, canvas_cells_h: 12,
            left_margin: 8, bottom_reserve: 3,
            x_min: 0.0, x_max: 100.0,
        }
    }

    #[test]
    fn paints_ridge_under_curve_in_color() {
        let mut buf = crate::PixelBuffer::new(20, 20);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("jan".into(), ORANGE);
        palette.insert("feb".into(), GRAY);
        rasterize_ridgeline(&fake_layout(), Some("jan"), &palette, &mut buf);

        // Jan's peak at column 5 has height 6 — fill from y=4 to y=10 should be ORANGE.
        assert_eq!(buf.get(5, 5),  Some(ORANGE));
        assert_eq!(buf.get(5, 10), Some(ORANGE));
        // Above the peak (y=3) at column 5 should be empty.
        assert_eq!(buf.get(5, 3),  None);
        // Feb's peak at column 15 has height 6 — fill from y=12 to y=18 should be GRAY.
        assert_eq!(buf.get(15, 14), Some(GRAY));
        assert_eq!(buf.get(15, 18), Some(GRAY));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod ridgeline;
pub use ridgeline::rasterize_ridgeline;
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/ridgeline.rs (above tests)
use crate::layout::RidgelineLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_ridgeline(
    layout:  &RidgelineLayout,
    focal:   Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    // Paint non-focal ridges in order (top to bottom in the buffer = first to
    // last in the vec). Later ridges overwrite earlier ones in the overlap
    // zone, which produces the joy-plot stacking effect.
    for r in &layout.ridges {
        if Some(r.series_key.as_str()) == focal { continue; }
        paint_ridge(r, palette, buf);
    }
    // Focal ridge painted last → on top.
    if let Some(name) = focal {
        if let Some(r) = layout.ridges.iter().find(|r| r.series_key == name) {
            paint_ridge(r, palette, buf);
        }
    }
}

fn paint_ridge(
    r:       &crate::layout::Ridge,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    let color = palette.get(&r.series_key).copied()
        .unwrap_or(RgbColor { r: 0x76, g: 0x76, b: 0x76 });

    for (px, &h) in r.heights.iter().enumerate() {
        if h == 0 { continue; }
        let top = r.baseline_y.saturating_sub(h);
        let bot = r.baseline_y;
        for py in top..=bot {
            buf.set(px, py, color);
        }
    }
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot-core --lib rasterize::ridgeline
git add crates/tplot-core/
git commit -m "Add ridgeline rasterizer painting stacked filled curves"
```

---

## Task 4: CLI subcommand

**Files:**
- Modify: `crates/tplot/src/cli.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_ridge_subcommand() {
    let args = Cli::parse_from([
        "tplot", "ridge", "data.csv",
        "-x", "ms",
        "--group", "month",
    ]);
    match args.command {
        Command::Ridge(r) => {
            assert_eq!(r.input, "data.csv");
            assert_eq!(r.x, "ms");
            assert_eq!(r.group, "month");
        }
        _ => panic!("expected Ridge"),
    }
}
```

- [ ] **Step 2: Add the variant**

```rust
// crates/tplot/src/cli.rs — modify Command enum, add RidgeArgs
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
    Candle(CandleArgs),
    Tree(TreeArgs),
    Violin(ViolinArgs),
    /// Ridgeline (joy-plot) chart — stacked KDEs per group along a categorical axis.
    Ridge(RidgeArgs),
    Doctor,
    Json,
}

#[derive(Args, Debug)]
pub struct RidgeArgs {
    pub input: String,
    /// Numeric value column (the x-axis of each ridge).
    #[arg(short = 'x')]
    pub x: String,
    /// Categorical column whose unique values become the stacked rows.
    #[arg(long)]
    pub group: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test -p tplot --lib cli
git add crates/tplot/
git commit -m "Add ridge subcommand to CLI argument parser"
```

---

## Task 5: Ridgeline pipeline

**Files:**
- Create: `crates/tplot/src/commands/ridgeline.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

Mirrors violin's pipeline. Reuses `run_boxplot_story_pass_with_theme` for focal-by-IQR.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/ridgeline.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_ridgeline_with_widest_iqr_focal() {
        // jan: tight cluster ~50, feb: wider spread, mar: very wide.
        let csv = "month,ms\n\
            jan,40\njan,50\njan,55\njan,60\n\
            feb,30\nfeb,45\nfeb,80\nfeb,100\nfeb,130\nfeb,160\n\
            mar,5\nmar,40\nmar,80\nmar,150\nmar,250\nmar,300\nmar,350\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = RidgeOptions {
            x: "ms".into(), group: "month".into(),
            focus: None, annotate: None,
            neutral: false, no_takeaway: false,
            graphics: "none".into(),
            width: Some(80), height: 16,
            palette_name: "signature".into(),
        };
        let out = render_ridgeline(&df, &opts).unwrap();
        // mar has the widest IQR → focal in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // Group labels should appear on the left.
        assert!(out.contains("jan"));
        assert!(out.contains("feb"));
        assert!(out.contains("mar"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod ridgeline;
pub use ridgeline::{render_ridgeline, RidgeOptions};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/ridgeline.rs (above tests)
use crate::pipeline::{require_minimum_width, resolve_graphics};
use anyhow::{anyhow, Result};
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_ridgeline;
use tplot_core::rasterize::rasterize_ridgeline;
use tplot_core::PixelBuffer;
use tplot_render::render_halfblocks;
use tplot_protocol::{Capabilities, FocusMode, GraphicsProtocol, Palette, StoryConfig};
use tplot_story::{run_boxplot_story_pass_with_theme, focal::SeriesSpread, boxplot_takeaway};

#[derive(Debug, Clone)]
pub struct RidgeOptions {
    pub x:            String,
    pub group:        String,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub graphics:     String,
    pub width:        Option<usize>,
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_ridgeline(df: &DataFrame, opts: &RidgeOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = require_minimum_width(opts.width)?;
    let canvas_h = opts.height;

    let layout = layout_ridgeline(df, &opts.x, &opts.group, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Focal-by-IQR (same heuristic as box plot / violin).
    let spreads: Vec<SeriesSpread> = layout.ridges.iter()
        .map(|r| SeriesSpread { key: r.label.clone(), iqr: r.summary.iqr() })
        .collect();
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let story = run_boxplot_story_pass_with_theme(&spreads, &story_cfg, palette, caps.theme);

    // Rasterize.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_ridgeline(&layout, story.focal.as_deref(), &story.palette_map, &mut buf);

    // Graphics path.
    let protocol = resolve_graphics(&opts.graphics, caps);
    if protocol != GraphicsProtocol::None {
        let mut out = Vec::new();
        out.extend(tplot_render::graphics::render_graphics(&buf, protocol, 6));
        out.push(b'\n');
        return Ok(String::from_utf8_lossy(&out).to_string());
    }

    // Text path: render to half-blocks, prepend group labels per row.
    let body = render_halfblocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // Determine which CELL row each ridge's baseline lives on.
    let baseline_cell_rows: Vec<usize> = layout.ridges.iter()
        .map(|r| r.baseline_y / 2).collect();

    let mut out = String::new();
    let label_w = layout.left_margin.saturating_sub(2);
    for (cy, line) in body_lines.iter().enumerate() {
        // Find a ridge whose baseline is on this cell row.
        let ridge_label = layout.ridges.iter().enumerate()
            .find(|(i, _)| baseline_cell_rows[*i] == cy)
            .map(|(_, r)| r.label.as_str());
        if let Some(label) = ridge_label {
            let trimmed: String = label.chars().take(label_w).collect();
            out.push_str(&format!("{:>w$}  ", trimmed, w = label_w));
        } else {
            out.push_str(&" ".repeat(layout.left_margin));
        }
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

    // Takeaway.
    if !opts.no_takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        let takeaway = if let Some(custom) = &opts.annotate {
            custom.clone()
        } else if let Some(name) = &story.focal {
            let r = layout.ridges.iter().find(|r| r.label == *name);
            if let Some(r) = r {
                boxplot_takeaway(Some(name), r.summary.q1, r.summary.q3, r.summary.min, r.summary.max)
            } else {
                boxplot_takeaway(None, 0.0, 0.0, 0.0, 0.0)
            }
        } else {
            boxplot_takeaway(None, 0.0, 0.0, 0.0, 0.0)
        };
        out.push_str(&takeaway);
        out.push('\n');
    }

    Ok(out)
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot --lib commands::ridgeline
git add crates/tplot/
git commit -m "Add ridgeline pipeline reusing focal-by-IQR story-pass"
```

---

## Task 6: Wire main.rs and JSON dispatch

**Files:**
- Modify: `crates/tplot/src/main.rs`
- Modify: `crates/tplot/src/commands/json.rs`

- [ ] **Step 1: Wire main.rs**

```rust
// crates/tplot/src/main.rs — add Ridge match arm
Command::Ridge(r) => {
    let df = pipeline::read_dataframe(&r.input)?;
    let (_, height) = pipeline::detected_terminal_size(r.common.width);
    let out = commands::render_ridgeline(&df, &commands::RidgeOptions {
        x: r.x, group: r.group,
        focus: r.common.focus, annotate: r.common.annotate,
        neutral: r.common.neutral, no_takeaway: r.common.no_takeaway,
        graphics: r.common.graphics,
        width: r.common.width, height,
        palette_name: r.common.palette,
    })?;
    print!("{out}");
    Ok(())
}
```

- [ ] **Step 2: Replace JSON placeholder**

```rust
// crates/tplot/src/commands/json.rs — replace the Ridgeline placeholder arm
ChartKind::Ridgeline => {
    let x = match spec.x {
        Axis::Column(c) => c,
        _ => return Err(anyhow!("inline x axis not supported in v1")),
    };
    let group = spec.group.ok_or_else(|| anyhow!("ridgeline requires a `group` column"))?;
    let opts = RidgeOptions {
        x, group,
        focus: match spec.story.focus {
            tplot_protocol::FocusMode::Series(s) => Some(s),
            _ => None,
        },
        annotate:     spec.story.annotation,
        neutral:      !spec.story.enabled,
        no_takeaway:  !spec.story.takeaway,
        graphics:     "none".into(),
        width:        Some(canvas_w),
        height:       canvas_h,
        palette_name: "signature".into(),
    };
    crate::commands::render_ridgeline(&parsed.dataframe, &opts)
}
```

Add: `use crate::commands::RidgeOptions;`

- [ ] **Step 3: Run + smoke test**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

echo "month,ms
jan,40
jan,50
jan,55
jan,60
feb,30
feb,45
feb,80
feb,100
feb,130
feb,160
mar,5
mar,40
mar,80
mar,150
mar,250
mar,300
mar,350" | cargo run -p tplot -- ridge - -x ms --group month
```

Expected: 3 ridges stacked vertically (jan top, feb middle, mar bottom), `mar` highlighted in burnt orange (widest IQR), other two in gray. Group labels on the left.

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Wire ridgeline into main and JSON dispatch"
```

---

## Task 7: Snapshot tests + README + lint pass

**Files:**
- Create: `crates/tplot/tests/e2e_ridgeline.rs`
- Modify: `README.md`

- [ ] **Step 1: Snapshot test**

```rust
// crates/tplot/tests/e2e_ridgeline.rs
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
fn ridgeline_three_months_snapshot() {
    let csv = "month,ms\n\
        jan,40\njan,50\njan,55\njan,60\n\
        feb,30\nfeb,45\nfeb,80\nfeb,100\nfeb,130\nfeb,160\n\
        mar,5\nmar,40\nmar,80\nmar,150\nmar,250\nmar,300\nmar,350\n";
    let out = run_with_stdin(&[
        "ridge", "-",
        "-x", "ms",
        "--group", "month",
        "--width", "70",
    ], csv);
    insta::assert_snapshot!("ridgeline_three_months", strip_ansi(&out));
}
```

- [ ] **Step 2: Run, accept**

```bash
cargo test -p tplot --test e2e_ridgeline
cargo insta accept
cargo test -p tplot --test e2e_ridgeline
```

- [ ] **Step 3: README** — add ridge to chart-types list + quickstart:

```markdown
```bash
# Ridgeline (joy-plot) — distribution shapes stacked vertically by category
echo "month,ms
jan,40
jan,55
jan,60
feb,30
feb,80
feb,160
mar,5
mar,80
mar,300" | tplot ridge - -x ms --group month
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
git commit -m "Add e2e snapshot test for ridgeline, update README, lint pass"
```

---

## Done — Plan 11.5 deliverable

```bash
echo "month,ms
jan,40
jan,55
jan,60
feb,30
feb,80
feb,160
mar,5
mar,80
mar,300" | tplot ridge - -x ms --group month
```

**13 chart types now.** The classic joy-plot look — KDE shapes stacked along a categorical axis.

## Self-review notes

- ChartKind::Ridgeline (Task 1) ✓
- Layout (Task 2) ✓
- Rasterizer (Task 3) ✓
- CLI (Task 4) ✓
- Pipeline (Task 5) ✓
- Main + JSON wiring (Task 6) ✓
- Snapshot tests + README + lint (Task 7) ✓
