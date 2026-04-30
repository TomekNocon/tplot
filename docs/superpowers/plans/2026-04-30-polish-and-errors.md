# TerminalPlot — Plan 6: Polish & Error Messages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Polish the user-facing edges of v1: stop swallowing did-you-mean hints behind "no data" errors, give type-error guidance ("column `quarter` is strings; did you mean `-y revenue`?"), auto-detect light vs. dark terminal themes and adapt the context gray, fail gracefully on terminals too narrow to draw the requested chart.

**Architecture:** No new pipelines — this is a quality pass over existing layouts and the rendering layer. Most changes are additive (new error variants, new helper on `Capabilities`); the layout error-masking fix is the one structural change.

**Tech Stack:** Same as prior plans. No new dependencies.

**Inherited context (Plans 1–5.5):**
- 9 chart types working; 175 tests passing.
- All commits use `tomek.lm10@gmail.com`; local git config pins it.
- Eight `layout_*` functions all use `.map_err(|_| Error::Empty)` when looking up DataFrame columns — that's the bug fixed in Task 1.
- Subagents have caught plan-bugs in earlier plans (median computation, missing `Hash` derive, test-data near boundaries). Stay alert for similar issues.

---

## Concrete polish items found during demos

1. **Column-lookup errors hide did-you-mean.** Demo plan-5 typo `-x endpont` → "Error: no data rows" (should be: "unknown column `endpont` — did you mean `endpoint`?"). Affects all layouts.
2. **Type-error errors don't suggest alternatives.** `Error: y column "quarter" must be numeric` doesn't tell you which columns ARE numeric or that you may have meant `-x quarter -y revenue`.
3. **No theme detection.** Default context gray `#767676` is invisible on a light terminal background. Need to probe `$COLORFGBG` / `$TERM_PROGRAM` / fallback heuristics.
4. **Narrow-terminal failure mode.** Some layouts return `TooNarrow` (vertical bar) but most just degrade silently. Spec said: render at minimum legible size down to 40 cells; below 40, fail with width recommendation.
5. **Sparkline CSV-without-`-y` error message** is correct but doesn't list available columns.

---

## File Structure

No new files in protocol/render. Mostly extensions to existing modules.

```
crates/
├── tplot-protocol/src/
│   ├── capabilities.rs             EXTEND: add Theme + theme detection
│   └── color.rs                    EXTEND: helper `for_theme` to pick context gray
├── tplot-core/src/
│   ├── dataframe.rs                EXTEND: improve UnknownColumn message; expose numeric_columns()
│   └── layout/
│       ├── bar.rs, vertical_bar.rs, histogram.rs, line.rs,
│       │ scatter.rs, boxplot.rs, heatmap.rs, stacked_area.rs
│       │                           EXTEND each: stop masking column-lookup errors
├── tplot-story/src/                (no changes)
└── tplot/src/
    ├── pipeline.rs                 EXTEND: error formatting helper
    └── commands/
        └── (each command's *.rs)   EXTEND each: forward DataFrame errors verbatim
```

---

## Task 1: Surface DataFrame errors from all layouts

**Files:**
- Modify: every `crates/tplot-core/src/layout/*.rs` file (8 layouts)

The pattern `df.column(x_col).map_err(|_| Error::Empty)?.series()` swallows the rich DataFrameError (which contains the did-you-mean suggestion) into a generic "no data rows". Replace with proper From-conversion.

- [ ] **Step 1: Write failing tests** — one in each layout that currently masks the error. The test asserts the error message contains the typo'd column name AND a "did you mean" hint.

```rust
// crates/tplot-core/src/layout/bar.rs (within existing tests module, append)
#[test]
fn missing_column_returns_did_you_mean() {
    let df = DataFrame::from_columns(vec![
        Column::new("region",  Series::Strings(vec!["NA".into(),"EMEA".into()])),
        Column::new("revenue", Series::Numbers(vec![10.0, 20.0])),
    ]).unwrap();
    let err = layout_horizontal_bar(&df, "regin", "revenue", None, 80, 12).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("regin"),  "error should name the bad column: {msg}");
    assert!(msg.contains("region"), "error should suggest the closest match: {msg}");
}
```

Repeat the same shape for `vertical_bar.rs`, `histogram.rs`, `line.rs`, `scatter.rs`, `boxplot.rs`, `heatmap.rs`, `stacked_area.rs`.

- [ ] **Step 2: Run all 8 (expected fail)**

```bash
cargo test -p tplot-core --lib layout::
```

- [ ] **Step 3: Add `UnknownColumn` variant to each layout's error enum**

Each layout currently has its own error enum (e.g., `LayoutError`, `VerticalBarLayoutError`, etc.). Each gets a new variant:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    // existing variants...
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
}
```

The `#[from]` derive lets `?` convert `DataFrameError` directly. The `#[error(transparent)]` makes `Display` forward to `DataFrameError`'s message (which already contains the did-you-mean text).

- [ ] **Step 4: Replace the `.map_err(|_| Error::Empty)?` calls**

In each layout, change:
```rust
let labels = match df.column(x_col).map_err(|_| LayoutError::Empty)?.series() { ... }
```
to:
```rust
let labels = match df.column(x_col)?.series() { ... }
```

(With `#[from]` in place, the `?` does the conversion.)

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Surface DataFrame did-you-mean errors from all layouts"
```

---

## Task 2: Type-error guidance — list available numeric columns

**Files:**
- Modify: `crates/tplot-core/src/dataframe.rs`
- Modify: each layout's `NonNumericY` / `NonNumericX` / `NonNumericValue` error site (same 8 layouts)

Add `DataFrame::numeric_columns()` returning column names of numeric columns. The layout error message can include that list when a user passes a non-numeric column.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/dataframe.rs (within existing tests module)
#[test]
fn numeric_columns_lists_only_numeric() {
    let df = sample();   // has "quarter" (strings) and "revenue" (numbers)
    let numeric = df.numeric_columns();
    assert_eq!(numeric, vec!["revenue"]);
}
```

```rust
// crates/tplot-core/src/layout/bar.rs (within tests, append)
#[test]
fn non_numeric_y_lists_alternatives() {
    let df = DataFrame::from_columns(vec![
        Column::new("region",  Series::Strings(vec!["NA".into(),"EMEA".into()])),
        Column::new("revenue", Series::Numbers(vec![10.0, 20.0])),
    ]).unwrap();
    let err = layout_horizontal_bar(&df, "region", "region", None, 80, 12).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("must be numeric"));
    assert!(msg.contains("revenue"), "error should list numeric alternatives: {msg}");
}
```

(Add similar tests in any layout that has a `NonNumeric*` error — bar, vertical_bar, histogram, line, scatter, boxplot, heatmap, stacked_area.)

- [ ] **Step 2: Implement `DataFrame::numeric_columns`**

```rust
// crates/tplot-core/src/dataframe.rs (append on impl DataFrame)
impl DataFrame {
    /// Names of all columns whose `Series` is `Numbers`.
    pub fn numeric_columns(&self) -> Vec<&str> {
        self.columns.iter()
            .filter(|c| matches!(c.series(), Series::Numbers(_)))
            .map(|c| c.name())
            .collect()
    }
}
```

- [ ] **Step 3: Update layout error messages**

Each layout's `NonNumericY` (or equivalent) variant changes from:
```rust
#[error("y column `{0}` must be numeric")]
NonNumericY(String),
```
to:
```rust
#[error("y column `{name}` must be numeric (numeric columns: {numeric})")]
NonNumericY { name: String, numeric: String },
```

And the construction site changes to populate both fields:
```rust
Series::Strings(_) => return Err(LayoutError::NonNumericY {
    name: y_col.to_string(),
    numeric: comma_list(df.numeric_columns()),
}),
```

Add a small helper at the bottom of each layout file:
```rust
fn comma_list(names: Vec<&str>) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
    }
}
```

- [ ] **Step 4: Run (expected pass)**

```bash
cargo test -p tplot-core --lib
```

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add numeric-column hints to type-error messages across all layouts"
```

---

## Task 3: Theme protocol type

**Files:**
- Modify: `crates/tplot-protocol/src/capabilities.rs`
- Modify: `crates/tplot-protocol/src/lib.rs` (re-export)

Add a `Theme { Dark, Light }` enum and a `Capabilities::theme` field. Detect from environment variables: `$COLORFGBG` (foreground:background as numeric ANSI codes), `$TERM_PROGRAM` (iTerm.app, Apple_Terminal, etc.), default to Dark for unknown.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/capabilities.rs (within existing tests module)
#[test]
fn theme_default_is_dark() {
    let c = Capabilities::conservative();
    assert_eq!(c.theme, Theme::Dark);
}

#[test]
fn theme_dark_when_colorfgbg_says_dark_bg() {
    // $COLORFGBG="15;0" means fg=white(15), bg=black(0).
    let c = Capabilities::from_vars(|name| match name {
        "COLORFGBG" => Some("15;0".into()),
        _           => None,
    });
    assert_eq!(c.theme, Theme::Dark);
}

#[test]
fn theme_light_when_colorfgbg_says_light_bg() {
    // $COLORFGBG="0;15" means fg=black, bg=white(15).
    let c = Capabilities::from_vars(|name| match name {
        "COLORFGBG" => Some("0;15".into()),
        _           => None,
    });
    assert_eq!(c.theme, Theme::Light);
}

#[test]
fn theme_apple_terminal_default_inferred_light() {
    // Apple Terminal defaults to a light theme out of the box.
    let c = Capabilities::from_vars(|name| match name {
        "TERM_PROGRAM" => Some("Apple_Terminal".into()),
        _              => None,
    });
    assert_eq!(c.theme, Theme::Light);
}
```

- [ ] **Step 2: Run (expected fail)**

```bash
cargo test -p tplot-protocol --lib capabilities
```

- [ ] **Step 3: Add the variant + detection logic**

```rust
// crates/tplot-protocol/src/capabilities.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub color_depth: ColorDepth,
    pub glyph_set: GlyphSet,
    pub graphics_protocol: GraphicsProtocol,
    pub theme: Theme,
}
```

Update `Capabilities::conservative` to include `theme: Theme::Dark`.

In `Capabilities::from_vars`, append theme detection at the bottom:

```rust
let theme = detect_theme(&get);

Self { color_depth, glyph_set, graphics_protocol, theme }
```

```rust
fn detect_theme<F>(get: &F) -> Theme
where F: Fn(&str) -> Option<String>,
{
    // 1. $COLORFGBG = "fg;bg" or "fg;default;bg". A bg digit ≥ 8 (light gray
    //    or white) is a light terminal; ≤ 7 is dark.
    if let Some(raw) = get("COLORFGBG") {
        let parts: Vec<&str> = raw.split(';').collect();
        if let Some(bg_str) = parts.last() {
            if let Ok(bg) = bg_str.trim().parse::<u8>() {
                return if bg >= 8 || bg == 7 { Theme::Light } else { Theme::Dark };
            }
        }
    }
    // 2. Apple Terminal defaults to a light theme.
    if get("TERM_PROGRAM").as_deref() == Some("Apple_Terminal") {
        return Theme::Light;
    }
    // 3. Default to Dark for everything else (most modern dev terminals).
    Theme::Dark
}
```

- [ ] **Step 4: Re-export from lib.rs**

```rust
// crates/tplot-protocol/src/lib.rs (extend the existing re-export)
pub use capabilities::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib capabilities
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Theme detection (dark/light) to Capabilities"
```

---

## Task 4: Theme-aware context gray in palette

**Files:**
- Modify: `crates/tplot-protocol/src/palette.rs`

Change `Palette::context_color` from a unit method to one that takes a `Theme`. For light backgrounds, the gray needs to be DARKER (so it shows up against white) — `0x4f4f4f`-ish. For dark, keep the existing `0x767676`.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/palette.rs (within existing tests module)
#[test]
fn context_gray_for_dark_is_lighter_than_for_light() {
    let dark  = Palette::Signature.context_color_for(Theme::Dark);
    let light = Palette::Signature.context_color_for(Theme::Light);
    // On dark BG, gray should be lighter (higher RGB values) for legibility;
    // on light BG, gray should be darker.
    assert!(dark.r > light.r);
}

#[test]
fn context_color_default_path_is_dark() {
    // Backwards-compatible call defaults to Theme::Dark.
    let default = Palette::Signature.context_color();
    let dark    = Palette::Signature.context_color_for(Theme::Dark);
    assert_eq!(default, dark);
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-protocol/src/palette.rs
use crate::Theme;

impl Palette {
    pub fn context_color_for(self, theme: Theme) -> RgbColor {
        match theme {
            Theme::Dark  => RgbColor { r: 0x76, g: 0x76, b: 0x76 },  // existing
            Theme::Light => RgbColor { r: 0x4a, g: 0x4a, b: 0x4a },  // darker gray for white BG
        }
    }
    /// Backwards-compatible default — returns the dark-theme gray.
    pub fn context_color(self) -> RgbColor {
        self.context_color_for(Theme::Dark)
    }
}
```

- [ ] **Step 4: Update `dim_context_color` similarly**

```rust
impl Palette {
    pub fn dim_context_color_for(self, theme: Theme) -> RgbColor {
        match theme {
            Theme::Dark  => RgbColor { r: 0x4f, g: 0x4f, b: 0x4f },
            Theme::Light => RgbColor { r: 0x96, g: 0x96, b: 0x96 },
        }
    }
    pub fn dim_context_color(self) -> RgbColor {
        self.dim_context_color_for(Theme::Dark)
    }
}
```

- [ ] **Step 5: Run + commit**

```bash
cargo test -p tplot-protocol --lib palette
git add crates/tplot-protocol/
git commit -m "Make Palette context gray theme-aware (dark vs light backgrounds)"
```

---

## Task 5: Plumb theme into tplot-story palette resolver

**Files:**
- Modify: `crates/tplot-story/src/palette.rs`
- Modify: `crates/tplot-story/src/lib.rs` (story-pass entry points)

`build_palette_map` currently calls `palette.context_color()` (always-dark gray). Add a theme parameter that defaults to Dark via a new `build_palette_map_for(theme)` and have the existing `build_palette_map` forward to it for backwards compatibility. Then update each `run_*_story_pass` to accept and forward the theme.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-story/src/palette.rs (within existing tests module)
#[test]
fn light_theme_uses_darker_context_gray() {
    let dark_map  = build_palette_map_for(&["A","B"], Some("A"), Palette::Signature, Theme::Dark);
    let light_map = build_palette_map_for(&["A","B"], Some("A"), Palette::Signature, Theme::Light);
    let dark_b   = dark_map.get("B").copied().unwrap();
    let light_b  = light_map.get("B").copied().unwrap();
    assert!(light_b.r < dark_b.r, "light-theme gray should be darker");
}
```

- [ ] **Step 2: Implement**

```rust
// crates/tplot-story/src/palette.rs (modify)
use tplot_protocol::Theme;

pub fn build_palette_map(
    series_keys: &[&str],
    focal:       Option<&str>,
    palette:     Palette,
) -> HashMap<String, RgbColor> {
    build_palette_map_for(series_keys, focal, palette, Theme::Dark)
}

pub fn build_palette_map_for(
    series_keys: &[&str],
    focal:       Option<&str>,
    palette:     Palette,
    theme:       Theme,
) -> HashMap<String, RgbColor> {
    let focal_color   = palette.focal_color();
    let context_color = palette.context_color_for(theme);
    series_keys.iter().map(|k| {
        let color = if Some(*k) == focal { focal_color } else { context_color };
        (k.to_string(), color)
    }).collect()
}
```

- [ ] **Step 3: Add `_with_theme` variants to each story-pass entry point**

Each `run_*_story_pass` currently calls `build_palette_map`. Add a parallel `_with_theme` function that accepts a `Theme` and passes through:

```rust
// crates/tplot-story/src/lib.rs (append after existing run_bar_story_pass)
pub fn run_bar_story_pass_with_theme(
    series:  &[SeriesPoint],
    config:  &StoryConfig,
    palette: Palette,
    theme:   Theme,
) -> StoryAnnotated {
    // Same body as run_bar_story_pass but uses build_palette_map_for(..., theme).
    // The existing run_bar_story_pass forwards to this one with Theme::Dark.
}
```

Refactor the existing functions to delegate to the new `_with_theme` ones (the existing public API stays backwards-compatible).

Apply this to: `run_bar_story_pass`, `run_histogram_story_pass`, `run_line_story_pass`, `run_boxplot_story_pass`, `run_stacked_area_story_pass`.

- [ ] **Step 4: Run + commit**

```bash
cargo test -p tplot-story
git add crates/tplot-story/
git commit -m "Plumb Theme through story-pass entry points"
```

---

## Task 6: Use detected theme in the binary's pipelines

**Files:**
- Modify: every `crates/tplot/src/commands/*.rs` (8 commands, except sparkline which doesn't run a story-pass) plus the heatmap pipeline (which doesn't use the story-pass either, but its rasterizer doesn't care about theme).

Each pipeline currently calls `Capabilities::from_vars(...)` once for color depth. Now also pull `caps.theme` and pass it to the `_with_theme` story-pass variant.

- [ ] **Step 1: Update each pipeline**

In `crates/tplot/src/commands/bar.rs` (and analogues for histogram, line, scatter, boxplot, stacked_area), replace:
```rust
let story = run_bar_story_pass(&series_points, &story_cfg, palette);
let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
let body = render_halfblocks(&buf, caps);
```
with:
```rust
let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
let story = run_bar_story_pass_with_theme(&series_points, &story_cfg, palette, caps.theme);
// rest unchanged
let body = render_halfblocks(&buf, caps);
```

- [ ] **Step 2: Add a smoke-style test that exercises the theme-aware path**

```rust
// crates/tplot/src/commands/bar.rs (within tests module, append)
#[test]
fn theme_aware_pipeline_runs_without_panic() {
    use tplot_core::input::parse_csv_str;
    let csv = "region,revenue\nNA,100\nEMEA,200\n";
    let df = parse_csv_str(csv).unwrap();
    let opts = RenderOptions {
        x: "region".into(), y: "revenue".into(),
        group: None, vertical: false,
        focus: None, annotate: None,
        neutral: false, no_takeaway: false,
        width: Some(60), height: 8,
        palette_name: "signature".into(),
    };
    let _ = render_bar(&df, &opts).expect("should render successfully under any theme");
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test --workspace
git add crates/tplot/
git commit -m "Use detected theme in all chart pipelines"
```

---

## Task 7: Narrow-terminal handling

**Files:**
- Modify: `crates/tplot/src/pipeline.rs`

Currently `detected_terminal_size(width_override)` returns `width.max(40)` — meaning if the terminal is 30 cols wide, the engine silently pretends it's 40 and overflows. Spec says: render at the minimum legible size; below 40, fail with a clear message.

- [ ] **Step 1: Add a wrapper that returns a `Result`**

```rust
// crates/tplot/src/pipeline.rs (append, keep the old fn for backwards compat)
use anyhow::{anyhow, Result};

pub fn require_minimum_width(width_override: Option<usize>) -> Result<(usize, usize)> {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let w_actual = width_override.unwrap_or(w as usize);
    if w_actual < 40 {
        return Err(anyhow!(
            "terminal too narrow: needs at least 40 cells, got {w_actual}. \
             Resize the terminal or pass --width 40 to override."
        ));
    }
    Ok((w_actual, (h as usize).max(8)))
}
```

- [ ] **Step 2: Add a unit test**

```rust
// crates/tplot/src/pipeline.rs (within tests if any; otherwise add one)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_below_minimum_width() {
        let err = require_minimum_width(Some(20)).unwrap_err();
        assert!(err.to_string().contains("at least 40"));
    }

    #[test]
    fn accepts_exactly_minimum_width() {
        assert!(require_minimum_width(Some(40)).is_ok());
    }
}
```

- [ ] **Step 3: Update each command's pipeline call site**

Replace `detected_terminal_size(opts.width)` with `require_minimum_width(opts.width)?` in `commands/bar.rs`, `histogram.rs`, `line.rs`, `scatter.rs`, `boxplot.rs`, `heatmap.rs`, `stacked_area.rs`.

(Sparkline doesn't have width-sensitive layout; leave it alone.)

- [ ] **Step 4: Run + commit**

```bash
cargo test --workspace
git add crates/tplot/
git commit -m "Reject terminal widths below 40 with a clear error message"
```

---

## Task 8: Sparkline error polish

**Files:**
- Modify: `crates/tplot/src/commands/sparkline.rs`

When a CSV is detected but `-y` wasn't passed, list the available numeric columns.

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/sparkline.rs (within existing tests module)
#[test]
fn csv_without_y_lists_available_numeric_columns() {
    let csv = "name,score,age\nalice,98,30\nbob,72,25\n";
    let err = parse_input_numbers(csv, None).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("score"));
    assert!(msg.contains("age"));
}
```

- [ ] **Step 2: Update the error**

```rust
// crates/tplot/src/commands/sparkline.rs — modify the CSV branch in parse_input_numbers
if looks_like_csv(s) {
    let df = parse_csv_str(s).map_err(|e| anyhow!(e.to_string()))?;
    let col = y_col.ok_or_else(|| anyhow!(
        "input looks like CSV — pass `-y <column>` to pick the numeric column. \
         Available numeric columns: {}",
        if df.numeric_columns().is_empty() {
            "(none — all columns are strings)".to_string()
        } else {
            df.numeric_columns().iter().map(|n| format!("`{n}`"))
                .collect::<Vec<_>>().join(", ")
        }
    ))?;
    // rest unchanged
}
```

- [ ] **Step 3: Run + commit**

```bash
cargo test -p tplot --lib commands::sparkline
git add crates/tplot/
git commit -m "List available numeric columns when sparkline gets CSV without -y"
```

---

## Task 9: Snapshot tests for new error formats + lint pass

**Files:**
- Create: `crates/tplot/tests/e2e_errors.rs`

A small file that snapshot-tests several error-message paths so future polish doesn't regress them.

- [ ] **Step 1: Write the tests**

```rust
// crates/tplot/tests/e2e_errors.rs
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(2).unwrap().to_path_buf()
}

fn run_expecting_error(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args).current_dir(workspace_root())
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().expect("tplot binary failed to launch");
    String::from_utf8(out.stderr).expect("non-utf8 stderr")
}

#[test]
fn typo_column_lists_did_you_mean() {
    let stderr = run_expecting_error(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "regin", "-y", "revenue", "--group", "region",
    ]);
    insta::assert_snapshot!("error_typo_column", stderr);
}

#[test]
fn non_numeric_y_lists_alternatives() {
    let stderr = run_expecting_error(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "revenue", "-y", "quarter", "--group", "region",
    ]);
    insta::assert_snapshot!("error_non_numeric_y", stderr);
}

#[test]
fn narrow_terminal_rejects_below_40() {
    let stderr = run_expecting_error(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--width", "30",
    ]);
    insta::assert_snapshot!("error_narrow_terminal", stderr);
}
```

- [ ] **Step 2: Run + accept**

```bash
cargo test -p tplot --test e2e_errors
cargo insta accept
cargo test -p tplot --test e2e_errors
```

- [ ] **Step 3: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/tests/
git add -A
git commit -m "Add e2e snapshot tests for error messages, lint pass"
```

---

## Done — Plan 6 deliverable

Before:
```
$ tplot bar sales.csv -x regin -y revenue
Error: no data rows
```

After:
```
$ tplot bar sales.csv -x regin -y revenue
Error: unknown column `regin` — did you mean `region`?
```

```
$ tplot bar sales.csv -x revenue -y quarter
Error: y column `quarter` must be numeric (numeric columns: `revenue`)
```

```
$ tplot bar sales.csv ... --width 20
Error: terminal too narrow: needs at least 40 cells, got 20.
       Resize the terminal or pass --width 40 to override.
```

Plus theme-aware context gray for users on light terminals.

**Still missing for v1:** image protocols (Plan 7), distribution (Plan 8).

## Self-review notes

- DataFrame errors surfacing (Task 1) ✓
- Type-error guidance (Task 2) ✓
- Theme protocol type (Task 3) ✓
- Theme-aware palette gray (Task 4) ✓
- Story-pass plumbing (Task 5) ✓
- Pipeline plumbing (Task 6) ✓
- Narrow-terminal rejection (Task 7) ✓
- Sparkline error polish (Task 8) ✓
- Snapshot tests + lint (Task 9) ✓
