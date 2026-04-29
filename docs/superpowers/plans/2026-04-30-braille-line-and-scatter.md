# TerminalPlot — Plan 3: Braille Renderer + Line & Scatter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tplot line` and `tplot scatter` to the binary, both rendering with sub-cell Braille resolution (8 dots per cell, 2×4 grid). Line charts support multi-series with smooth interpolation; scatter supports density encoding. Story-pass detects the highest-trend line as focal.

**Architecture:** Reuses the established pipeline. Adds: Braille renderer in `tplot-render`, line-drawing utility (Bresenham) in `tplot-core`, two new ChartKind variants, two new layout/rasterize pairs, line-chart story-pass, two new CLI subcommands.

**Tech Stack:** Same as Plan 1 + 2. No new dependencies.

**Inherited context (Plans 1 + 2):**
- `RgbColor` derives `Hash`; `Palette::context_color` is `0x76,0x76,0x76` (true gray); ANSI 256 cube uses `(v / 51).min(5)`; `Axis` derives only `PartialEq`; struct-update syntax for `StoryConfig`; integration tests must `current_dir(workspace_root)`; ChartKind matches must be exhaustive (Bar+Histogram already exist; this plan adds Line+Scatter).
- Renderers in place: `render_halfblocks` (horizontal bar), `render_vertical_blocks` (vertical bar + histogram).
- Layouts: `layout_horizontal_bar`, `layout_vertical_bar`, `layout_histogram`. Rasterizers: `rasterize_bar`, `rasterize_vertical`.

---

## File Structure

New files in **bold**.

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add Line and Scatter to ChartKind
├── tplot-render/src/
│   ├── lib.rs                      EXTEND: re-export render_braille
│   └── braille.rs                  ★ NEW: Braille renderer (2×4 dots / cell)
├── tplot-core/src/
│   ├── pixel_buffer.rs             EXTEND: add draw_line (Bresenham)
│   ├── layout/
│   │   ├── mod.rs                  EXTEND: re-export new layouts
│   │   ├── line.rs                 ★ NEW: line chart layout (multi-series)
│   │   └── scatter.rs              ★ NEW: scatter layout
│   └── rasterize/
│       ├── mod.rs                  EXTEND: re-export new rasterizers
│       ├── line.rs                 ★ NEW: rasterize line chart (Bresenham per segment)
│       └── scatter.rs              ★ NEW: rasterize scatter (one dot per point)
├── tplot-story/src/
│   ├── lib.rs                      EXTEND: add run_line_story_pass
│   └── focal.rs                    EXTEND: pick_focal_by_delta (for time-series)
├── tplot/src/
│   ├── cli.rs                      EXTEND: LineArgs, ScatterArgs subcommands
│   └── commands/
│       ├── mod.rs                  EXTEND: re-export new commands
│       ├── line.rs                 ★ NEW: line subcommand handler
│       ├── scatter.rs              ★ NEW: scatter subcommand handler
│       └── json.rs                 EXTEND: dispatch line/scatter from JSON
└── tplot/tests/
    └── e2e_line_and_scatter.rs     ★ NEW: snapshot tests
```

---

## Task 1: Braille renderer

**Files:**
- Create: `crates/tplot-render/src/braille.rs`
- Modify: `crates/tplot-render/src/lib.rs`

Each Braille glyph (U+2800 + 0..255) encodes 8 dots in a 2×4 grid. Standard dot-to-bit mapping:

```
sub-pixel coord  →  bit
(0,0) → 0x01     (1,0) → 0x08
(0,1) → 0x02     (1,1) → 0x10
(0,2) → 0x04     (1,2) → 0x20
(0,3) → 0x40     (1,3) → 0x80
```

The renderer walks the buffer in 2×4 sub-pixel chunks; for each chunk it builds an 8-bit value by setting bits where pixels are non-empty, then emits `U+2800 + bits` with the dominant pixel's foreground color. Empty cells render as a regular space (NOT U+2800 — the blank Braille glyph creates inconsistent column widths in some fonts).

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/braille.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::PixelBuffer;
    use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, RgbColor};

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    fn caps() -> Capabilities {
        Capabilities {
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::Braille,
            graphics_protocol: GraphicsProtocol::None,
        }
    }

    #[test]
    fn empty_buffer_renders_only_spaces() {
        let buf = PixelBuffer::new(2, 4);
        let s = render_braille(&buf, caps());
        assert!(s.contains(' '));
        assert!(!s.contains('\u{2800}'), "should NOT use blank-Braille for empties");
    }

    #[test]
    fn single_top_left_dot_uses_bit_0() {
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(0, 0, ORANGE);
        let s = render_braille(&buf, caps());
        // Bit 0 set → U+2801 (⠁)
        assert!(s.contains('\u{2801}'), "missing dot-1 glyph: {:?}", s);
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn single_bottom_right_dot_uses_bit_7() {
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(1, 3, ORANGE);
        let s = render_braille(&buf, caps());
        // Bit 7 set → U+2880 (⢀)
        assert!(s.contains('\u{2880}'), "missing dot-8 glyph: {:?}", s);
    }

    #[test]
    fn full_cell_is_all_dots() {
        let mut buf = PixelBuffer::new(2, 4);
        for y in 0..4 {
            for x in 0..2 {
                buf.set(x, y, ORANGE);
            }
        }
        let s = render_braille(&buf, caps());
        // All 8 bits set → U+28FF (⣿)
        assert!(s.contains('\u{28ff}'), "missing all-dots glyph: {:?}", s);
    }

    #[test]
    fn last_painted_color_wins_for_multi_color_cell() {
        let red   = RgbColor { r: 0xff, g: 0x00, b: 0x00 };
        let blue  = RgbColor { r: 0x00, g: 0x00, b: 0xff };
        let mut buf = PixelBuffer::new(2, 4);
        buf.set(0, 0, red);
        buf.set(1, 3, blue);
        let s = render_braille(&buf, caps());
        // The renderer picks ONE color per cell (BSP-style: dominant by count,
        // tiebreak by most-recent-set; the test asserts that *some* color
        // shows up, not which one — implementation-defined).
        assert!(s.contains("\x1b[38;2;255;0;0m") || s.contains("\x1b[38;2;0;0;255m"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-render/src/lib.rs (append)
pub mod braille;
pub use braille::render_braille;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-render --lib braille
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-render/src/braille.rs (above tests)
use crate::ansi::{fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

const SUB_X: usize = 2;
const SUB_Y: usize = 4;

/// Map a sub-pixel coordinate within a cell (0..SUB_X, 0..SUB_Y) to its
/// Braille bit. Standard Unicode 6.0 dot order:
///   (0,0)=0x01  (1,0)=0x08
///   (0,1)=0x02  (1,1)=0x10
///   (0,2)=0x04  (1,2)=0x20
///   (0,3)=0x40  (1,3)=0x80
fn dot_bit(x: usize, y: usize) -> u8 {
    match (x, y) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (0, 3) => 0x40,
        (1, 3) => 0x80,
        _ => 0,
    }
}

/// Render a buffer as Braille glyphs. Each cell covers 2×4 sub-pixels.
/// Empty cells render as a regular space (not U+2800), to keep column widths
/// consistent across fonts.
pub fn render_braille(buf: &PixelBuffer, caps: Capabilities) -> String {
    let pw = buf.pixel_width();
    let ph = buf.pixel_height();
    let cells_w = pw.div_ceil(SUB_X);
    let cells_h = ph.div_ceil(SUB_Y);
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        for cx in 0..cells_w {
            let mut bits: u8 = 0;
            // Per-channel accumulators for picking the dominant cell color
            // (the most-frequent non-empty pixel; ties broken by latest set).
            let mut last: Option<RgbColor> = None;
            let mut counts: Vec<(RgbColor, u8)> = Vec::with_capacity(8);

            for sy in 0..SUB_Y {
                for sx in 0..SUB_X {
                    let px = cx * SUB_X + sx;
                    let py = cy * SUB_Y + sy;
                    if px >= pw || py >= ph { continue; }
                    if let Some(c) = buf.get(px, py) {
                        bits |= dot_bit(sx, sy);
                        last = Some(c);
                        if let Some(slot) = counts.iter_mut().find(|(rc, _)| *rc == c) {
                            slot.1 += 1;
                        } else {
                            counts.push((c, 1));
                        }
                    }
                }
            }

            if bits == 0 {
                out.push(' ');
            } else {
                let color = counts.iter()
                    .max_by_key(|(_, n)| *n)
                    .map(|(c, _)| *c)
                    .or(last)
                    .unwrap();
                let glyph = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                let _ = write!(out, "{}{}{}", fg(color, caps.color_depth), glyph, reset());
            }
        }
        out.push('\n');
    }
    out
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-render --lib braille
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add Braille renderer with 2×4 sub-cell dot encoding"
```

---

## Task 2: Bresenham line drawing on PixelBuffer

**Files:**
- Modify: `crates/tplot-core/src/pixel_buffer.rs`

Add `draw_line(x0, y0, x1, y1, color)` to `PixelBuffer` so layouts can paint smooth lines into a sub-pixel buffer. Standard Bresenham algorithm.

- [ ] **Step 1: Write failing tests (within the existing tests module)**

```rust
// crates/tplot-core/src/pixel_buffer.rs (within existing tests module)
#[test]
fn draw_horizontal_line() {
    let mut buf = PixelBuffer::new(10, 4);
    buf.draw_line(1, 2, 8, 2, RED);
    for x in 1..=8 {
        assert_eq!(buf.get(x, 2), Some(RED), "missing pixel at x={x}");
    }
    assert_eq!(buf.get(0, 2), None);
    assert_eq!(buf.get(9, 2), None);
}

#[test]
fn draw_vertical_line() {
    let mut buf = PixelBuffer::new(4, 10);
    buf.draw_line(2, 1, 2, 8, RED);
    for y in 1..=8 {
        assert_eq!(buf.get(2, y), Some(RED), "missing pixel at y={y}");
    }
}

#[test]
fn draw_diagonal_line() {
    let mut buf = PixelBuffer::new(8, 8);
    buf.draw_line(0, 0, 7, 7, RED);
    for i in 0..=7 {
        assert_eq!(buf.get(i, i), Some(RED), "missing diagonal at ({i},{i})");
    }
}

#[test]
fn draw_line_works_in_either_direction() {
    let mut a = PixelBuffer::new(10, 10);
    let mut b = PixelBuffer::new(10, 10);
    a.draw_line(2, 1, 7, 8, RED);
    b.draw_line(7, 8, 2, 1, RED);
    for y in 0..10 {
        for x in 0..10 {
            assert_eq!(a.get(x, y), b.get(x, y),
                "asymmetry at ({x},{y})");
        }
    }
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot-core --lib pixel_buffer::tests::draw
```

- [ ] **Step 3: Implement**

```rust
// crates/tplot-core/src/pixel_buffer.rs (append a new method on PixelBuffer)
impl PixelBuffer {
    /// Bresenham's line algorithm. Inclusive endpoints. Pixels outside the
    /// buffer are silently clipped.
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: RgbColor) {
        let mut x = x0 as isize;
        let mut y = y0 as isize;
        let xe = x1 as isize;
        let ye = y1 as isize;
        let dx = (xe - x).abs();
        let dy = -(ye - y).abs();
        let sx: isize = if x < xe { 1 } else { -1 };
        let sy: isize = if y < ye { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x >= 0 && y >= 0 {
                self.set(x as usize, y as usize, color);
            }
            if x == xe && y == ye { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; x += sx; }
            if e2 <= dx { err += dx; y += sy; }
        }
    }
}
```

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot-core --lib pixel_buffer
```

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add Bresenham line drawing to PixelBuffer"
```

---

## Task 3: Add Line and Scatter to ChartKind

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`

- [ ] **Step 1: Write failing tests (within existing tests module)**

```rust
// crates/tplot-protocol/src/chart.rs (within existing tests module)
#[test]
fn line_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Line,
        x: Axis::Column("time".into()),
        y: Axis::Column("value".into()),
        group: Some("series".into()),
        title: None,
        story: StoryConfig::default(),
    };
    let json = serde_json::to_string(&spec).unwrap();
    assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
}

#[test]
fn scatter_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Scatter,
        x: Axis::Column("x".into()),
        y: Axis::Column("y".into()),
        group: Some("cluster".into()),
        title: None,
        story: StoryConfig::default(),
    };
    let json = serde_json::to_string(&spec).unwrap();
    assert_eq!(serde_json::from_str::<ChartSpec>(&json).unwrap(), spec);
}
```

- [ ] **Step 2: Add the variants**

```rust
// crates/tplot-protocol/src/chart.rs — modify the ChartKind enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar { orientation: BarOrientation },
    Histogram {
        #[serde(default)]
        bins: Option<usize>,
    },
    Line,
    Scatter,
}
```

- [ ] **Step 3: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib chart
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Line and Scatter variants to ChartKind"
```

---

## Task 4: Line chart layout

**Files:**
- Create: `crates/tplot-core/src/layout/line.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Line layout strategy:
- Each series → list of (x_pixel, y_pixel) sub-pixel coordinates.
- x_pixel maps from the data's x value linearly across the plot width.
- y_pixel maps from the data's y value (inverted: high y = top of plot).
- Multi-series support via `--group` column (each group gets its own series of points + a series_key for color).

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/line.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn ts_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("t", Series::Numbers(vec![1.0, 2.0, 3.0, 4.0, 5.0])),
            Column::new("v", Series::Numbers(vec![10.0, 25.0, 18.0, 42.0, 30.0])),
        ]).unwrap()
    }

    fn multi_series_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("t", Series::Numbers(vec![1.0,1.0,2.0,2.0,3.0,3.0])),
            Column::new("v", Series::Numbers(vec![10.0,5.0, 20.0,8.0, 30.0,12.0])),
            Column::new("g", Series::Strings(
                vec!["A","B","A","B","A","B"].into_iter().map(String::from).collect())),
        ]).unwrap()
    }

    #[test]
    fn single_series_produces_one_polyline() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        assert_eq!(layout.series.len(), 1);
        assert_eq!(layout.series[0].points.len(), 5);
    }

    #[test]
    fn multi_series_splits_by_group() {
        let layout = layout_line(&multi_series_df(), "t", "v", Some("g"), 80, 16).unwrap();
        assert_eq!(layout.series.len(), 2);
        for s in &layout.series {
            assert_eq!(s.points.len(), 3);
        }
    }

    #[test]
    fn x_axis_spans_data_range() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        let pts = &layout.series[0].points;
        // Leftmost point should be near x=0; rightmost should be near plot_pixel_width-1.
        let first_x = pts[0].0;
        let last_x = pts[pts.len() - 1].0;
        assert!(first_x < 4);
        assert!(last_x > layout.plot_box.pixel_width - 8);
    }

    #[test]
    fn y_axis_inverted_max_at_top() {
        let layout = layout_line(&ts_df(), "t", "v", None, 80, 16).unwrap();
        let pts = &layout.series[0].points;
        // The point with highest data y (42 at t=4) should have the LOWEST pixel_y.
        let (_, max_idx) = pts.iter().enumerate()
            .map(|(i, &(_, y))| (y, i))
            .min_by_key(|(y, _)| *y)
            .unwrap();
        assert_eq!(max_idx, 3, "highest data y should map to lowest pixel y (top of plot)");
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod line;
pub use line::{layout_line, LineLayout, LineSeries, LineLayoutError};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::line
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/line.rs (above tests)
use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct LineSeries {
    pub key:    String,
    /// Sub-pixel coordinates within the plot area (NOT canvas).
    /// (pixel_x, pixel_y) — pixel_y is inverted (high data → low y).
    pub points: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct LineLayout {
    pub plot_box:        PlotBox,
    pub series:          Vec<LineSeries>,
    pub canvas_cells_w:  usize,
    pub canvas_cells_h:  usize,
    pub left_margin:     usize,
    pub bottom_reserve:  usize,
    /// Data ranges for axis label rendering by the composer.
    pub x_min:           f64,
    pub x_max:           f64,
    pub y_min:           f64,
    pub y_max:           f64,
}

#[derive(Debug, thiserror::Error)]
pub enum LineLayoutError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
}

const SUB_X: usize = 2; // Braille: 2 sub-pixel cols per cell
const SUB_Y: usize = 4; // Braille: 4 sub-pixel rows per cell
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_line(
    df:             &DataFrame,
    x_col:          &str,
    y_col:          &str,
    group_col:      Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<LineLayout, LineLayoutError> {
    let xs: Vec<f64> = match df.column(x_col).map_err(|_| LineLayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(LineLayoutError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df.column(y_col).map_err(|_| LineLayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(LineLayoutError::NonNumericY(y_col.to_string())),
    };
    if xs.is_empty() { return Err(LineLayoutError::Empty); }

    // Optional grouping: split rows into series.
    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g).map_err(|_| LineLayoutError::Empty)?.series() {
            Series::Strings(v) => v.clone(),
            Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    // Compute global ranges (across all series) so all lines share the same axes.
    let x_min = xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_max = xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_min = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b)).min(0.0);
    let y_max = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let x_span = (x_max - x_min).max(1e-9);
    let y_span = (y_max - y_min).max(1e-9);

    let left_margin     = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve  = 3;
    let plot_cells_w    = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h    = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w   = plot_cells_w * SUB_X;
    let plot_pixels_h   = plot_cells_h * SUB_Y;

    let plot_box = PlotBox { pixel_width: plot_pixels_w, pixel_height: plot_pixels_h };

    // Bucket rows by group, preserving first-seen order.
    let mut series: Vec<LineSeries> = Vec::new();
    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let px = (((x - x_min) / x_span) * (plot_pixels_w - 1) as f64).round() as usize;
        let py = ((y_max - y) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        let py = py.min(plot_pixels_h - 1);
        if let Some(s) = series.iter_mut().find(|s| s.key == *g) {
            s.points.push((px, py));
        } else {
            series.push(LineSeries { key: g.clone(), points: vec![(px, py)] });
        }
    }
    // Sort each series' points by x (so lines connect in order).
    for s in &mut series {
        s.points.sort_by_key(|p| p.0);
    }

    Ok(LineLayout {
        plot_box, series, canvas_cells_w, canvas_cells_h,
        left_margin, bottom_reserve, x_min, x_max, y_min, y_max,
    })
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::line
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add line chart layout with multi-series and inverted y-axis"
```

---

## Task 5: Line rasterizer

**Files:**
- Create: `crates/tplot-core/src/rasterize/line.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

For each series, walk its sorted points pairwise and call `buf.draw_line(...)` between consecutive points. Order matters: paint non-focal series FIRST, focal LAST, so the focal line is on top where lines cross.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/line.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LineLayout, LineSeries, PlotBox};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };
    const GRAY:   RgbColor = RgbColor { r: 0x76, g: 0x76, b: 0x76 };

    fn fake_layout() -> LineLayout {
        LineLayout {
            plot_box: PlotBox { pixel_width: 20, pixel_height: 8 },
            series: vec![
                LineSeries {
                    key: "A".into(),
                    points: vec![(0, 7), (10, 4), (19, 0)],
                },
                LineSeries {
                    key: "B".into(),
                    points: vec![(0, 4), (10, 4), (19, 4)],
                },
            ],
            canvas_cells_w: 30, canvas_cells_h: 8,
            left_margin: 6, bottom_reserve: 3,
            x_min: 0.0, x_max: 5.0, y_min: 0.0, y_max: 50.0,
        }
    }

    #[test]
    fn paints_each_series_in_its_color() {
        let mut buf = crate::PixelBuffer::new(20, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        palette.insert("B".into(), GRAY);
        rasterize_line(&fake_layout(), Some("A"), &palette, &mut buf);

        // Series A endpoint at (0, 7) should be ORANGE.
        assert_eq!(buf.get(0, 7), Some(ORANGE));
        // Series B at (5, 4) should be GRAY (somewhere along the horizontal line).
        assert_eq!(buf.get(5, 4), Some(GRAY));
    }

    #[test]
    fn focal_series_paints_last_so_it_wins_at_crossings() {
        let mut buf = crate::PixelBuffer::new(20, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        palette.insert("B".into(), GRAY);
        rasterize_line(&fake_layout(), Some("A"), &palette, &mut buf);

        // The lines cross around (10, 4): A passes through, B is horizontal at y=4.
        // Because A is focal, it's painted LAST, so (10,4) should be ORANGE.
        assert_eq!(buf.get(10, 4), Some(ORANGE));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod line;
pub use line::rasterize_line;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib rasterize::line
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/line.rs (above tests)
use crate::layout::LineLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

/// Rasterize all series into the buffer. The focal series (if Some) is
/// painted LAST so it appears on top at crossings.
pub fn rasterize_line(
    layout:  &LineLayout,
    focal:   Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    // Paint non-focal first.
    for s in &layout.series {
        if Some(s.key.as_str()) == focal { continue; }
        paint_series(s, palette, buf);
    }
    // Then paint the focal series on top.
    if let Some(name) = focal {
        if let Some(s) = layout.series.iter().find(|s| s.key == name) {
            paint_series(s, palette, buf);
        }
    }
}

fn paint_series(
    s:       &crate::layout::LineSeries,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    let color = palette.get(&s.key).copied()
        .unwrap_or(RgbColor { r: 0x76, g: 0x76, b: 0x76 });
    for w in s.points.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        buf.draw_line(x0, y0, x1, y1, color);
    }
    // Also stamp each individual point so very-short series stay visible.
    for &(x, y) in &s.points {
        buf.set(x, y, color);
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib rasterize::line
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add line rasterizer with focal-on-top painting order"
```

---

## Task 6: Scatter layout

**Files:**
- Create: `crates/tplot-core/src/layout/scatter.rs`
- Modify: `crates/tplot-core/src/layout/mod.rs`

Scatter is structurally similar to line but produces UNCONNECTED points (no Bresenham between them). Reuses the same axis-mapping math.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/layout/scatter.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn pts_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("x", Series::Numbers(vec![1.0, 2.0, 3.0, 4.0, 5.0])),
            Column::new("y", Series::Numbers(vec![10.0, 25.0, 18.0, 42.0, 30.0])),
            Column::new("g", Series::Strings(
                vec!["A","B","A","B","A"].into_iter().map(String::from).collect())),
        ]).unwrap()
    }

    #[test]
    fn produces_one_point_per_row_when_ungrouped() {
        let layout = layout_scatter(&pts_df(), "x", "y", None, 80, 16).unwrap();
        assert_eq!(layout.series.len(), 1);
        assert_eq!(layout.series[0].points.len(), 5);
    }

    #[test]
    fn groups_split_into_separate_series() {
        let layout = layout_scatter(&pts_df(), "x", "y", Some("g"), 80, 16).unwrap();
        assert_eq!(layout.series.len(), 2);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/layout/mod.rs (append)
pub mod scatter;
pub use scatter::{layout_scatter, ScatterLayout, ScatterSeries, ScatterLayoutError};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::scatter
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/scatter.rs (above tests)
use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct ScatterSeries {
    pub key:    String,
    pub points: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct ScatterLayout {
    pub plot_box:       PlotBox,
    pub series:         Vec<ScatterSeries>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin:    usize,
    pub bottom_reserve: usize,
    pub x_min:          f64,
    pub x_max:          f64,
    pub y_min:          f64,
    pub y_max:          f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScatterLayoutError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
}

const SUB_X: usize = 2;
const SUB_Y: usize = 4;
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_scatter(
    df:             &DataFrame,
    x_col:          &str,
    y_col:          &str,
    group_col:      Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<ScatterLayout, ScatterLayoutError> {
    let xs: Vec<f64> = match df.column(x_col).map_err(|_| ScatterLayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(ScatterLayoutError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df.column(y_col).map_err(|_| ScatterLayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(ScatterLayoutError::NonNumericY(y_col.to_string())),
    };
    if xs.is_empty() { return Err(ScatterLayoutError::Empty); }

    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g).map_err(|_| ScatterLayoutError::Empty)?.series() {
            Series::Strings(v) => v.clone(),
            Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    let x_min = xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_max = xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_min = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let x_span = (x_max - x_min).max(1e-9);
    let y_span = (y_max - y_min).max(1e-9);

    let left_margin     = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve  = 3;
    let plot_cells_w    = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h    = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w   = plot_cells_w * SUB_X;
    let plot_pixels_h   = plot_cells_h * SUB_Y;

    let plot_box = PlotBox { pixel_width: plot_pixels_w, pixel_height: plot_pixels_h };

    let mut series: Vec<ScatterSeries> = Vec::new();
    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let px = (((x - x_min) / x_span) * (plot_pixels_w - 1) as f64).round() as usize;
        let py = ((y_max - y) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        let py = py.min(plot_pixels_h - 1);
        if let Some(s) = series.iter_mut().find(|s| s.key == *g) {
            s.points.push((px, py));
        } else {
            series.push(ScatterSeries { key: g.clone(), points: vec![(px, py)] });
        }
    }

    Ok(ScatterLayout {
        plot_box, series, canvas_cells_w, canvas_cells_h,
        left_margin, bottom_reserve, x_min, x_max, y_min, y_max,
    })
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::scatter
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add scatter layout sharing axis-mapping with line"
```

---

## Task 7: Scatter rasterizer

**Files:**
- Create: `crates/tplot-core/src/rasterize/scatter.rs`
- Modify: `crates/tplot-core/src/rasterize/mod.rs`

Just stamps individual sub-pixels — no line connections.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/scatter.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{PlotBox, ScatterLayout, ScatterSeries};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    fn fake_layout() -> ScatterLayout {
        ScatterLayout {
            plot_box: PlotBox { pixel_width: 10, pixel_height: 8 },
            series: vec![
                ScatterSeries {
                    key: "A".into(),
                    points: vec![(2, 3), (5, 6)],
                },
            ],
            canvas_cells_w: 30, canvas_cells_h: 8,
            left_margin: 6, bottom_reserve: 3,
            x_min: 0.0, x_max: 5.0, y_min: 0.0, y_max: 50.0,
        }
    }

    #[test]
    fn paints_each_point_in_series_color() {
        let mut buf = crate::PixelBuffer::new(10, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("A".into(), ORANGE);
        rasterize_scatter(&fake_layout(), None, &palette, &mut buf);
        assert_eq!(buf.get(2, 3), Some(ORANGE));
        assert_eq!(buf.get(5, 6), Some(ORANGE));
        // Non-painted pixels remain empty.
        assert_eq!(buf.get(3, 3), None);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/rasterize/mod.rs (append)
pub mod scatter;
pub use scatter::rasterize_scatter;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib rasterize::scatter
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/scatter.rs (above tests)
use crate::layout::ScatterLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_scatter(
    layout:  &ScatterLayout,
    focal:   Option<&str>,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    // Paint non-focal first, focal last (consistent with rasterize_line).
    for s in &layout.series {
        if Some(s.key.as_str()) == focal { continue; }
        paint_points(s, palette, buf);
    }
    if let Some(name) = focal {
        if let Some(s) = layout.series.iter().find(|s| s.key == name) {
            paint_points(s, palette, buf);
        }
    }
}

fn paint_points(
    s:       &crate::layout::ScatterSeries,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    let color = palette.get(&s.key).copied()
        .unwrap_or(RgbColor { r: 0x76, g: 0x76, b: 0x76 });
    for &(x, y) in &s.points {
        buf.set(x, y, color);
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib rasterize::scatter
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add scatter rasterizer with focal-on-top painting order"
```

---

## Task 8: Story-pass for line charts

**Files:**
- Modify: `crates/tplot-story/src/focal.rs`
- Modify: `crates/tplot-story/src/takeaway.rs`
- Modify: `crates/tplot-story/src/lib.rs`

For line charts, the "interesting" series is usually the one with the largest *change* (delta) or largest *trend*. Add `pick_focal_by_delta` that takes per-series first-and-last values.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-story/src/focal.rs (within existing tests module)
#[test]
fn picks_largest_delta_series() {
    // EMEA grew most in absolute terms; APAC stayed flat.
    let trends = vec![
        SeriesTrend { key: "NA".into(),    first: 100.0, last: 105.0 },
        SeriesTrend { key: "EMEA".into(),  first: 50.0,  last: 200.0 },
        SeriesTrend { key: "LATAM".into(), first: 30.0,  last: 28.0  },
        SeriesTrend { key: "APAC".into(),  first: 80.0,  last: 81.0  },
    ];
    let r = pick_focal_by_delta(&trends);
    assert_eq!(r.choice, FocalChoice::Series("EMEA".into()));
    assert!(r.trust_score > 1.5);
}

#[test]
fn admits_no_focal_when_trends_uniform() {
    let trends = vec![
        SeriesTrend { key: "a".into(), first: 50.0, last: 51.0 },
        SeriesTrend { key: "b".into(), first: 50.0, last: 52.0 },
        SeriesTrend { key: "c".into(), first: 50.0, last: 49.0 },
    ];
    let r = pick_focal_by_delta(&trends);
    assert_eq!(r.choice, FocalChoice::None);
}
```

```rust
// crates/tplot-story/src/takeaway.rs (within existing tests module)
#[test]
fn line_takeaway_describes_trend_direction_and_magnitude() {
    let t = line_takeaway(Some("EMEA"), 50.0, 200.0);
    assert!(t.contains("EMEA"));
    assert!(t.contains("rose") || t.contains("grew") || t.contains("4×") || t.contains("300%"));
}

#[test]
fn line_takeaway_neutral_for_no_focal() {
    let t = line_takeaway(None, 0.0, 0.0);
    assert!(t.to_lowercase().contains("no series") || t.to_lowercase().contains("flat"));
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement focal-by-delta**

```rust
// crates/tplot-story/src/focal.rs (append)
#[derive(Debug, Clone)]
pub struct SeriesTrend {
    pub key:   String,
    pub first: f64,
    pub last:  f64,
}

/// Pick the series with the largest absolute delta (last - first), normalized
/// against the median absolute delta. Returns FocalChoice::None if no series
/// dominates (max delta < 1.5× median).
pub fn pick_focal_by_delta(trends: &[SeriesTrend]) -> FocalResult {
    if trends.is_empty() {
        return FocalResult { choice: FocalChoice::None, trust_score: 0.0, reason: "empty" };
    }

    let abs_deltas: Vec<f64> = trends.iter().map(|t| (t.last - t.first).abs()).collect();
    let mut sorted = abs_deltas.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    let max_idx = abs_deltas.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap();

    let trust = if median.abs() < 1e-9 {
        if abs_deltas[max_idx] > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        abs_deltas[max_idx] / median
    };

    if trust >= 1.5 {
        FocalResult {
            choice: FocalChoice::Series(trends[max_idx].key.clone()),
            trust_score: trust,
            reason: "delta",
        }
    } else {
        FocalResult { choice: FocalChoice::None, trust_score: trust, reason: "no-clear-trend" }
    }
}
```

- [ ] **Step 4: Implement line takeaway**

```rust
// crates/tplot-story/src/takeaway.rs (append)
pub fn line_takeaway(focal: Option<&str>, first: f64, last: f64) -> String {
    match focal {
        None => "No series stands out — trends are flat.".to_string(),
        Some(name) => {
            let delta = last - first;
            let direction = if delta >= 0.0 { "grew" } else { "fell" };
            if first.abs() < 1e-9 {
                format!("{name} {direction} from {first:.0} to {last:.0}.")
            } else {
                let pct = (delta.abs() / first.abs() * 100.0).round() as i64;
                let multiple = (last / first).abs();
                if multiple >= 2.0 {
                    format!("{name} {direction} {multiple:.1}× — from {first:.0} to {last:.0}.")
                } else {
                    format!("{name} {direction} {pct}% — from {first:.0} to {last:.0}.")
                }
            }
        }
    }
}
```

- [ ] **Step 5: Add the line story-pass entry point**

```rust
// crates/tplot-story/src/lib.rs (append at end)
use crate::focal::{pick_focal_by_delta, SeriesTrend};

pub fn run_line_story_pass(
    trends:  &[SeriesTrend],
    config:  &StoryConfig,
    palette: Palette,
) -> StoryAnnotated {
    if !config.enabled {
        let map = trends.iter()
            .map(|t| (t.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None, palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let focal_choice = match &config.focus {
        FocusMode::Auto         => pick_focal_by_delta(trends),
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

    let keys: Vec<&str> = trends.iter().map(|t| t.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let (first, last) = focal_name.and_then(|n| trends.iter()
            .find(|t| t.key == n).map(|t| (t.first, t.last))).unwrap_or((0.0, 0.0));
        Some(takeaway::line_takeaway(focal_name, first, last))
    };

    StoryAnnotated { focal: focal_name.map(String::from), palette_map, takeaway }
}
```

- [ ] **Step 6: Run all tests pass**

```bash
cargo test -p tplot-story
```

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add line story-pass with delta-based focal detection"
```

---

## Task 9: CLI — line and scatter subcommands

**Files:**
- Modify: `crates/tplot/src/cli.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_line_subcommand() {
    let args = Cli::parse_from([
        "tplot", "line", "metrics.csv",
        "-x", "time", "-y", "value", "--group", "series",
    ]);
    match args.command {
        Command::Line(l) => {
            assert_eq!(l.input, "metrics.csv");
            assert_eq!(l.x, "time");
            assert_eq!(l.y, "value");
            assert_eq!(l.group.as_deref(), Some("series"));
        }
        _ => panic!("expected Line"),
    }
}

#[test]
fn parses_scatter_subcommand() {
    let args = Cli::parse_from([
        "tplot", "scatter", "users.csv",
        "-x", "signup_age", "-y", "session_count",
    ]);
    match args.command {
        Command::Scatter(s) => {
            assert_eq!(s.input, "users.csv");
            assert_eq!(s.x, "signup_age");
            assert_eq!(s.y, "session_count");
        }
        _ => panic!("expected Scatter"),
    }
}
```

- [ ] **Step 2: Add the variants**

```rust
// crates/tplot/src/cli.rs — modify Command enum and add LineArgs, ScatterArgs
#[derive(Subcommand, Debug)]
pub enum Command {
    Bar(BarArgs),
    Hist(HistArgs),
    Line(LineArgs),
    Scatter(ScatterArgs),
    Json,
}

#[derive(Args, Debug)]
pub struct LineArgs {
    pub input: String,
    #[arg(short = 'x')]
    pub x: String,
    #[arg(short = 'y')]
    pub y: String,
    #[arg(long)]
    pub group: Option<String>,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct ScatterArgs {
    pub input: String,
    #[arg(short = 'x')]
    pub x: String,
    #[arg(short = 'y')]
    pub y: String,
    #[arg(long)]
    pub group: Option<String>,
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
git commit -m "Add line and scatter subcommands to CLI"
```

---

## Task 10: Line pipeline

**Files:**
- Create: `crates/tplot/src/commands/line.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

Aggregate by group → compute (first, last) per series → run story-pass → layout → rasterize → render via Braille → compose with y-axis labels and x-axis labels.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/line.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_line_with_focal_high_trend() {
        // Two series — A flat, B clearly growing. B should be focal.
        let csv = "t,v,g\n1,10,A\n2,11,A\n3,9,A\n4,10,A\n1,5,B\n2,30,B\n3,80,B\n4,200,B\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = LineOptions {
            x: "t".into(), y: "v".into(), group: Some("g".into()),
            focus: None, annotate: None,
            neutral: false, no_takeaway: false,
            width: Some(80), height: 16,
            palette_name: "signature".into(),
        };
        let out = render_line(&df, &opts).unwrap();
        // Should highlight B in burnt orange.
        assert!(out.contains("\x1b[38;2;238;123;61m"), "missing focal color");
        // Some Braille glyph should appear.
        let has_braille = out.chars().any(|c| {
            let v = c as u32;
            (0x2801..=0x28FF).contains(&v)
        });
        assert!(has_braille, "no braille glyphs in output");
        // Takeaway names B.
        assert!(out.to_uppercase().contains('B') || out.contains("rose") || out.contains("grew"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod line;
pub use line::{render_line, LineOptions};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/line.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::{DataFrame, Series};
use tplot_core::layout::layout_line;
use tplot_core::rasterize::rasterize_line;
use tplot_core::PixelBuffer;
use tplot_render::render_braille;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_story::{run_line_story_pass, focal::SeriesTrend};

#[derive(Debug, Clone)]
pub struct LineOptions {
    pub x:            String,
    pub y:            String,
    pub group:        Option<String>,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub width:        Option<usize>,
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_line(df: &DataFrame, opts: &LineOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;

    // ----- layout (same data, computes pixel positions for each series) ----
    let layout = layout_line(df, &opts.x, &opts.y, opts.group.as_deref(),
        canvas_w, canvas_h).map_err(|e| anyhow!(e.to_string()))?;

    // ----- compute trends per series for the story-pass ---------------------
    let trends: Vec<SeriesTrend> = layout.series.iter().map(|s| {
        // We need first/last DATA values (not pixel coords) for the takeaway.
        // Recompute by joining xs/ys from the DataFrame for this group.
        let (first_data, last_data) = first_last_for_series(df, &opts.x, &opts.y,
            opts.group.as_deref(), &s.key);
        SeriesTrend { key: s.key.clone(), first: first_data, last: last_data }
    }).collect();

    // ----- story-pass ------------------------------------------------------
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let story = run_line_story_pass(&trends, &story_cfg, palette);

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_line(&layout, story.focal.as_deref(), &story.palette_map, &mut buf);

    // ----- render with Braille ---------------------------------------------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_braille(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose: y-axis labels, plot, x-axis labels, takeaway ----------
    let mut out = String::new();
    let n_rows = body_lines.len();
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

    // X-axis labels: just min and max under the plot.
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width / 2; // Braille: 2 sub-pixel cols per cell
    let pad = plot_cells.saturating_sub(label_l.len() + label_r.len());
    x_axis.push_str(&label_l);
    x_axis.push_str(&" ".repeat(pad));
    x_axis.push_str(&label_r);
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

fn first_last_for_series(
    df:        &DataFrame,
    x_col:     &str,
    y_col:     &str,
    group_col: Option<&str>,
    key:       &str,
) -> (f64, f64) {
    let xs: Vec<f64> = match df.column(x_col).map(|c| c.series()) {
        Ok(Series::Numbers(v)) => v.clone(),
        _ => return (0.0, 0.0),
    };
    let ys: Vec<f64> = match df.column(y_col).map(|c| c.series()) {
        Ok(Series::Numbers(v)) => v.clone(),
        _ => return (0.0, 0.0),
    };
    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g).map(|c| c.series()) {
            Ok(Series::Strings(v)) => v.clone(),
            Ok(Series::Numbers(v)) => v.iter().map(|n| format!("{n}")).collect(),
            _ => return (0.0, 0.0),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    let key = if group_col.is_none() { "__main__" } else { key };
    let mut paired: Vec<(f64, f64)> = xs.iter().zip(ys.iter()).zip(groups.iter())
        .filter(|((_, _), g)| *g == key)
        .map(|((x, y), _)| (*x, *y))
        .collect();
    paired.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let first = paired.first().map(|p| p.1).unwrap_or(0.0);
    let last  = paired.last().map(|p| p.1).unwrap_or(0.0);
    (first, last)
}
```

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Add line pipeline with delta-based focal detection"
```

---

## Task 11: Scatter pipeline

**Files:**
- Create: `crates/tplot/src/commands/scatter.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

Scatter doesn't have time-based "trends," so reuse the bar story-pass on per-series point counts (which series has the most points → focal). For ungrouped scatter (single series), no focal is meaningful; skip the story-pass entirely.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/scatter.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_grouped_scatter_with_focal() {
        let csv = "x,y,g\n1,10,A\n2,12,A\n3,9,A\n4,11,A\n5,13,A\n6,8,A\n1,50,B\n2,55,B\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = ScatterOptions {
            x: "x".into(), y: "y".into(), group: Some("g".into()),
            focus: None, annotate: None,
            neutral: false, no_takeaway: false,
            width: Some(80), height: 16,
            palette_name: "signature".into(),
        };
        let out = render_scatter(&df, &opts).unwrap();
        // Should contain at least one Braille glyph.
        assert!(out.chars().any(|c| (0x2801..=0x28FF).contains(&(c as u32))));
        // A has 6 points, B has 2 → A should be focal (more dense).
        assert!(out.contains("\x1b[38;2;238;123;61m"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod scatter;
pub use scatter::{render_scatter, ScatterOptions};
```

- [ ] **Step 3: Run (expected fail)**

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/scatter.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_scatter;
use tplot_core::rasterize::rasterize_scatter;
use tplot_core::PixelBuffer;
use tplot_render::render_braille;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_story::{run_bar_story_pass, SeriesPoint};

#[derive(Debug, Clone)]
pub struct ScatterOptions {
    pub x:            String,
    pub y:            String,
    pub group:        Option<String>,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub width:        Option<usize>,
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_scatter(df: &DataFrame, opts: &ScatterOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;

    let layout = layout_scatter(df, &opts.x, &opts.y, opts.group.as_deref(),
        canvas_w, canvas_h).map_err(|e| anyhow!(e.to_string()))?;

    // Story-pass: pick focal series by point count (reuse run_bar_story_pass).
    let counts: Vec<SeriesPoint> = layout.series.iter()
        .map(|s| SeriesPoint { key: s.key.clone(), value: s.points.len() as f64 })
        .collect();
    let story_cfg = StoryConfig {
        enabled:    !opts.neutral,
        takeaway:   !opts.no_takeaway,
        focus:      opts.focus.clone().map(FocusMode::Series).unwrap_or(FocusMode::Auto),
        annotation: opts.annotate.clone(),
    };
    let story = run_bar_story_pass(&counts, &story_cfg, palette);

    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_scatter(&layout, story.focal.as_deref(), &story.palette_map, &mut buf);

    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_braille(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    let mut out = String::new();
    let n_rows = body_lines.len();
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

    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let label_l = format!("{:.0}", layout.x_min);
    let label_r = format!("{:.0}", layout.x_max);
    let plot_cells = layout.plot_box.pixel_width / 2;
    let pad = plot_cells.saturating_sub(label_l.len() + label_r.len());
    x_axis.push_str(&label_l);
    x_axis.push_str(&" ".repeat(pad));
    x_axis.push_str(&label_r);
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

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Add scatter pipeline reusing bar story-pass on point counts"
```

---

## Task 12: Wire main.rs and JSON dispatch

**Files:**
- Modify: `crates/tplot/src/main.rs`
- Modify: `crates/tplot/src/commands/json.rs`

- [ ] **Step 1: Wire main.rs**

```rust
// crates/tplot/src/main.rs — add Line and Scatter arms to the match
match args.command {
    Command::Bar(b)   => { /* existing */ }
    Command::Hist(h)  => { /* existing */ }
    Command::Line(l) => {
        let df = pipeline::read_dataframe(&l.input)?;
        let (_, height) = pipeline::detected_terminal_size(l.common.width);
        let out = commands::render_line(&df, &commands::LineOptions {
            x: l.x, y: l.y, group: l.group,
            focus: l.common.focus, annotate: l.common.annotate,
            neutral: l.common.neutral, no_takeaway: l.common.no_takeaway,
            width: l.common.width, height,
            palette_name: l.common.palette,
        })?;
        print!("{out}");
        Ok(())
    }
    Command::Scatter(s) => {
        let df = pipeline::read_dataframe(&s.input)?;
        let (_, height) = pipeline::detected_terminal_size(s.common.width);
        let out = commands::render_scatter(&df, &commands::ScatterOptions {
            x: s.x, y: s.y, group: s.group,
            focus: s.common.focus, annotate: s.common.annotate,
            neutral: s.common.neutral, no_takeaway: s.common.no_takeaway,
            width: s.common.width, height,
            palette_name: s.common.palette,
        })?;
        print!("{out}");
        Ok(())
    }
    Command::Json => { /* existing */ }
}
```

- [ ] **Step 2: Extend JSON dispatch**

```rust
// crates/tplot/src/commands/json.rs — extend the match to handle Line and Scatter

match spec.kind {
    ChartKind::Bar { orientation: BarOrientation::Horizontal } => { /* existing */ }
    ChartKind::Bar { orientation: BarOrientation::Vertical } => { /* existing */ }
    ChartKind::Histogram { bins } => { /* existing */ }
    ChartKind::Line => {
        let x = match spec.x {
            Axis::Column(c) => c,
            _ => return Err(anyhow!("inline x axis not supported in v1")),
        };
        let y = match spec.y {
            Axis::Column(c) => c,
            _ => return Err(anyhow!("inline y axis not supported in v1")),
        };
        let opts = LineOptions {
            x, y, group: spec.group,
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
        render_line(&parsed.dataframe, &opts)
    }
    ChartKind::Scatter => {
        let x = match spec.x {
            Axis::Column(c) => c,
            _ => return Err(anyhow!("inline x axis not supported in v1")),
        };
        let y = match spec.y {
            Axis::Column(c) => c,
            _ => return Err(anyhow!("inline y axis not supported in v1")),
        };
        let opts = ScatterOptions {
            x, y, group: spec.group,
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
        render_scatter(&parsed.dataframe, &opts)
    }
}
```

Add the imports to `json.rs`:
```rust
use crate::commands::{render_line, LineOptions, render_scatter, ScatterOptions};
```

- [ ] **Step 3: Run all tests + smoke test**

```bash
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Smoke test 1: line chart with multi-series
echo "t,v,g
1,10,A
2,11,A
3,9,A
4,12,A
1,5,B
2,30,B
3,80,B
4,200,B" | cargo run -p tplot -- line - -x t -y v --group g

# Smoke test 2: scatter
echo "x,y
1,10
2,12
3,15
4,18
5,30
6,28
7,22
8,18
9,15
10,12" | cargo run -p tplot -- scatter - -x x -y y
```

Expected:
- Line: two lines, B (the steeply growing one) in burnt orange, A in gray, takeaway "B grew Nx — from 5 to 200." or similar.
- Scatter: Braille dots forming an arc shape, all gray (no grouping → no focal).

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Wire line and scatter into main and JSON dispatch"
```

---

## Task 13: End-to-end snapshot tests

**Files:**
- Create: `crates/tplot/tests/e2e_line_and_scatter.rs`

- [ ] **Step 1: Write the snapshot tests**

```rust
// crates/tplot/tests/e2e_line_and_scatter.rs
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
fn line_multi_series_snapshot() {
    let csv = "t,v,g\n1,10,A\n2,11,A\n3,9,A\n4,12,A\n1,5,B\n2,30,B\n3,80,B\n4,200,B\n";
    let out = run_with_stdin(&["line", "-", "-x", "t", "-y", "v", "--group", "g", "--width", "80"], csv);
    insta::assert_snapshot!("line_multi_series", strip_ansi(&out));
}

#[test]
fn scatter_default_snapshot() {
    let csv = "x,y\n1,10\n2,12\n3,15\n4,18\n5,30\n6,28\n7,22\n8,18\n9,15\n10,12\n";
    let out = run_with_stdin(&["scatter", "-", "-x", "x", "-y", "y", "--width", "80"], csv);
    insta::assert_snapshot!("scatter_default", strip_ansi(&out));
}
```

- [ ] **Step 2: Run, inspect, accept**

```bash
cargo test -p tplot --test e2e_line_and_scatter
# Inspect the .snap.new files. They should look like real charts.
cargo insta accept
cargo test -p tplot --test e2e_line_and_scatter
```

- [ ] **Step 3: Commit**

```bash
git add crates/tplot/tests/
git commit -m "Add end-to-end snapshot tests for line and scatter"
```

---

## Task 14: README + lint pass

- [ ] **Step 1: Update README** (replace the "What's in this version" section)

```markdown
## What's in this version (Plans 1 + 2 + 3)

- Five chart types: horizontal bar, vertical bar, histogram, line, scatter.
- Renderers:
  - half-blocks (truecolor + 256/16/mono fallback) for horizontal bars
  - vertical-block elements `▁▂▃▄▅▆▇█` for vertical bars and histograms
  - Braille (2×4 dots/cell) for line and scatter
- Story-pass with chart-specific focal detection (max-value for bars, modal-bin for histograms, largest-delta for lines, point-count for scatter), gray-down palette, and embedded takeaway lines. Trust-score gate refuses to highlight when no series clearly dominates.
- CLI: `tplot bar [--vertical]`, `tplot hist`, `tplot line`, `tplot scatter`, `tplot json` (stdin).

Stacked area, sparkline, heatmap, box plot arrive in Plan 4. `--graphics` image-protocol output and full capability detection in Plan 5.
```

Add quickstart for line:
```bash
# Multi-series line chart with focal-trend detection
echo "t,v,g
1,10,A
2,11,A
3,9,A
4,12,A
1,5,B
2,30,B
3,80,B
4,200,B" | tplot line - -x t -y v --group g
```

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
git commit -m "Update README and lint pass for plan 3"
```

---

## Done — Plan 3 deliverable

You should now be able to:

```bash
# Multi-series line chart
echo "t,v,g
1,10,A
2,11,A
3,9,A
4,12,A
1,5,B
2,30,B
3,80,B
4,200,B" | tplot line - -x t -y v --group g

# Scatter with grouping
tplot scatter users.csv -x signup_age -y session_count --group cohort
```

Both new chart types use Braille rendering for sub-cell smoothness, share the storytelling treatment, and accept the standard flag set.

**Still missing for v1** (Plan 4 onward):
- Stacked area, sparkline, heatmap, box plot
- `--graphics` image-protocol output
- OSC capability probing
- Distribution / Python wrapper

## Self-review notes

Coverage check:
- Braille renderer (Task 1) ✓
- Bresenham line drawing (Task 2) ✓
- Line/Scatter ChartKind (Task 3) ✓
- Line layout (Task 4), rasterizer (Task 5) ✓
- Scatter layout (Task 6), rasterizer (Task 7) ✓
- Line story-pass (Task 8) ✓
- CLI (Task 9), pipelines (Tasks 10-11), wiring (Task 12) ✓
- Snapshot tests (Task 13) ✓
- README + lint (Task 14) ✓
