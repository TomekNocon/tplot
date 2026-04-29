# TerminalPlot — Plan 2: Vertical Bars & Histograms Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tplot bar --vertical` and `tplot hist` to the binary, both rendering with sub-cell vertical resolution via Unicode lower-block elements (`▁▂▃▄▅▆▇█`). Story-pass extended to detect the modal/dominant bin in histograms.

**Architecture:** Reuses the Plan-1 pipeline (input → story → layout → raster → render). Adds: a new `tplot-render::vertical_blocks` renderer, two new `ChartKind` variants (`Bar { orientation: Vertical }` and `Histogram`), two new layout/rasterize pairs in `tplot-core`, a histogram story-pass in `tplot-story`, and two new CLI handlers in `tplot`.

**Tech Stack:** Same as Plan 1 — Rust 2024, `clap`, `serde`, `csv`, `crossterm`, `insta`, `proptest`. No new dependencies.

**Important context from Plan 1:** Several plan-bugs were caught and fixed inline by implementer subagents (Palette gray values, ANSI 256 quantization formula, RgbColor missing Hash derive, takeaway wording, integration-test cwd). Treat that as a working assumption: this plan is also code that *should* compile, but will get the same skeptical-implementer review.

---

## File Structure

New files in **bold**, existing files marked with their Plan-1 location.

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add Histogram variant to ChartKind
├── tplot-render/src/
│   ├── lib.rs                      EXTEND: re-export render_vertical_blocks
│   └── vertical_blocks.rs          ★ NEW: 8-step vertical-block renderer
├── tplot-core/src/
│   ├── layout/
│   │   ├── mod.rs                  EXTEND: re-export new layouts
│   │   ├── vertical_bar.rs         ★ NEW: vertical bar layout
│   │   └── histogram.rs            ★ NEW: histogram (binning + layout)
│   └── rasterize/
│       ├── mod.rs                  EXTEND: re-export new rasterizers
│       └── vertical.rs             ★ NEW: vertical-orientation rasterizer (used for both bar+hist)
├── tplot-story/src/
│   ├── lib.rs                      EXTEND: add run_histogram_story_pass
│   ├── focal.rs                    EXTEND: add pick_modal_bin
│   └── takeaway.rs                 EXTEND: histogram_takeaway template
├── tplot/src/
│   ├── cli.rs                      EXTEND: HistArgs subcommand, --vertical flag already exists
│   └── commands/
│       ├── mod.rs                  EXTEND: re-export new commands
│       ├── bar.rs                  EXTEND: handle vertical orientation
│       └── histogram.rs            ★ NEW: hist subcommand handler
└── tplot/tests/
    └── e2e_vertical_and_hist.rs    ★ NEW: snapshot tests for the new charts
```

---

## Task 1: Vertical-block renderer (the new glyph technique)

**Files:**
- Create: `crates/tplot-render/src/vertical_blocks.rs`
- Modify: `crates/tplot-render/src/lib.rs`

The renderer reads a sub-pixel buffer where each cell covers 1×8 sub-pixels (1 column, 8 rows). For each cell, it counts the number of "filled" sub-pixels from the bottom up and emits the corresponding lower-block glyph (` `, `▁`, `▂`, `▃`, `▄`, `▅`, `▆`, `▇`, `█`) with truecolor foreground. Discontinuous fills within a cell are not supported — the renderer assumes solid columns (which the layout/raster guarantee).

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/vertical_blocks.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::PixelBuffer;
    use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, RgbColor};

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    fn caps() -> Capabilities {
        Capabilities {
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::HalfBlocks,
            graphics_protocol: GraphicsProtocol::None,
        }
    }

    fn paint_column_from_bottom(buf: &mut PixelBuffer, x: usize, fill_height: usize, color: RgbColor) {
        let h = buf.pixel_height();
        for y in (h - fill_height)..h {
            buf.set(x, y, color);
        }
    }

    #[test]
    fn empty_buffer_renders_only_spaces() {
        let buf = PixelBuffer::new(3, 8);
        let s = render_vertical_blocks(&buf, caps());
        assert!(!s.contains('\u{2581}')); // no ▁
        assert!(!s.contains('\u{2588}')); // no █
    }

    #[test]
    fn full_column_renders_full_block() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 8, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2588}'));
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn one_eighth_column_renders_lower_one_eighth() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 1, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2581}'));
    }

    #[test]
    fn five_eighths_column_renders_lower_five_eighths() {
        let mut buf = PixelBuffer::new(1, 8);
        paint_column_from_bottom(&mut buf, 0, 5, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2585}'));
    }

    #[test]
    fn taller_than_one_cell_uses_full_blocks_for_lower_cells() {
        // 16 sub-pixels = 2 cells. Bar fills 12 sub-pixels.
        // → bottom cell (rows 8-15): full block (8 filled)
        // → top cell    (rows 0-7):  4 filled from bottom = ▄ (lower-half block, U+2584)
        let mut buf = PixelBuffer::new(1, 16);
        paint_column_from_bottom(&mut buf, 0, 12, ORANGE);
        let s = render_vertical_blocks(&buf, caps());
        assert!(s.contains('\u{2588}'));        // full block somewhere
        assert!(s.contains('\u{2584}'));        // half block somewhere
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-render/src/lib.rs (append)
pub mod vertical_blocks;
pub use vertical_blocks::render_vertical_blocks;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-render --lib vertical_blocks
```
Expected: FAIL — module/function not found.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-render/src/vertical_blocks.rs (above tests)
use crate::ansi::{fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

/// 8 lower-block glyphs from empty (0/8) to full (8/8) of a cell.
/// Index = number of filled sub-pixel rows in the cell, counted from the bottom.
const GLYPHS: [char; 9] = [
    ' ',         // 0/8
    '\u{2581}',  // ▁ 1/8
    '\u{2582}',  // ▂ 2/8
    '\u{2583}',  // ▃ 3/8
    '\u{2584}',  // ▄ 4/8 (lower half)
    '\u{2585}',  // ▅ 5/8
    '\u{2586}',  // ▆ 6/8
    '\u{2587}',  // ▇ 7/8
    '\u{2588}',  // █ 8/8 (full)
];

const SUB_PIXELS_PER_CELL: usize = 8;

/// Render a buffer expecting 8 vertical sub-pixels per cell. The buffer must
/// hold solid bottom-up columns (anything above the bar is empty); the
/// renderer counts filled rows from the bottom of each cell.
pub fn render_vertical_blocks(buf: &PixelBuffer, caps: Capabilities) -> String {
    let cells_w = buf.pixel_width();
    let pixel_h = buf.pixel_height();
    let cells_h = pixel_h.div_ceil(SUB_PIXELS_PER_CELL);
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        // Cells are walked top-down (cy=0 is the topmost row of the chart).
        let py_top = cy * SUB_PIXELS_PER_CELL;
        for cx in 0..cells_w {
            // Determine the dominant color in this cell (first non-empty pixel
            // wins — for solid bars they're all the same color, anyway).
            let mut color: Option<RgbColor> = None;
            let mut filled = 0usize;
            for row in 0..SUB_PIXELS_PER_CELL {
                let py = py_top + row;
                if py >= pixel_h { break; }
                if let Some(c) = buf.get(cx, py) {
                    color = color.or(Some(c));
                    filled += 1;
                }
            }

            if filled == 0 || color.is_none() {
                out.push(' ');
            } else {
                let glyph = GLYPHS[filled.min(8)];
                let _ = write!(out, "{}{}{}", fg(color.unwrap(), caps.color_depth), glyph, reset());
            }
        }
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn _typecheck(_: RgbColor) {}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-render --lib vertical_blocks
```
Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add vertical-block renderer with 8-step sub-cell quantization"
```

---

## Task 2: Add `Histogram` to ChartKind protocol

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`
- Note: `BarOrientation::Vertical` already exists from Plan 1

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-protocol/src/chart.rs (within the existing tests module)
#[test]
fn histogram_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Histogram { bins: Some(20) },
        x: Axis::Column("latency_ms".into()),
        y: Axis::Column("__count__".into()),
        group: None,
        title: Some("Request latency distribution".into()),
        story: StoryConfig::default(),
    };
    let json = serde_json::to_string(&spec).unwrap();
    let back: ChartSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(back, spec);
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot-protocol --lib chart::tests::histogram_spec_round_trip
```
Expected: FAIL — `ChartKind::Histogram` doesn't exist.

- [ ] **Step 3: Add the variant**

```rust
// crates/tplot-protocol/src/chart.rs
// In the ChartKind enum, add a new variant after Bar:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar { orientation: BarOrientation },
    /// Histogram of a single numeric column. Bin count is auto-computed
    /// when None (Sturges' rule); explicit override via Some(N).
    Histogram {
        #[serde(default)]
        bins: Option<usize>,
    },
}
```

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib chart
```
Expected: all green (existing bar test + new histogram test).

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Histogram variant to ChartKind with optional bin count"
```

---

## Task 3: Vertical bar layout (`tplot-core`)

**Files:**
- Create: `crates/tplot-core/src/layout/vertical_bar.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Layout strategy: each bar gets one *column* (>= 1 cell wide). Bar height in sub-pixels = `(value / max_value) * plot_pixels_h`. The buffer is sub-pixel-vertical (8 per cell), sub-pixel-horizontal = 1 (one sub-pixel per cell, like the half-block renderer).

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/vertical_bar.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn small_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("month", Series::Strings(
                vec!["Jan","Feb","Mar","Apr"].into_iter().map(String::from).collect())),
            Column::new("active", Series::Numbers(vec![12.4, 13.1, 18.2, 21.4])),
        ]).unwrap()
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
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod vertical_bar;
pub use vertical_bar::{layout_vertical_bar, VerticalBarLayout, VerticalBarRect};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::vertical_bar
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/vertical_bar.rs (above tests)
use crate::dataframe::{DataFrame, Series};

pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct VerticalBarRect {
    pub label:        String,
    pub value:        f64,
    pub pixel_x:      usize,
    pub pixel_y:      usize,
    pub pixel_width:  usize,
    pub pixel_height: usize,
    pub series_key:   String,
}

#[derive(Debug, Clone)]
pub struct VerticalBarLayout {
    pub plot_box:        PlotBox,
    pub bars:            Vec<VerticalBarRect>,
    pub canvas_cells_w:  usize,
    pub canvas_cells_h:  usize,
    /// Cell width per bar column in the rendered output (≥ 1).
    pub bar_cell_width:  usize,
    /// Cell width of the gap between bars.
    pub gap_cell_width:  usize,
    /// Cells reserved on the left for the y-axis labels.
    pub left_margin:     usize,
    /// Cells reserved at the bottom for x-axis labels and takeaway.
    pub bottom_reserve:  usize,
}

#[derive(Debug, thiserror::Error)]
pub enum VerticalBarLayoutError {
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
    #[error("canvas too narrow for {n} bars (need at least {needed} cells, have {have})")]
    TooNarrow { n: usize, needed: usize, have: usize },
}

const SUB_PIXELS_PER_CELL_Y: usize = 8;
const Y_AXIS_LABEL_WIDTH: usize    = 6;  // " 21.4 " etc.

pub fn layout_vertical_bar(
    df:             &DataFrame,
    x_col:          &str,
    y_col:          &str,
    _group_col:     Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<VerticalBarLayout, VerticalBarLayoutError> {
    let labels: Vec<String> = match df.column(x_col).map_err(|_| VerticalBarLayoutError::Empty)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col).map_err(|_| VerticalBarLayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(VerticalBarLayoutError::NonNumericY(y_col.to_string())),
    };
    if labels.is_empty() { return Err(VerticalBarLayoutError::Empty); }

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
    let left_margin    = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3; // 1 row x-axis labels + 1 blank + 1 takeaway

    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_h = plot_cells_h * SUB_PIXELS_PER_CELL_Y;

    // Pick bar_w + gap_w such that n*(bar_w+gap_w) ≤ plot_cells_w.
    // Default: gap_w = max(1, bar_w / 3).
    let max_bar_w = plot_cells_w / n;
    if max_bar_w == 0 {
        return Err(VerticalBarLayoutError::TooNarrow {
            n, needed: n, have: plot_cells_w
        });
    }
    let bar_cell_width = max_bar_w.min(8).max(1);
    let gap_cell_width = (bar_cell_width / 3).max(1).min(max_bar_w.saturating_sub(bar_cell_width));
    let group_w        = bar_cell_width + gap_cell_width;
    let total_w        = group_w * n;
    let leading        = (plot_cells_w.saturating_sub(total_w)) / 2;

    let plot_box = PlotBox {
        pixel_width:  plot_cells_w,
        pixel_height: plot_pixels_h,
    };

    let max_value = agg.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max).max(1e-9);

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
            pixel_width:  bar_cell_width,
            pixel_height,
            series_key:   label,
        });
    }

    Ok(VerticalBarLayout {
        plot_box, bars,
        canvas_cells_w, canvas_cells_h,
        bar_cell_width, gap_cell_width,
        left_margin, bottom_reserve,
    })
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::vertical_bar
```
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add vertical bar layout with adaptive bar/gap widths"
```

---

## Task 4: Vertical-orientation rasterizer (`tplot-core`)

**Files:**
- Create: `crates/tplot-core/src/rasterize/vertical.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

Generic enough to paint both vertical bars and histogram bins (same shape — column-from-the-bottom).

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/vertical.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, VerticalBarLayout, VerticalBarRect};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    fn fake_layout() -> VerticalBarLayout {
        VerticalBarLayout {
            plot_box: PlotBox { pixel_width: 16, pixel_height: 16 },
            bars: vec![
                VerticalBarRect {
                    label: "Apr".into(), value: 21.4,
                    pixel_x: 4, pixel_y: 0,
                    pixel_width: 2, pixel_height: 16,
                    series_key: "Apr".into(),
                },
                VerticalBarRect {
                    label: "Jan".into(), value: 12.4,
                    pixel_x: 0, pixel_y: 7,
                    pixel_width: 2, pixel_height: 9,
                    series_key: "Jan".into(),
                },
            ],
            canvas_cells_w: 30, canvas_cells_h: 16,
            bar_cell_width: 2, gap_cell_width: 2,
            left_margin: 6, bottom_reserve: 3,
        }
    }

    #[test]
    fn paints_vertical_columns_of_correct_color() {
        let mut buf = crate::PixelBuffer::new(16, 16);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("Apr".into(), ORANGE);
        rasterize_vertical(&fake_layout(), &palette, &mut buf);
        // Apr bar at x=4, full column (y=0..15).
        assert_eq!(buf.get(4, 0),  Some(ORANGE));
        assert_eq!(buf.get(4, 15), Some(ORANGE));
        // Outside the bar, untouched.
        assert_eq!(buf.get(3, 0), None);
    }

    #[test]
    fn paints_only_below_bar_top() {
        let mut buf = crate::PixelBuffer::new(16, 16);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("Jan".into(), ORANGE);
        rasterize_vertical(&fake_layout(), &palette, &mut buf);
        // Jan bar starts at pixel_y=7, height=9 → fills y=7..15.
        assert_eq!(buf.get(0, 7),  Some(ORANGE));
        assert_eq!(buf.get(0, 15), Some(ORANGE));
        assert_eq!(buf.get(0, 6),  None);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod vertical;
pub use vertical::rasterize_vertical;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib rasterize::vertical
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/vertical.rs (above tests)
use crate::layout::VerticalBarLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_vertical(
    layout:  &VerticalBarLayout,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    for bar in &layout.bars {
        if bar.pixel_height == 0 || bar.pixel_width == 0 { continue; }
        let color = palette.get(&bar.series_key)
            .copied()
            .unwrap_or(RgbColor { r: 0x76, g: 0x76, b: 0x76 });
        let x0 = bar.pixel_x;
        let x1 = bar.pixel_x + bar.pixel_width.saturating_sub(1);
        let y0 = bar.pixel_y;
        let y1 = bar.pixel_y + bar.pixel_height.saturating_sub(1);
        buf.fill_rect(x0, y0, x1, y1, color);
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib rasterize::vertical
```
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add vertical-orientation rasterizer for bars and histogram bins"
```

---

## Task 5: Histogram layout (`tplot-core`)

**Files:**
- Create: `crates/tplot-core/src/layout/histogram.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Histograms compute bin boundaries from the data range, count occurrences per bin, then reuse `VerticalBarLayout` (each bin → one bar). Default bin count uses Sturges' rule: `ceil(log2(N) + 1)`. Override via the `bins` argument.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/histogram.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn latency_df() -> DataFrame {
        // Synthetic latency distribution with a clear peak around 50ms.
        let values: Vec<f64> = vec![
            10.0, 22.0, 35.0, 41.0, 48.0, 49.0, 50.0, 50.0, 51.0, 52.0,
            55.0, 58.0, 60.0, 65.0, 80.0, 95.0, 110.0, 145.0, 220.0,
        ];
        DataFrame::from_columns(vec![
            Column::new("ms", Series::Numbers(values)),
        ]).unwrap()
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
        let modal = layout.bars.bars.iter().max_by_key(|b| b.pixel_height).unwrap();
        assert!(modal.label.contains("4") || modal.label.contains("5") || modal.label.contains("6"),
            "modal bin label was {:?}", modal.label);
    }

    #[test]
    fn bin_labels_are_ranges() {
        let layout = layout_histogram(&latency_df(), "ms", Some(5), 80, 16).unwrap();
        for bar in &layout.bars.bars {
            // Labels look like "10–52" (range form).
            assert!(bar.label.contains('–') || bar.label.contains('-'),
                "label {:?} should be a range", bar.label);
        }
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod histogram;
pub use histogram::{layout_histogram, HistogramLayout, HistogramError};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::histogram
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/histogram.rs (above tests)
use crate::dataframe::{Column, DataFrame, Series};
use crate::layout::vertical_bar::{layout_vertical_bar, VerticalBarLayout, VerticalBarLayoutError};

#[derive(Debug, Clone)]
pub struct HistogramLayout {
    pub bars:       VerticalBarLayout,
    pub bin_count:  usize,
    pub bin_width:  f64,
    pub data_min:   f64,
    pub data_max:   f64,
}

#[derive(Debug, thiserror::Error)]
pub enum HistogramError {
    #[error("column `{0}` must be numeric")]
    NonNumeric(String),
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    Layout(#[from] VerticalBarLayoutError),
}

pub fn layout_histogram(
    df:             &DataFrame,
    value_col:      &str,
    bins:           Option<usize>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<HistogramLayout, HistogramError> {
    let values: Vec<f64> = match df.column(value_col)
        .map_err(|_| HistogramError::Empty)?.series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(HistogramError::NonNumeric(value_col.to_string())),
    };
    if values.is_empty() { return Err(HistogramError::Empty); }

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
        if idx >= bin_count { idx = bin_count - 1; } // pin upper boundary
        counts[idx] += 1;
    }

    // Build a synthetic DataFrame: one row per bin, label = "lo–hi", value = count.
    let labels: Vec<String> = (0..bin_count).map(|i| {
        let lo = data_min + i as f64       * bin_width;
        let hi = data_min + (i + 1) as f64 * bin_width;
        format!("{lo:.0}–{hi:.0}")
    }).collect();
    let count_floats: Vec<f64> = counts.iter().map(|c| *c as f64).collect();

    let bar_df = DataFrame::from_columns(vec![
        Column::new("__bin__",   Series::Strings(labels)),
        Column::new("__count__", Series::Numbers(count_floats)),
    ]).map_err(|_| HistogramError::Empty)?;

    let bars = layout_vertical_bar(&bar_df, "__bin__", "__count__", None,
        canvas_cells_w, canvas_cells_h)?;

    Ok(HistogramLayout { bars, bin_count, bin_width, data_min, data_max })
}

/// Sturges' rule: ceil(log2(N) + 1). Standard textbook default.
fn sturges(n: usize) -> usize {
    if n < 2 { return 1; }
    ((n as f64).log2() + 1.0).ceil() as usize
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::histogram
```
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add histogram layout with Sturges'-rule auto-binning"
```

---

## Task 6: Story-pass — modal bin detection (`tplot-story`)

**Files:**
- Modify: `crates/tplot-story/src/focal.rs`
- Modify: `crates/tplot-story/src/takeaway.rs`
- Modify: `crates/tplot-story/src/lib.rs`

The histogram story-pass picks the modal bin (most observations) IF it dominates clearly (count >= 1.5× median bin count). Takeaway: "Most observations clustered in `<bin_label>` — `<pct>%` of the total."

- [ ] **Step 1: Write failing tests for the new takeaway**

```rust
// crates/tplot-story/src/takeaway.rs (within existing tests module)
#[test]
fn histogram_takeaway_names_modal_bin_and_pct() {
    let t = histogram_takeaway(Some("40–60"), 8, 19);
    assert!(t.contains("40–60"));
    assert!(t.contains("42%") || t.contains("43%") || t.contains("4"));
}

#[test]
fn histogram_takeaway_neutral_for_uniform() {
    let t = histogram_takeaway(None, 0, 0);
    assert!(t.to_lowercase().contains("evenly") || t.to_lowercase().contains("no clear"));
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot-story --lib takeaway
```
Expected: FAIL.

- [ ] **Step 3: Implement the takeaway template**

```rust
// crates/tplot-story/src/takeaway.rs (append at the end of the module)
pub fn histogram_takeaway(modal_bin: Option<&str>, modal_count: u64, total: u64) -> String {
    match modal_bin {
        None => "Distribution is roughly even — no clear cluster.".to_string(),
        Some(label) if total == 0 => format!("Most observations fell in {label}."),
        Some(label) => {
            let pct = (modal_count as f64 / total as f64 * 100.0).round() as i64;
            format!("Most observations clustered in {label} — {pct}% of the total.")
        }
    }
}
```

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot-story --lib takeaway
```
Expected: all green.

- [ ] **Step 5: Write failing tests for the histogram story-pass entry point**

```rust
// crates/tplot-story/src/lib.rs (within existing tests module)
#[test]
fn histogram_story_pass_picks_modal_bin() {
    let bins = vec![
        SeriesPoint { key: "0–10".into(),   value: 1.0 },
        SeriesPoint { key: "10–20".into(),  value: 2.0 },
        SeriesPoint { key: "20–30".into(),  value: 8.0 },
        SeriesPoint { key: "30–40".into(),  value: 4.0 },
        SeriesPoint { key: "40–50".into(),  value: 1.0 },
    ];
    let s = run_histogram_story_pass(&bins, &StoryConfig::default(), Palette::Signature);
    assert_eq!(s.focal.as_deref(), Some("20–30"));
    assert!(s.takeaway.unwrap().contains("20–30"));
}

#[test]
fn histogram_story_pass_neutral_when_uniform() {
    let bins = vec![
        SeriesPoint { key: "a".into(), value: 5.0 },
        SeriesPoint { key: "b".into(), value: 5.0 },
        SeriesPoint { key: "c".into(), value: 5.0 },
        SeriesPoint { key: "d".into(), value: 6.0 },
    ];
    let s = run_histogram_story_pass(&bins, &StoryConfig::default(), Palette::Signature);
    assert!(s.focal.is_none());
}
```

- [ ] **Step 6: Implement the story-pass entry point**

```rust
// crates/tplot-story/src/lib.rs (append after run_bar_story_pass)
pub fn run_histogram_story_pass(
    bins:    &[SeriesPoint],
    config:  &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = bins.iter()
            .map(|p| (p.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let total: u64 = bins.iter().map(|p| p.value as u64).sum();
    let focal_choice = match &config.focus {
        FocusMode::Auto         => pick_focal(bins),
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

    let keys: Vec<&str> = bins.iter().map(|p| p.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let modal_count = focal_name.and_then(|n| bins.iter()
            .find(|p| p.key == n).map(|p| p.value as u64)).unwrap_or(0);
        Some(takeaway::histogram_takeaway(focal_name, modal_count, total))
    };

    StoryAnnotated { focal: focal_name.map(String::from), palette_map, takeaway }
}
```

- [ ] **Step 7: Run (expected pass)**

```bash
cargo test -p tplot-story --lib
```
Expected: all green (existing + 2 new histogram tests).

- [ ] **Step 8: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add histogram story-pass with modal-bin detection and takeaway"
```

---

## Task 7: CLI — extend bar with `--vertical` (it already exists), add `hist` subcommand

**Files:**
- Modify: `crates/tplot/src/cli.rs`

The `--vertical` flag already exists on the bar subcommand from Plan 1 (it currently returns "vertical bars land in plan 2"). Now add the `hist` subcommand.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_hist_subcommand() {
    let args = Cli::parse_from([
        "tplot", "hist", "latencies.csv",
        "-x", "ms",
        "--bins", "20",
    ]);
    match args.command {
        Command::Hist(h) => {
            assert_eq!(h.input, "latencies.csv");
            assert_eq!(h.x, "ms");
            assert_eq!(h.bins, Some(20));
        }
        _ => panic!("expected Hist"),
    }
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot --lib cli
```
Expected: FAIL — `Command::Hist` doesn't exist.

- [ ] **Step 3: Add the Hist variant**

```rust
// crates/tplot/src/cli.rs — modify the Command enum and add HistArgs

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Render a bar chart from a CSV/JSON file.
    Bar(BarArgs),
    /// Render a histogram of a single numeric column.
    Hist(HistArgs),
    /// Read a JSON ChartSpec from stdin and render it.
    Json,
}

#[derive(Args, Debug)]
pub struct HistArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Numeric column to bin.
    #[arg(short = 'x')]
    pub x: String,
    /// Bin count. Default: Sturges' rule (ceil(log2(N)+1)).
    #[arg(long)]
    pub bins: Option<usize>,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}
```

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot --lib cli
```
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/tplot/
git commit -m "Add hist subcommand to CLI argument parser"
```

---

## Task 8: Bar pipeline — handle `--vertical`

**Files:**
- Modify: `crates/tplot/src/commands/bar.rs`

Replace the `if opts.vertical { return Err(...) }` placeholder with a real vertical-bar pipeline.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/bar.rs (within existing tests module)
#[test]
fn renders_vertical_bars_with_focal() {
    use tplot_core::input::parse_csv_str;
    let csv = "month,active\nJan,12.4\nFeb,13.1\nMar,18.2\nApr,21.4\nMay,17.9\nJun,9.0\n";
    let df = parse_csv_str(csv).unwrap();
    let opts = RenderOptions {
        x: "month".into(), y: "active".into(),
        group: None,
        vertical: true,
        focus: None, annotate: None,
        neutral: false, no_takeaway: false,
        width: Some(60), height: 16,
        palette_name: "signature".into(),
    };
    let out = render_bar(&df, &opts).unwrap();
    // Apr is the max → focal color (burnt orange) should appear.
    assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color escape");
    assert!(out.contains("Apr"));
    // Vertical bars use the lower-block glyphs.
    assert!(out.contains('\u{2588}') || out.contains('\u{2587}') || out.contains('\u{2585}'),
        "no lower-block glyphs in vertical bar output");
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot --lib commands::bar::tests::renders_vertical_bars_with_focal
```
Expected: FAIL — current code returns "vertical bars land in plan 2" error.

- [ ] **Step 3: Replace the placeholder with the real vertical pipeline**

```rust
// crates/tplot/src/commands/bar.rs — locate the early `if opts.vertical { ... }` block
// and REPLACE it with a call to the new helper.

// At the top of render_bar (replacing the old early-return):
if opts.vertical {
    return render_vertical_bar(df, opts);
}
```

Add the helper function below `render_bar`:

```rust
// crates/tplot/src/commands/bar.rs (append)
use tplot_core::layout::layout_vertical_bar;
use tplot_core::rasterize::rasterize_vertical;
use tplot_render::render_vertical_blocks;

fn render_vertical_bar(df: &DataFrame, opts: &RenderOptions) -> Result<String> {
    // ----- aggregate -------------------------------------------------------
    let group_col = opts.group.as_deref().unwrap_or(&opts.x);
    let labels: Vec<String> = match df.column(group_col)
        .map_err(|e| anyhow!(e.to_string()))?.series()
    {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(&opts.y)
        .map_err(|e| anyhow!(e.to_string()))?.series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("y column `{}` must be numeric", opts.y)),
    };

    let mut series_points: Vec<SeriesPoint> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(p) = series_points.iter_mut().find(|p| p.key == *l) {
            p.value += v;
        } else {
            series_points.push(SeriesPoint { key: l.clone(), value: *v });
        }
    }

    // ----- story-pass ------------------------------------------------------
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let story = run_bar_story_pass(&series_points, &story_cfg, palette);

    // ----- layout ----------------------------------------------------------
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;
    let agg_df = DataFrame::from_columns(vec![
        Column::new("__label__", Series::Strings(series_points.iter().map(|p| p.key.clone()).collect())),
        Column::new("__value__", Series::Numbers(series_points.iter().map(|p| p.value).collect())),
    ]).map_err(|e| anyhow!(e.to_string()))?;
    let layout = layout_vertical_bar(&agg_df, "__label__", "__value__", None, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_vertical(&layout, &story.palette_map, &mut buf);

    // ----- render to vertical-blocks ---------------------------------------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_vertical_blocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose ---------------------------------------------------------
    // Each chart row gets a y-axis label (left margin), then the rendered row.
    // Below the chart: x-axis labels (one per bar), centered under each bar.
    let mut out = String::new();
    let max_value = layout.bars.iter().map(|b| b.value).fold(f64::MIN, f64::max);
    let n_rows = body_lines.len();

    for (i, line) in body_lines.iter().enumerate() {
        // Y-axis label: print value at top, half-value mid, 0 at bottom.
        let y_label = if i == 0 {
            format!("{:>5.0}", max_value)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", max_value / 2.0)
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

    // X-axis labels row: one label per bar column. Truncate labels that
    // are wider than the per-group budget so neighboring labels stay aligned.
    let group_w = layout.bar_cell_width + layout.gap_cell_width;
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout.bars.first().map(|b| b.pixel_x).unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    for (idx, bar) in layout.bars.iter().enumerate() {
        let max_label_w = if idx + 1 < layout.bars.len() { group_w } else { layout.bar_cell_width };
        let trimmed: String = bar.label.chars().take(max_label_w).collect();
        let pad_left  = layout.bar_cell_width.saturating_sub(trimmed.chars().count()) / 2;
        let pad_right = layout.bar_cell_width.saturating_sub(trimmed.chars().count() + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.bars.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}
```

Note: `Column`, `DataFrame`, `Series`, `Palette`, `StoryConfig`, `FocusMode`, `SeriesPoint`, `PixelBuffer`, `Capabilities`, `run_bar_story_pass`, `detected_terminal_size`, `anyhow!`, `Result` are all imported at the top of the file from Plan 1's work.

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot --lib commands::bar
```
Expected: all green (Plan-1 horizontal bar tests still pass + new vertical test passes).

- [ ] **Step 5: Smoke test**

```bash
cargo run -p tplot -- bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical
```
Expected: vertical bar chart with the largest quarter (likely Q4 with 173 sum) in burnt-orange and others in gray. Visually inspect and confirm bars are vertical with sub-cell smoothness.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Wire vertical bar pipeline into bar subcommand"
```

---

## Task 9: Histogram subcommand — full pipeline

**Files:**
- Create: `crates/tplot/src/commands/histogram.rs`
- Modify: `crates/tplot/src/commands/mod.rs`
- Modify: `crates/tplot/src/main.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/histogram.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_histogram_with_focal_modal_bin() {
        // Synthetic latency data: clear peak in 40-60ms range.
        let csv = "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = HistogramOptions {
            x: "ms".into(),
            bins: Some(7),
            focus: None, annotate: None,
            neutral: false, no_takeaway: false,
            width: Some(80), height: 16,
            palette_name: "signature".into(),
        };
        let out = render_histogram(&df, &opts).unwrap();
        assert!(out.contains("\x1b[38;2;238;123;61m"), "no focal color");
        assert!(out.contains("Most observations clustered in"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod histogram;
pub use histogram::{render_histogram, HistogramOptions};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot --lib commands::histogram
```
Expected: FAIL — `render_histogram` not found.

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/histogram.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_histogram;
use tplot_core::rasterize::rasterize_vertical;
use tplot_core::PixelBuffer;
use tplot_render::render_vertical_blocks;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_story::{run_histogram_story_pass, SeriesPoint};

#[derive(Debug, Clone)]
pub struct HistogramOptions {
    pub x:            String,
    pub bins:         Option<usize>,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub width:        Option<usize>,
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_histogram(df: &DataFrame, opts: &HistogramOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;

    // Compute the histogram layout (this also produces the per-bin labels +
    // counts as a synthetic vertical bar layout).
    let hist = layout_histogram(df, &opts.x, opts.bins, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Run the story-pass over the bins.
    let bins_as_points: Vec<SeriesPoint> = hist.bars.bars.iter()
        .map(|b| SeriesPoint { key: b.label.clone(), value: b.value })
        .collect();
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let story = run_histogram_story_pass(&bins_as_points, &story_cfg, palette);

    // Rasterize.
    let mut buf = PixelBuffer::new(hist.bars.plot_box.pixel_width, hist.bars.plot_box.pixel_height);
    rasterize_vertical(&hist.bars, &story.palette_map, &mut buf);

    // Render via vertical-block glyphs.
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_vertical_blocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // Compose: y-axis labels (max count, half, 0), then bar labels under bars.
    let mut out = String::new();
    let max_count = hist.bars.bars.iter().map(|b| b.value).fold(f64::MIN, f64::max);
    let n_rows = body_lines.len();

    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", max_count)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", max_count / 2.0)
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

    // Bin-range labels under the bars.
    let group_w = hist.bars.bar_cell_width + hist.bars.gap_cell_width;
    out.push_str(&" ".repeat(hist.bars.left_margin));
    let leading = hist.bars.bars.first().map(|b| b.pixel_x).unwrap_or(0);
    out.push_str(&" ".repeat(leading));
    for (idx, bar) in hist.bars.bars.iter().enumerate() {
        let trimmed: String = bar.label.chars().take(group_w.saturating_sub(1)).collect();
        let pad_left  = hist.bars.bar_cell_width.saturating_sub(trimmed.chars().count()) / 2;
        let pad_right = hist.bars.bar_cell_width
            .saturating_sub(trimmed.chars().count() + pad_left);
        out.push_str(&" ".repeat(pad_left));
        out.push_str(&trimmed);
        out.push_str(&" ".repeat(pad_right));
        if idx + 1 < hist.bars.bars.len() {
            out.push_str(&" ".repeat(hist.bars.gap_cell_width));
        }
    }
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(hist.bars.left_margin + 1));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}
```

- [ ] **Step 5: Wire into main**

```rust
// crates/tplot/src/main.rs — replace the existing match block

match args.command {
    Command::Bar(b) => {
        let df = pipeline::read_dataframe(&b.input)?;
        let (_, h) = pipeline::detected_terminal_size(b.common.width);
        let out = render_bar(&df, &RenderOptions {
            x: b.x, y: b.y, group: b.group,
            vertical: b.vertical,
            focus: b.common.focus, annotate: b.common.annotate,
            neutral: b.common.neutral, no_takeaway: b.common.no_takeaway,
            width: b.common.width, height: h,
            palette_name: b.common.palette,
        })?;
        print!("{out}");
        Ok(())
    }
    Command::Hist(h) => {
        let df = pipeline::read_dataframe(&h.input)?;
        let (_, height) = pipeline::detected_terminal_size(h.common.width);
        let out = render_histogram(&df, &HistogramOptions {
            x: h.x, bins: h.bins,
            focus: h.common.focus, annotate: h.common.annotate,
            neutral: h.common.neutral, no_takeaway: h.common.no_takeaway,
            width: h.common.width, height,
            palette_name: h.common.palette,
        })?;
        print!("{out}");
        Ok(())
    }
    Command::Json => {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        let (w, h) = pipeline::detected_terminal_size(None);
        let out = commands::render_from_json(&buf, w, h)?;
        print!("{out}");
        Ok(())
    }
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot --lib commands
cargo build --workspace
```
Expected: all green.

- [ ] **Step 7: Smoke test**

```bash
# Synthetic latency data via shell
seq 1 100 | awk 'BEGIN{srand(42)} { print int(50 + 30*sin($1/8) + 20*(rand()-0.5)) }' \
  | sed '1iLATENCY_MS' \
  | tr 'A-Z_' 'a-z_' \
  | cargo run -p tplot -- hist - -x latency_ms
```
Or simpler:
```bash
{ echo "ms"; seq -50 50; } | cargo run -p tplot -- hist - -x ms
```
Expected: a visible histogram with auto-binned ranges and a focal bin (whichever has the highest count) in burnt orange + a "Most observations clustered in …" takeaway.

- [ ] **Step 8: Commit**

```bash
git add crates/tplot/
git commit -m "Add hist subcommand wiring with story-pass and vertical-block rendering"
```

---

## Task 10: Update `tplot-core::layout` PlotBox export

**Files:**
- Modify: `crates/tplot-core/src/layout/mod.rs`

`PlotBox` was originally only exported from `bar.rs`. The vertical bar layout re-exports it via `pub use crate::layout::bar::PlotBox`. Confirm both `BarLayout::plot_box` and `VerticalBarLayout::plot_box` reference the SAME type — no duplicates.

- [ ] **Step 1: Inspect current state**

```bash
grep -rn "pub struct PlotBox" crates/tplot-core/src/
```

Expected: a SINGLE definition in `crates/tplot-core/src/layout/bar.rs`. If the implementer accidentally added a second one in `vertical_bar.rs`, fix that now: remove the duplicate definition and re-export from `bar.rs`.

- [ ] **Step 2: Confirm the canonical export**

```rust
// crates/tplot-core/src/layout/mod.rs (ensure this is present)
pub use bar::PlotBox;
```

- [ ] **Step 3: Run all tests**

```bash
cargo test --workspace
```
Expected: all green.

- [ ] **Step 4: Commit (only if changes were needed)**

If any cleanup happened:
```bash
git add crates/tplot-core/
git commit -m "Consolidate PlotBox to a single canonical export"
```
Otherwise skip.

---

## Task 11: End-to-end snapshot tests for vertical bar + histogram

**Files:**
- Create: `crates/tplot/tests/e2e_vertical_and_hist.rs`

- [ ] **Step 1: Write the snapshot tests**

```rust
// crates/tplot/tests/e2e_vertical_and_hist.rs
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(2).unwrap().to_path_buf()
}

fn run(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args)
        .current_dir(workspace_root())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("tplot binary failed to launch");
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
}

fn run_with_stdin(args: &[&str], input: &str) -> String {
    use std::io::Write;
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
fn vertical_bar_default_snapshot() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--vertical", "--width", "80",
    ]);
    insta::assert_snapshot!("vertical_bar_default", strip_ansi(&out));
}

#[test]
fn histogram_default_snapshot() {
    let csv = "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
    let out = run_with_stdin(&["hist", "-", "-x", "ms", "--bins", "7", "--width", "80"], csv);
    insta::assert_snapshot!("histogram_default", strip_ansi(&out));
}

#[test]
fn histogram_neutral_snapshot() {
    let csv = "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
    let out = run_with_stdin(&["hist", "-", "-x", "ms", "--bins", "7", "--neutral", "--width", "80"], csv);
    insta::assert_snapshot!("histogram_neutral", strip_ansi(&out));
}
```

- [ ] **Step 2: Run the tests once to write initial snapshots**

```bash
cargo test -p tplot --test e2e_vertical_and_hist
```
Tests will fail (no existing snapshots). Inspect the new `.snap.new` files in `crates/tplot/tests/snapshots/`. They should look like:

- `vertical_bar_default`: a vertical bar chart with the largest quarter highlighted, x-axis labels (Q1/Q2/Q3/Q4) below, takeaway underneath.
- `histogram_default`: a histogram with 7 bin-range labels, the modal bin highlighted, takeaway underneath.
- `histogram_neutral`: same data, every bin in focal color, no takeaway.

If the snapshots look right, accept them:

```bash
cargo insta accept
```

- [ ] **Step 3: Re-run and confirm green**

```bash
cargo test -p tplot --test e2e_vertical_and_hist
```
Expected: 3 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/tests/
git commit -m "Add end-to-end snapshot tests for vertical bar and histogram"
```

---

## Task 12: README update + final lint pass

**Files:**
- Modify: `README.md`
- Workspace clippy + fmt

- [ ] **Step 1: Update the README** to mention vertical bars and histograms

Replace the "What's in this version (Plan 1)" section with:

````markdown
## What's in this version (Plan 1 + 2)

- Three chart types: horizontal bar, vertical bar, histogram.
- Renderers: half-blocks (truecolor + 256/16/mono fallback) for horizontal bars; vertical-block elements (`▁▂▃▄▅▆▇█`) for vertical bars and histograms.
- Story-pass: focal-series detection, gray-down palette, embedded takeaway line. Histograms get a modal-bin treatment with a "Most observations clustered in …" takeaway.
- CLI: `tplot bar [--vertical] FILE -x col -y col --group col`, `tplot hist FILE -x col [--bins N]`, `tplot json` (stdin).

Line / scatter / area / sparkline / heatmap / box plot arrive in subsequent plans.
````

Add a new quickstart line:

````markdown
```bash
# Histogram of latency values
echo "ms"; seq 1 200 | awk '{print int(50 + 30*sin($1/8) + 30*(rand()-0.5))}' | tplot hist - -x ms
```
````

- [ ] **Step 2: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 3: Commit**

```bash
git add README.md
# If fmt/clippy produced changes:
git add -A
git commit -m "Update README and lint pass for plan 2"
```

---

## Done — Plan 2 deliverable

You should now be able to:

```bash
# Vertical bars
tplot bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical

# Histogram of synthetic data
{ echo "ms"; seq -50 50; } | tplot hist - -x ms

# Histogram with explicit bins
tplot hist some-numeric.csv -x value --bins 25
```

Both new chart types use sub-cell vertical resolution (8 steps per cell via `▁▂▃▄▅▆▇█`), apply the storytelling treatment by default, and accept the same `--neutral`, `--focus`, `--annotate`, `--palette`, `--width` flags as the horizontal bar.

**Still missing for v1** (later plans):
- Line, scatter (Plan 3 — Braille renderer)
- Stacked area, sparkline, heatmap, box plot (Plan 4)
- `--graphics` image-protocol output (Plan 5)
- Full capability detection / OSC probing (Plan 5)
- Distribution / Python wrapper (Plan 6)

## Self-review notes

Coverage check against the deliverables stated up top:
- Vertical-block renderer (Task 1) ✓
- ChartKind::Histogram protocol type (Task 2) ✓
- Vertical bar layout (Task 3) ✓
- Vertical-orientation rasterizer (Task 4) ✓
- Histogram layout with binning (Task 5) ✓
- Story-pass extension for histograms (Task 6) ✓
- CLI subcommand for hist (Task 7) ✓
- Vertical bar pipeline (Task 8) ✓
- Histogram pipeline (Task 9) ✓
- Snapshot tests (Task 11) ✓
- README + lint (Task 12) ✓
- Capability auto-detection note: the existing `Capabilities::from_vars` already detects `Octants` glyph_set on truecolor terminals; for v1's vertical-block renderer, that's enough. True Octants probing (the OSC dance) is Plan 5 work.
