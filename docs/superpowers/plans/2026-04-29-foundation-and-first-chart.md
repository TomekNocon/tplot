# TerminalPlot — Plan 1: Foundation + First Chart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working `tplot bar` binary that reads a CSV/JSON file and renders a horizontal bar chart to the terminal with the *Storytelling with Data* treatment applied (focal series, gray-down, embedded takeaway). Validates the full architecture end to end with the simplest renderer (truecolor half-blocks).

**Architecture:** Cargo workspace with five crates — `tplot-protocol` (shared types), `tplot-core` (data → pixel buffer), `tplot-render` (pixel buffer → ANSI string), `tplot-story` (SWD treatment), and `tplot` (binary). Strict TDD on the math-heavy and heuristic layers; `insta` snapshot tests for the rendering layer; integration snapshot tests for the CLI.

**Tech Stack:** Rust 2024 edition, `clap` (CLI), `serde` + `serde_json` (protocol), `csv` (input), `crossterm` (terminal capability detection), `anyhow` + `thiserror` (errors), `insta` (snapshot tests), `proptest` (property tests).

---

## File Structure

```
TerminalPlot/
├── Cargo.toml                              workspace manifest
├── rust-toolchain.toml                     pin toolchain to 1.84+
├── .gitignore
├── README.md                               quickstart only — full README in plan 6
├── crates/
│   ├── tplot-protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      pub re-exports
│   │       ├── chart.rs                    ChartSpec, ChartKind, BarOrientation
│   │       ├── color.rs                    RgbColor, parse hex
│   │       ├── palette.rs                  Palette enum + named palettes
│   │       ├── capabilities.rs             Capabilities, ColorDepth, GlyphSet
│   │       └── story.rs                    StoryConfig
│   ├── tplot-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dataframe.rs                DataFrame, Series, ColumnType
│   │       ├── input/
│   │       │   ├── mod.rs                  dispatcher (csv | json | stdin sniff)
│   │       │   ├── csv.rs
│   │       │   └── json.rs
│   │       ├── pixel_buffer.rs             PixelBuffer with sub-cell (2×4) addressing
│   │       ├── layout/
│   │       │   ├── mod.rs                  Layout struct + dispatcher
│   │       │   └── bar.rs                  horizontal bar layout algorithm
│   │       └── rasterize/
│   │           ├── mod.rs                  rasterize dispatcher
│   │           └── bar.rs                  horizontal bar rasterizer
│   ├── tplot-render/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ansi.rs                     escape-sequence helpers
│   │       └── halfblocks.rs               half-block renderer
│   ├── tplot-story/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                      run_story_pass entry point
│   │       ├── focal.rs                    focal-series detection + trust score
│   │       ├── palette.rs                  apply signature palette + gray-down
│   │       └── takeaway.rs                 templated takeaway lines
│   └── tplot/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── cli.rs                      clap derive structs
│           ├── pipeline.rs                 orchestrates input→story→layout→render
│           └── commands/
│               ├── mod.rs
│               └── bar.rs                  `tplot bar` subcommand
├── tests/
│   ├── fixtures/
│   │   ├── sales.csv                       Q1–Q4 × 5 regions
│   │   └── sales.json
│   └── e2e_bar.rs                          integration tests (snapshot)
└── docs/superpowers/...                    (already exists)
```

---

## Task 1: Workspace setup

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Modify: `.gitignore`
- Create: `crates/tplot/Cargo.toml`, `crates/tplot/src/main.rs`
- Create: `crates/tplot-protocol/Cargo.toml`, `crates/tplot-protocol/src/lib.rs`
- Create: `crates/tplot-core/Cargo.toml`, `crates/tplot-core/src/lib.rs`
- Create: `crates/tplot-render/Cargo.toml`, `crates/tplot-render/src/lib.rs`
- Create: `crates/tplot-story/Cargo.toml`, `crates/tplot-story/src/lib.rs`

- [ ] **Step 1: Create the workspace `Cargo.toml`**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = [
    "crates/tplot",
    "crates/tplot-core",
    "crates/tplot-protocol",
    "crates/tplot-render",
    "crates/tplot-story",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/TomekNocon/tplot"

[workspace.dependencies]
tplot-protocol = { path = "crates/tplot-protocol", version = "0.1.0" }
tplot-core     = { path = "crates/tplot-core",     version = "0.1.0" }
tplot-render   = { path = "crates/tplot-render",   version = "0.1.0" }
tplot-story    = { path = "crates/tplot-story",    version = "0.1.0" }

clap        = { version = "4",    features = ["derive"] }
serde       = { version = "1",    features = ["derive"] }
serde_json  = "1"
csv         = "1"
crossterm   = "0.28"
anyhow      = "1"
thiserror   = "2"
insta       = { version = "1", features = ["yaml"] }
proptest    = "1"

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

- [ ] **Step 2: Pin the toolchain**

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.84"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 3: Update `.gitignore`**

```
# .gitignore  (replace existing)
.superpowers/
target/
**/*.rs.bk
.DS_Store
```

- [ ] **Step 4: Scaffold each crate**

Run for each crate name `<name>` in the list `tplot tplot-protocol tplot-core tplot-render tplot-story`:

```bash
mkdir -p crates/<name>/src
```

Then write each `Cargo.toml` and `src/lib.rs` (or `src/main.rs` for `tplot`). Examples:

```toml
# crates/tplot-protocol/Cargo.toml
[package]
name = "tplot-protocol"
version.workspace      = true
edition.workspace      = true
license.workspace      = true
repository.workspace   = true

[dependencies]
serde       = { workspace = true }
serde_json  = { workspace = true }
```

```rust
// crates/tplot-protocol/src/lib.rs
//! Shared types for the tplot toolchain.
```

For `tplot` (binary):
```toml
# crates/tplot/Cargo.toml
[package]
name = "tplot"
version.workspace      = true
edition.workspace      = true
license.workspace      = true
repository.workspace   = true

[[bin]]
name = "tplot"
path = "src/main.rs"

[dependencies]
tplot-protocol = { workspace = true }
tplot-core     = { workspace = true }
tplot-render   = { workspace = true }
tplot-story    = { workspace = true }
clap           = { workspace = true }
serde          = { workspace = true }
serde_json     = { workspace = true }
anyhow         = { workspace = true }
```

```rust
// crates/tplot/src/main.rs
fn main() {
    println!("tplot scaffolded");
}
```

Repeat the `Cargo.toml` pattern for each library crate (no binary, depend only on what each will need; protocol crate has no internal deps).

- [ ] **Step 5: Verify the workspace builds**

```bash
cargo build --workspace
```
Expected: compiles cleanly. If a missing-dep error appears, add the dep to that crate's `Cargo.toml`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore crates/
git commit -m "Scaffold cargo workspace with five crates"
```

---

## Task 2: `tplot-protocol` — RgbColor

**Files:**
- Create: `crates/tplot-protocol/src/color.rs`
- Modify: `crates/tplot-protocol/src/lib.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-protocol/src/color.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex() {
        let c = RgbColor::from_hex("#ee7b3d").unwrap();
        assert_eq!(c, RgbColor { r: 0xee, g: 0x7b, b: 0x3d });
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(RgbColor::from_hex("not-a-color").is_err());
        assert!(RgbColor::from_hex("#zzz").is_err());
    }

    #[test]
    fn desaturates_to_grayscale() {
        let orange = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };
        let gray = orange.desaturated();
        // BT.601 luminance check
        assert_eq!(gray.r, gray.g);
        assert_eq!(gray.g, gray.b);
    }
}
```

- [ ] **Step 2: Add the module reference**

```rust
// crates/tplot-protocol/src/lib.rs
pub mod color;
pub use color::RgbColor;
```

- [ ] **Step 3: Run test (expected fail)**

```bash
cargo test -p tplot-protocol --lib color
```
Expected: FAIL — `RgbColor` not found.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-protocol/src/color.rs  (above the tests module)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid hex color: {0}")]
pub struct ParseColorError(pub String);

impl RgbColor {
    pub fn from_hex(s: &str) -> Result<Self, ParseColorError> {
        let s = s.strip_prefix('#').ok_or_else(|| ParseColorError(s.to_string()))?;
        if s.len() != 6 {
            return Err(ParseColorError(s.to_string()));
        }
        let parse = |i: usize| u8::from_str_radix(&s[i..i + 2], 16);
        let r = parse(0).map_err(|_| ParseColorError(s.to_string()))?;
        let g = parse(2).map_err(|_| ParseColorError(s.to_string()))?;
        let b = parse(4).map_err(|_| ParseColorError(s.to_string()))?;
        Ok(RgbColor { r, g, b })
    }

    /// BT.601 luminance, returned as gray RGB.
    pub fn desaturated(self) -> RgbColor {
        let lum = (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        // Slightly darker than pure luminance so context recedes.
        let context = (lum as f32 * 0.55).round() as u8;
        RgbColor { r: context, g: context, b: context }
    }
}
```

Add `thiserror` to `crates/tplot-protocol/Cargo.toml`:
```toml
thiserror = { workspace = true }
```

- [ ] **Step 5: Run tests (expected pass)**

```bash
cargo test -p tplot-protocol --lib color
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add RgbColor type with hex parsing and desaturation"
```

---

## Task 3: `tplot-protocol` — Palette

**Files:**
- Create: `crates/tplot-protocol/src/palette.rs`
- Modify: `crates/tplot-protocol/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/palette.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_palette_focal_is_burnt_orange() {
        let p = Palette::Signature;
        assert_eq!(p.focal_color(), RgbColor { r: 0xee, g: 0x7b, b: 0x3d });
    }

    #[test]
    fn signature_context_color_is_desaturated() {
        let p = Palette::Signature;
        let ctx = p.context_color();
        assert_eq!(ctx.r, ctx.g);
        assert_eq!(ctx.g, ctx.b);
    }

    #[test]
    fn parses_palette_name() {
        assert_eq!(Palette::from_name("signature").unwrap(), Palette::Signature);
        assert_eq!(Palette::from_name("editorial").unwrap(), Palette::Editorial);
        assert_eq!(Palette::from_name("colorblind-safe").unwrap(), Palette::ColorblindSafe);
        assert!(Palette::from_name("nope").is_err());
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-protocol/src/lib.rs (append)
pub mod palette;
pub use palette::Palette;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-protocol --lib palette
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-protocol/src/palette.rs (above tests)
use crate::color::RgbColor;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Palette {
    Signature,
    Editorial,
    ColorblindSafe,
}

impl Palette {
    pub fn focal_color(self) -> RgbColor {
        match self {
            Palette::Signature      => RgbColor::from_hex("#ee7b3d").unwrap(),
            Palette::Editorial      => RgbColor::from_hex("#1f6feb").unwrap(),
            Palette::ColorblindSafe => RgbColor::from_hex("#0072b2").unwrap(),
        }
    }

    pub fn context_color(self) -> RgbColor {
        // Desaturated context color. All palettes share the same gray.
        RgbColor { r: 0x6e, g: 0x76, b: 0x81 }
    }

    pub fn dim_context_color(self) -> RgbColor {
        RgbColor { r: 0x48, g: 0x4f, b: 0x58 }
    }

    pub fn from_name(name: &str) -> Result<Self, UnknownPalette> {
        match name {
            "signature"       => Ok(Palette::Signature),
            "editorial"       => Ok(Palette::Editorial),
            "colorblind-safe" => Ok(Palette::ColorblindSafe),
            other             => Err(UnknownPalette(other.to_string())),
        }
    }
}

impl Default for Palette {
    fn default() -> Self { Palette::Signature }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown palette: {0}")]
pub struct UnknownPalette(pub String);
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib palette
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Palette enum with signature/editorial/colorblind variants"
```

---

## Task 4: `tplot-protocol` — Capabilities

**Files:**
- Create: `crates/tplot-protocol/src/capabilities.rs`
- Modify: `crates/tplot-protocol/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/capabilities.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_is_safe_for_unknown_terminals() {
        let c = Capabilities::conservative();
        assert_eq!(c.color_depth, ColorDepth::Ansi256);
        assert_eq!(c.glyph_set, GlyphSet::HalfBlocks);
        assert_eq!(c.graphics_protocol, GraphicsProtocol::None);
    }

    #[test]
    fn detects_truecolor_from_env() {
        let c = Capabilities::from_vars(|name| match name {
            "COLORTERM" => Some("truecolor".into()),
            "TERM"      => Some("xterm-256color".into()),
            _           => None,
        });
        assert_eq!(c.color_depth, ColorDepth::Truecolor);
    }

    #[test]
    fn detects_kitty_graphics() {
        let c = Capabilities::from_vars(|name| match name {
            "TERM"         => Some("xterm-kitty".into()),
            "COLORTERM"    => Some("truecolor".into()),
            _              => None,
        });
        assert_eq!(c.graphics_protocol, GraphicsProtocol::Kitty);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-protocol/src/lib.rs (append)
pub mod capabilities;
pub use capabilities::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-protocol --lib capabilities
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-protocol/src/capabilities.rs (above tests)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorDepth {
    Mono,
    Ansi16,
    Ansi256,
    Truecolor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlyphSet {
    /// ASCII-only fallback (no Unicode block characters).
    Ascii,
    /// Half-blocks `▀▄█`.
    HalfBlocks,
    /// Half-blocks + Braille (assumed in modern terminals).
    Braille,
    /// Half-blocks + Braille + Unicode 16 octants/sextants.
    Octants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicsProtocol {
    None,
    Kitty,
    ITerm2,
    Sixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub color_depth: ColorDepth,
    pub glyph_set: GlyphSet,
    pub graphics_protocol: GraphicsProtocol,
}

impl Capabilities {
    pub fn conservative() -> Self {
        Self {
            color_depth: ColorDepth::Ansi256,
            glyph_set: GlyphSet::HalfBlocks,
            graphics_protocol: GraphicsProtocol::None,
        }
    }

    /// Detect from environment variables. Caller supplies a getter so tests
    /// can inject env without touching std::env.
    pub fn from_vars<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let term = get("TERM").unwrap_or_default();
        let colorterm = get("COLORTERM").unwrap_or_default();
        let term_program = get("TERM_PROGRAM").unwrap_or_default();

        let color_depth = if matches!(colorterm.as_str(), "truecolor" | "24bit") {
            ColorDepth::Truecolor
        } else if term.contains("256") {
            ColorDepth::Ansi256
        } else if term.is_empty() || term == "dumb" {
            ColorDepth::Mono
        } else {
            ColorDepth::Ansi16
        };

        // Default to Octants (Unicode 16) on truecolor terminals — most modern
        // fonts ship them by 2025. Probing via OSC happens in plan 5.
        let glyph_set = if matches!(color_depth, ColorDepth::Truecolor) {
            GlyphSet::Octants
        } else if matches!(color_depth, ColorDepth::Ansi256) {
            GlyphSet::Braille
        } else {
            GlyphSet::HalfBlocks
        };

        let graphics_protocol = if term == "xterm-kitty" {
            GraphicsProtocol::Kitty
        } else if term_program == "iTerm.app" {
            GraphicsProtocol::ITerm2
        } else {
            GraphicsProtocol::None
        };

        Self { color_depth, glyph_set, graphics_protocol }
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib capabilities
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Capabilities detection from env (color depth, glyphs, graphics)"
```

---

## Task 5: `tplot-protocol` — ChartSpec & StoryConfig

**Files:**
- Create: `crates/tplot-protocol/src/chart.rs`
- Create: `crates/tplot-protocol/src/story.rs`
- Modify: `crates/tplot-protocol/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/chart.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_spec_round_trip_json() {
        let spec = ChartSpec {
            kind: ChartKind::Bar { orientation: BarOrientation::Horizontal },
            x: Axis::Column("quarter".into()),
            y: Axis::Column("revenue".into()),
            group: Some("region".into()),
            title: None,
            story: StoryConfig::default(),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let back: ChartSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back, spec);
    }
}
```

```rust
// crates/tplot-protocol/src/story.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_story_on() {
        let s = StoryConfig::default();
        assert!(s.enabled);
        assert!(s.takeaway);
        assert!(matches!(s.focus, FocusMode::Auto));
    }
}
```

- [ ] **Step 2: Add module refs**

```rust
// crates/tplot-protocol/src/lib.rs (append)
pub mod chart;
pub mod story;
pub use chart::{Axis, BarOrientation, ChartKind, ChartSpec};
pub use story::{FocusMode, StoryConfig};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-protocol --lib
```
Expected: FAIL.

- [ ] **Step 4: Implement story**

```rust
// crates/tplot-protocol/src/story.rs (above tests)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FocusMode {
    Auto,
    Series(String),
    None,
}

impl Default for FocusMode {
    fn default() -> Self { FocusMode::Auto }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryConfig {
    /// Master switch. `--neutral` flips this off.
    pub enabled: bool,
    /// Print the auto-generated "so what" line.
    pub takeaway: bool,
    /// Focal-series resolution mode.
    pub focus: FocusMode,
    /// Optional user-supplied takeaway override.
    pub annotation: Option<String>,
}

impl Default for StoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            takeaway: true,
            focus: FocusMode::Auto,
            annotation: None,
        }
    }
}
```

- [ ] **Step 5: Implement chart**

```rust
// crates/tplot-protocol/src/chart.rs (above tests)
use serde::{Deserialize, Serialize};
use crate::story::StoryConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChartKind {
    Bar { orientation: BarOrientation },
    // Other variants land in subsequent plans.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Axis {
    /// Column lookup (preferred for CSV/file inputs).
    Column(String),
    /// Inline literal data (used for the `--json` form when the caller passed
    /// values inline rather than as columns).
    Inline(Vec<serde_json::Value>),
}

// Note: `Axis::Inline` carries `serde_json::Value`, which does NOT implement
// `Eq` (floats / NaN). Hence `Axis` and the types embedding it derive only
// `PartialEq`. This is intentional — equality of a chart spec is not a
// production concern; it only shows up in tests, which use `assert_eq!`
// (which only needs `PartialEq`).

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartSpec {
    #[serde(flatten)]
    pub kind: ChartKind,
    pub x: Axis,
    pub y: Axis,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub story: StoryConfig,
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib
```
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add ChartSpec, ChartKind, Axis, and StoryConfig protocol types"
```

---

## Task 6: `tplot-core` — DataFrame

**Files:**
- Create: `crates/tplot-core/src/dataframe.rs`
- Modify: `crates/tplot-core/src/lib.rs`
- Modify: `crates/tplot-core/Cargo.toml` (add `tplot-protocol`, `thiserror`)

- [ ] **Step 1: Update Cargo.toml**

```toml
# crates/tplot-core/Cargo.toml — append [dependencies]
tplot-protocol = { workspace = true }
thiserror      = { workspace = true }
serde_json     = { workspace = true }
```

- [ ] **Step 2: Write failing tests**

```rust
// crates/tplot-core/src/dataframe.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("quarter", Series::Strings(
                vec!["Q1","Q2","Q3","Q4"].into_iter().map(String::from).collect())),
            Column::new("revenue", Series::Numbers(vec![42.0, 58.0, 71.0, 52.0])),
        ]).unwrap()
    }

    #[test]
    fn column_lookup_by_name() {
        let df = sample();
        let col = df.column("revenue").unwrap();
        assert_eq!(col.name(), "revenue");
        assert!(matches!(col.series(), Series::Numbers(_)));
    }

    #[test]
    fn column_missing_reports_options() {
        let df = sample();
        let err = df.column("revneue").unwrap_err();
        // Did-you-mean lists at least the closest column.
        assert!(err.to_string().contains("revenue"));
    }

    #[test]
    fn rejects_mismatched_column_lengths() {
        let bad = DataFrame::from_columns(vec![
            Column::new("a", Series::Numbers(vec![1.0, 2.0])),
            Column::new("b", Series::Numbers(vec![3.0])),
        ]);
        assert!(bad.is_err());
    }
}
```

- [ ] **Step 3: Add module ref**

```rust
// crates/tplot-core/src/lib.rs
pub mod dataframe;
pub use dataframe::{Column, DataFrame, DataFrameError, Series};
```

- [ ] **Step 4: Run (expected fail)**

```bash
cargo test -p tplot-core --lib
```
Expected: FAIL.

- [ ] **Step 5: Implement**

```rust
// crates/tplot-core/src/dataframe.rs (above tests)
#[derive(Debug, Clone, PartialEq)]
pub enum Series {
    Numbers(Vec<f64>),
    Strings(Vec<String>),
}

impl Series {
    pub fn len(&self) -> usize {
        match self {
            Series::Numbers(v) => v.len(),
            Series::Strings(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: String,
    series: Series,
}

impl Column {
    pub fn new(name: impl Into<String>, series: Series) -> Self {
        Self { name: name.into(), series }
    }
    pub fn name(&self) -> &str { &self.name }
    pub fn series(&self) -> &Series { &self.series }
    pub fn len(&self) -> usize { self.series.len() }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataFrame {
    columns: Vec<Column>,
}

#[derive(Debug, thiserror::Error)]
pub enum DataFrameError {
    #[error("column lengths differ: {0}")]
    MismatchedLengths(String),
    #[error("unknown column `{name}`{}",
        if let Some(s) = suggestion { format!(" — did you mean `{s}`?") } else { String::new() })]
    UnknownColumn { name: String, suggestion: Option<String> },
}

impl DataFrame {
    pub fn from_columns(columns: Vec<Column>) -> Result<Self, DataFrameError> {
        if let Some(first) = columns.first() {
            let n = first.len();
            for c in &columns[1..] {
                if c.len() != n {
                    return Err(DataFrameError::MismatchedLengths(
                        format!("`{}` has {} rows, `{}` has {}",
                            first.name(), n, c.name(), c.len())));
                }
            }
        }
        Ok(Self { columns })
    }

    pub fn column(&self, name: &str) -> Result<&Column, DataFrameError> {
        if let Some(c) = self.columns.iter().find(|c| c.name() == name) {
            Ok(c)
        } else {
            // Cheap Levenshtein-ish suggestion: pick the first column whose
            // first 3 chars overlap with the requested name.
            let suggestion = self.columns.iter()
                .map(|c| c.name())
                .min_by_key(|n| levenshtein(n, name))
                .map(String::from);
            Err(DataFrameError::UnknownColumn { name: name.to_string(), suggestion })
        }
    }

    pub fn columns(&self) -> &[Column] { &self.columns }
    pub fn nrows(&self) -> usize { self.columns.first().map(|c| c.len()).unwrap_or(0) }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-core --lib
```
Expected: 3 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add DataFrame, Column, Series with did-you-mean lookup"
```

---

## Task 7: `tplot-core` — CSV input

**Files:**
- Create: `crates/tplot-core/src/input/mod.rs`
- Create: `crates/tplot-core/src/input/csv.rs`
- Create: `tests/fixtures/sales.csv`
- Modify: `crates/tplot-core/Cargo.toml` (add `csv`)
- Modify: `crates/tplot-core/src/lib.rs`

- [ ] **Step 1: Add csv dep**

```toml
# crates/tplot-core/Cargo.toml — append
csv = { workspace = true }
```

- [ ] **Step 2: Create the fixture**

```csv
# tests/fixtures/sales.csv
quarter,region,revenue
Q1,NA,42
Q1,EMEA,28
Q1,LATAM,18
Q1,APAC,22
Q1,AU,11
Q2,NA,44
Q2,EMEA,38
Q2,LATAM,19
Q2,APAC,24
Q2,AU,13
Q3,NA,46
Q3,EMEA,55
Q3,LATAM,20
Q3,APAC,25
Q3,AU,14
Q4,NA,47
Q4,EMEA,72
Q4,LATAM,21
Q4,APAC,26
Q4,AU,15
```

- [ ] **Step 3: Write failing tests**

```rust
// crates/tplot-core/src/input/csv.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::Series;

    #[test]
    fn parses_simple_csv() {
        let csv = "a,b\n1,foo\n2,bar\n3,baz\n";
        let df = parse_csv_str(csv).unwrap();
        assert_eq!(df.nrows(), 3);
        assert!(matches!(df.column("a").unwrap().series(), Series::Numbers(_)));
        assert!(matches!(df.column("b").unwrap().series(), Series::Strings(_)));
    }

    #[test]
    fn promotes_numeric_columns_when_all_rows_parse() {
        let csv = "x,y\nQ1,1\nQ2,2\nQ3,3\n";
        let df = parse_csv_str(csv).unwrap();
        assert!(matches!(df.column("x").unwrap().series(), Series::Strings(_)));
        assert!(matches!(df.column("y").unwrap().series(), Series::Numbers(_)));
    }

    #[test]
    fn parses_sales_fixture() {
        let csv = include_str!("../../../../tests/fixtures/sales.csv");
        let df = parse_csv_str(csv).unwrap();
        assert_eq!(df.nrows(), 20);
        let revenue = df.column("revenue").unwrap();
        assert!(matches!(revenue.series(), Series::Numbers(_)));
    }
}
```

- [ ] **Step 4: Add module refs**

```rust
// crates/tplot-core/src/input/mod.rs
pub mod csv;
pub use csv::{parse_csv_reader, parse_csv_str, CsvError};
```

```rust
// crates/tplot-core/src/lib.rs (append)
pub mod input;
```

- [ ] **Step 5: Run (expected fail)**

```bash
cargo test -p tplot-core --lib input
```
Expected: FAIL.

- [ ] **Step 6: Implement**

```rust
// crates/tplot-core/src/input/csv.rs (above tests)
use crate::dataframe::{Column, DataFrame, DataFrameError, Series};
use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("csv parse error: {0}")]
    Parse(#[from] csv::Error),
    #[error(transparent)]
    DataFrame(#[from] DataFrameError),
    #[error("empty input — no header row")]
    Empty,
}

pub fn parse_csv_str(s: &str) -> Result<DataFrame, CsvError> {
    parse_csv_reader(s.as_bytes())
}

pub fn parse_csv_reader<R: Read>(r: R) -> Result<DataFrame, CsvError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(r);

    let headers: Vec<String> = rdr.headers()?.iter().map(String::from).collect();
    if headers.is_empty() {
        return Err(CsvError::Empty);
    }

    let mut raw: Vec<Vec<String>> = vec![Vec::new(); headers.len()];
    for record in rdr.records() {
        let record = record?;
        for (i, field) in record.iter().enumerate() {
            if i < raw.len() {
                raw[i].push(field.to_string());
            }
        }
    }

    // Type inference: if every row in a column parses as f64, it's numeric.
    let columns: Vec<Column> = headers.into_iter().zip(raw).map(|(name, vals)| {
        let parsed: Option<Vec<f64>> = vals.iter()
            .map(|v| v.trim().parse::<f64>().ok())
            .collect();
        let series = match parsed {
            Some(numbers) if !numbers.is_empty() => Series::Numbers(numbers),
            _                                    => Series::Strings(vals),
        };
        Column::new(name, series)
    }).collect();

    Ok(DataFrame::from_columns(columns)?)
}
```

- [ ] **Step 7: Run (expected pass)**

```bash
cargo test -p tplot-core --lib input
```
Expected: 3 passed.

- [ ] **Step 8: Commit**

```bash
git add crates/tplot-core/ tests/fixtures/sales.csv
git commit -m "Add CSV input parser with type inference and sales fixture"
```

---

## Task 8: `tplot-core` — JSON input

**Files:**
- Create: `crates/tplot-core/src/input/json.rs`
- Create: `tests/fixtures/sales.json`
- Modify: `crates/tplot-core/src/input/mod.rs`

- [ ] **Step 1: Create fixture**

```json
{
  "kind": "bar",
  "orientation": "horizontal",
  "x": {"Column": "quarter"},
  "y": {"Column": "revenue"},
  "group": "region",
  "data": {
    "quarter": ["Q1","Q1","Q1","Q1","Q1","Q2","Q2","Q2","Q2","Q2","Q3","Q3","Q3","Q3","Q3","Q4","Q4","Q4","Q4","Q4"],
    "region":  ["NA","EMEA","LATAM","APAC","AU","NA","EMEA","LATAM","APAC","AU","NA","EMEA","LATAM","APAC","AU","NA","EMEA","LATAM","APAC","AU"],
    "revenue": [42,28,18,22,11,44,38,19,24,13,46,55,20,25,14,47,72,21,26,15]
  }
}
```

Save to `tests/fixtures/sales.json`.

- [ ] **Step 2: Write failing test**

```rust
// crates/tplot-core/src/input/json.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::Series;

    #[test]
    fn parses_inline_columns() {
        let j = r#"{
            "data": {
                "x": ["a","b","c"],
                "y": [1.0, 2.0, 3.0]
            }
        }"#;
        let parsed = parse_json_str(j).unwrap();
        let df = parsed.dataframe;
        assert_eq!(df.nrows(), 3);
        assert!(matches!(df.column("y").unwrap().series(), Series::Numbers(_)));
    }

    #[test]
    fn extracts_chart_kind() {
        let j = r#"{
            "kind": "bar",
            "orientation": "horizontal",
            "x": {"Column": "x"},
            "y": {"Column": "y"},
            "data": {"x": ["a"], "y": [1]}
        }"#;
        let parsed = parse_json_str(j).unwrap();
        assert!(parsed.spec.is_some());
    }
}
```

- [ ] **Step 3: Add module ref**

```rust
// crates/tplot-core/src/input/mod.rs (append)
pub mod json;
pub use json::{parse_json_str, ParsedJson, JsonError};
```

- [ ] **Step 4: Run (expected fail)**

```bash
cargo test -p tplot-core --lib input::json
```
Expected: FAIL.

- [ ] **Step 5: Implement**

```rust
// crates/tplot-core/src/input/json.rs (above tests)
use crate::dataframe::{Column, DataFrame, DataFrameError, Series};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use tplot_protocol::ChartSpec;

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("json parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("missing `data` object — expected per-column arrays")]
    MissingData,
    #[error(transparent)]
    DataFrame(#[from] DataFrameError),
}

#[derive(Debug)]
pub struct ParsedJson {
    pub dataframe: DataFrame,
    pub spec: Option<ChartSpec>,
}

#[derive(Debug, Deserialize)]
struct RawJson {
    #[serde(default)]
    data: Option<BTreeMap<String, Vec<Value>>>,
    #[serde(flatten)]
    rest: serde_json::Map<String, Value>,
}

pub fn parse_json_str(s: &str) -> Result<ParsedJson, JsonError> {
    let raw: RawJson = serde_json::from_str(s)?;
    let data = raw.data.ok_or(JsonError::MissingData)?;

    let columns: Vec<Column> = data.into_iter().map(|(name, values)| {
        // If every value is a JSON number, store as Numbers.
        let all_numeric = values.iter().all(|v| v.is_number());
        let series = if all_numeric {
            Series::Numbers(values.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
        } else {
            Series::Strings(values.into_iter().map(|v| match v {
                Value::String(s) => s,
                other            => other.to_string(),
            }).collect())
        };
        Column::new(name, series)
    }).collect();

    let dataframe = DataFrame::from_columns(columns)?;

    // Best-effort spec extraction: if the JSON has a `kind` field, try to
    // interpret it as a ChartSpec (the binary will use this for `--json` mode).
    let spec = if raw.rest.contains_key("kind") {
        serde_json::from_value::<ChartSpec>(Value::Object(raw.rest)).ok()
    } else {
        None
    };

    Ok(ParsedJson { dataframe, spec })
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-core --lib input::json
```
Expected: 2 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-core/ tests/fixtures/sales.json
git commit -m "Add JSON input parser with optional embedded ChartSpec"
```

---

## Task 9: `tplot-core` — PixelBuffer

**Files:**
- Create: `crates/tplot-core/src/pixel_buffer.rs`
- Modify: `crates/tplot-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/pixel_buffer.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::RgbColor;

    const RED: RgbColor   = RgbColor { r: 0xff, g: 0x00, b: 0x00 };
    const BLACK: RgbColor = RgbColor { r: 0x00, g: 0x00, b: 0x00 };

    #[test]
    fn new_buffer_is_transparent() {
        let buf = PixelBuffer::new(10, 8);
        assert_eq!(buf.cell_width(), 10);
        assert_eq!(buf.cell_height(), 4); // 8 sub-pixel rows = 4 cells (2 per cell)
        assert_eq!(buf.get(0, 0), None);
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut buf = PixelBuffer::new(4, 4);
        buf.set(2, 1, RED);
        assert_eq!(buf.get(2, 1), Some(RED));
        assert_eq!(buf.get(0, 0), None);
    }

    #[test]
    fn fill_rect_paints_inclusive_bounds() {
        let mut buf = PixelBuffer::new(8, 4);
        buf.fill_rect(2, 1, 5, 2, RED);
        for y in 1..=2 {
            for x in 2..=5 {
                assert_eq!(buf.get(x, y), Some(RED));
            }
        }
        assert_eq!(buf.get(1, 1), None);
        assert_eq!(buf.get(6, 1), None);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-core/src/lib.rs (append)
pub mod pixel_buffer;
pub use pixel_buffer::PixelBuffer;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib pixel_buffer
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/pixel_buffer.rs (above tests)
use tplot_protocol::RgbColor;

/// Sub-pixel buffer. Width is in sub-pixel columns; height is in sub-pixel rows.
/// Each terminal cell covers 2 sub-pixels horizontally × 2 vertically (for
/// half-blocks). For Octants/Braille the same buffer is consumed at higher
/// resolution by the renderer (handled in later plans).
#[derive(Debug, Clone)]
pub struct PixelBuffer {
    width: usize,
    height: usize,
    pixels: Vec<Option<RgbColor>>,
}

impl PixelBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![None; width * height],
        }
    }

    pub fn pixel_width(&self)  -> usize { self.width }
    pub fn pixel_height(&self) -> usize { self.height }
    pub fn cell_width(&self)   -> usize { self.width }
    /// For half-blocks: 2 vertical sub-pixels per cell.
    pub fn cell_height(&self)  -> usize { (self.height + 1) / 2 }

    pub fn get(&self, x: usize, y: usize) -> Option<RgbColor> {
        if x >= self.width || y >= self.height { return None; }
        self.pixels[y * self.width + x]
    }

    pub fn set(&mut self, x: usize, y: usize, color: RgbColor) {
        if x >= self.width || y >= self.height { return; }
        self.pixels[y * self.width + x] = Some(color);
    }

    pub fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: RgbColor) {
        let x_lo = x0.min(x1);
        let x_hi = x0.max(x1).min(self.width.saturating_sub(1));
        let y_lo = y0.min(y1);
        let y_hi = y0.max(y1).min(self.height.saturating_sub(1));
        for y in y_lo..=y_hi {
            for x in x_lo..=x_hi {
                self.pixels[y * self.width + x] = Some(color);
            }
        }
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib pixel_buffer
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add PixelBuffer with sub-pixel addressing and rect fill"
```

---

## Task 10: `tplot-core` — Horizontal bar layout

**Files:**
- Create: `crates/tplot-core/src/layout/mod.rs`
- Create: `crates/tplot-core/src/layout/bar.rs`
- Modify: `crates/tplot-core/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-core/src/layout/bar.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn small_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("region", Series::Strings(
                vec!["NA","EMEA","LATAM","APAC"].into_iter().map(String::from).collect())),
            Column::new("revenue", Series::Numbers(vec![42.0, 72.0, 21.0, 26.0])),
        ]).unwrap()
    }

    #[test]
    fn produces_one_bar_per_row() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        assert_eq!(layout.bars.len(), 4);
        // Each bar spans both sub-pixel rows of its cell row → full block.
        for bar in &layout.bars {
            assert_eq!(bar.pixel_height, 2);
        }
    }

    #[test]
    fn longest_bar_uses_most_of_plot_width() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        let max_bar = layout.bars.iter().max_by_key(|b| b.pixel_width).unwrap();
        // The 72 (EMEA) bar should be the widest.
        assert_eq!(max_bar.label, "EMEA");
        // Should consume close to (but not exactly) the available plot width.
        assert!(max_bar.pixel_width > layout.plot_box.pixel_width / 2);
    }

    #[test]
    fn shortest_bar_is_proportional() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        let min_bar = layout.bars.iter().min_by_key(|b| b.pixel_width).unwrap();
        let max_bar = layout.bars.iter().max_by_key(|b| b.pixel_width).unwrap();
        // 21/72 ≈ 0.29
        let ratio = min_bar.pixel_width as f64 / max_bar.pixel_width as f64;
        assert!((ratio - (21.0 / 72.0)).abs() < 0.05);
    }

    #[test]
    fn label_margin_accommodates_longest_label() {
        let layout = layout_horizontal_bar(&small_df(), "region", "revenue", None, 80, 12).unwrap();
        // "LATAM" is 5 chars + 2 cells of padding.
        assert_eq!(layout.label_margin, 7);
    }
}
```

- [ ] **Step 2: Add module refs**

```rust
// crates/tplot-core/src/layout/mod.rs
pub mod bar;
pub use bar::{layout_horizontal_bar, BarLayout, BarRect, PlotBox, LayoutError};
```

```rust
// crates/tplot-core/src/lib.rs (append)
pub mod layout;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib layout::bar
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/layout/bar.rs (above tests)
use crate::dataframe::{DataFrame, Series};

/// Region of the buffer (in sub-pixel coords) reserved for the chart itself.
/// In v1 the buffer holds *only* the plot area — labels and value strings are
/// composed around the rendered output by the binary, not painted into the
/// buffer.
#[derive(Debug, Clone, Copy)]
pub struct PlotBox {
    pub pixel_width:  usize,
    pub pixel_height: usize,
}

#[derive(Debug, Clone)]
pub struct BarRect {
    pub label:        String,
    pub value:        f64,
    /// Sub-pixel rect within the plot area (x always starts at 0 in v1).
    pub pixel_x:      usize,
    pub pixel_y:      usize,
    pub pixel_width:  usize,
    pub pixel_height: usize,
    /// Series the bar belongs to (for color routing). For ungrouped charts
    /// each bar is its own "series" keyed by `label`.
    pub series_key:   String,
}

#[derive(Debug, Clone)]
pub struct BarLayout {
    pub plot_box:        PlotBox,
    pub bars:            Vec<BarRect>,
    /// Cell width / height of the surrounding canvas (for the composer).
    pub canvas_cells_w:  usize,
    pub canvas_cells_h:  usize,
    /// Width in cells of the longest left label (used by the composer).
    pub label_margin:    usize,
    /// Width in cells reserved on the right for the value text.
    pub value_margin:    usize,
}

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
}

const SUB_PIXELS_PER_CELL_X: usize = 1; // half-blocks: 1 sub-pixel column per cell
const SUB_PIXELS_PER_CELL_Y: usize = 2; // half-blocks: 2 sub-pixel rows per cell

pub fn layout_horizontal_bar(
    df:             &DataFrame,
    x_col:          &str,
    y_col:          &str,
    _group_col:     Option<&str>,   // grouping handled in plan 2 — this plan keeps it single-series
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<BarLayout, LayoutError> {
    let labels: Vec<String> = match df.column(x_col).map_err(|_| LayoutError::Empty)?.series() {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df.column(y_col).map_err(|_| LayoutError::Empty)?.series() {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(LayoutError::NonNumericY(y_col.to_string())),
    };
    if labels.is_empty() { return Err(LayoutError::Empty); }

    // Aggregate duplicate labels by sum.
    let mut agg: Vec<(String, f64)> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(slot) = agg.iter_mut().find(|(name, _)| name == l) {
            slot.1 += v;
        } else {
            agg.push((l.clone(), *v));
        }
    }

    // Margins live OUTSIDE the buffer. The composer prepends the label and
    // appends the value to each rendered cell row.
    let label_margin = agg.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0) + 2;
    let value_margin = 8;
    let plot_cells_w  = canvas_cells_w.saturating_sub(label_margin + value_margin).max(8);
    let plot_pixels_w = plot_cells_w * SUB_PIXELS_PER_CELL_X;

    // One cell per bar (with a 1-row gap between bars when there's space).
    // For half-blocks each row covers 2 sub-pixels; the bar fills both halves.
    let bar_count = agg.len();
    let row_per_bar: usize = if bar_count <= canvas_cells_h.saturating_sub(2) { 2 } else { 1 };
    let plot_cells_h  = bar_count * row_per_bar;
    let plot_pixels_h = plot_cells_h * SUB_PIXELS_PER_CELL_Y;

    let plot_box = PlotBox {
        pixel_width:  plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    let max_value = agg.iter().map(|(_, v)| *v).fold(f64::MIN, f64::max).max(1e-9);

    let mut bars = Vec::with_capacity(bar_count);
    for (i, (label, value)) in agg.into_iter().enumerate() {
        let pixel_width  = ((value / max_value) * plot_pixels_w as f64).round() as usize;
        // Each bar fills one cell row vertically — both upper and lower
        // sub-pixels — so half-block rendering produces a FULL block.
        let pixel_y      = i * row_per_bar * SUB_PIXELS_PER_CELL_Y;
        let pixel_height = SUB_PIXELS_PER_CELL_Y;
        bars.push(BarRect {
            label: label.clone(),
            value,
            pixel_x:      0,
            pixel_y,
            pixel_width,
            pixel_height,
            series_key:   label,
        });
    }

    Ok(BarLayout {
        plot_box,
        bars,
        canvas_cells_w,
        canvas_cells_h,
        label_margin,
        value_margin,
    })
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-core --lib layout::bar
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add horizontal bar layout with proportional widths and label margin"
```

---

## Task 11: `tplot-core` — Rasterize horizontal bar

**Files:**
- Create: `crates/tplot-core/src/rasterize/mod.rs`
- Create: `crates/tplot-core/src/rasterize/bar.rs`
- Modify: `crates/tplot-core/src/lib.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot-core/src/rasterize/bar.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{BarLayout, BarRect, PlotBox};
    use std::collections::HashMap;
    use tplot_protocol::RgbColor;

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    fn fake_layout() -> BarLayout {
        BarLayout {
            plot_box: PlotBox { pixel_width: 30, pixel_height: 8 },
            bars: vec![
                BarRect {
                    label: "EMEA".into(),
                    value: 72.0,
                    pixel_x: 0, pixel_y: 0,
                    pixel_width: 30, pixel_height: 2,
                    series_key: "EMEA".into(),
                },
            ],
            canvas_cells_w: 60, canvas_cells_h: 12,
            label_margin: 8, value_margin: 8,
        }
    }

    #[test]
    fn paints_focal_bar_in_focal_color() {
        let mut buf = crate::PixelBuffer::new(30, 8);
        let mut palette: HashMap<String, RgbColor> = HashMap::new();
        palette.insert("EMEA".into(), ORANGE);
        rasterize_bar(&fake_layout(), &palette, &mut buf);
        assert_eq!(buf.get(0,  0), Some(ORANGE));
        assert_eq!(buf.get(29, 0), Some(ORANGE));
        assert_eq!(buf.get(0,  1), Some(ORANGE));
        // Outside the bar should still be empty.
        assert_eq!(buf.get(0,  2), None);
    }
}
```

- [ ] **Step 2: Add module refs**

```rust
// crates/tplot-core/src/rasterize/mod.rs
pub mod bar;
pub use bar::rasterize_bar;
```

```rust
// crates/tplot-core/src/lib.rs (append)
pub mod rasterize;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-core --lib rasterize::bar
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-core/src/rasterize/bar.rs (above tests)
use crate::layout::BarLayout;
use crate::PixelBuffer;
use std::collections::HashMap;
use tplot_protocol::RgbColor;

pub fn rasterize_bar(
    layout:  &BarLayout,
    palette: &HashMap<String, RgbColor>,
    buf:     &mut PixelBuffer,
) {
    for bar in &layout.bars {
        let color = palette.get(&bar.series_key)
            .copied()
            .unwrap_or(RgbColor { r: 0x6e, g: 0x76, b: 0x81 });
        if bar.pixel_width == 0 { continue; }
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
cargo test -p tplot-core --lib rasterize::bar
```
Expected: 1 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-core/
git commit -m "Add bar rasterizer that paints layouts into the pixel buffer"
```

---

## Task 12: `tplot-render` — ANSI escape helpers

**Files:**
- Create: `crates/tplot-render/src/ansi.rs`
- Modify: `crates/tplot-render/src/lib.rs`
- Modify: `crates/tplot-render/Cargo.toml`

- [ ] **Step 1: Update Cargo.toml**

```toml
# crates/tplot-render/Cargo.toml — append [dependencies]
tplot-protocol = { workspace = true }
tplot-core     = { workspace = true }
```

- [ ] **Step 2: Write failing tests**

```rust
// crates/tplot-render/src/ansi.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{ColorDepth, RgbColor};

    const ORANGE: RgbColor = RgbColor { r: 0xee, g: 0x7b, b: 0x3d };

    #[test]
    fn truecolor_emits_38_2_form() {
        assert_eq!(fg(ORANGE, ColorDepth::Truecolor), "\x1b[38;2;238;123;61m");
    }

    #[test]
    fn ansi256_uses_color_cube_index() {
        let s = fg(ORANGE, ColorDepth::Ansi256);
        // Should match the 6×6×6 color cube (16 + 36*r6 + 6*g6 + b6).
        // For (238,123,61) → r6=4, g6=2, b6=1 → 16 + 144 + 12 + 1 = 173.
        assert_eq!(s, "\x1b[38;5;173m");
    }

    #[test]
    fn reset_is_constant() {
        assert_eq!(reset(), "\x1b[0m");
    }
}
```

- [ ] **Step 3: Add module ref**

```rust
// crates/tplot-render/src/lib.rs
pub mod ansi;
pub use ansi::{bg, fg, reset};
```

- [ ] **Step 4: Run (expected fail)**

```bash
cargo test -p tplot-render --lib ansi
```
Expected: FAIL.

- [ ] **Step 5: Implement**

```rust
// crates/tplot-render/src/ansi.rs (above tests)
use tplot_protocol::{ColorDepth, RgbColor};

pub fn reset() -> &'static str { "\x1b[0m" }

pub fn fg(c: RgbColor, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b),
        ColorDepth::Ansi256   => format!("\x1b[38;5;{}m", to_256(c)),
        ColorDepth::Ansi16    => format!("\x1b[{}m", to_16_fg(c)),
        ColorDepth::Mono      => String::new(),
    }
}

pub fn bg(c: RgbColor, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("\x1b[48;2;{};{};{}m", c.r, c.g, c.b),
        ColorDepth::Ansi256   => format!("\x1b[48;5;{}m", to_256(c)),
        ColorDepth::Ansi16    => format!("\x1b[{}m", to_16_fg(c) + 10),
        ColorDepth::Mono      => String::new(),
    }
}

fn to_256(c: RgbColor) -> u8 {
    // 6×6×6 color cube starts at 16. Each channel mapped to 0..=5.
    let q = |v: u8| -> u8 {
        if v < 48 { 0 }
        else if v < 115 { 1 }
        else { ((v as u16 - 35) / 40).min(5) as u8 }
    };
    16 + 36 * q(c.r) + 6 * q(c.g) + q(c.b)
}

fn to_16_fg(c: RgbColor) -> u32 {
    // Crude perceptual nearest of standard ANSI 0..7 (+8 for bright).
    let (r, g, b) = (c.r as i32, c.g as i32, c.b as i32);
    let bright_threshold = 0xb0;
    let bright = r > bright_threshold || g > bright_threshold || b > bright_threshold;
    let base = match (r > 0x60, g > 0x60, b > 0x60) {
        (false, false, false) => 30,
        (true,  false, false) => 31,
        (false, true,  false) => 32,
        (true,  true,  false) => 33,
        (false, false, true)  => 34,
        (true,  false, true)  => 35,
        (false, true,  true)  => 36,
        (true,  true,  true)  => 37,
    };
    if bright { base + 60 } else { base }
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-render --lib ansi
```
Expected: 3 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add ANSI escape helpers for truecolor / 256 / 16 / mono"
```

---

## Task 13: `tplot-render` — Half-blocks renderer

**Files:**
- Create: `crates/tplot-render/src/halfblocks.rs`
- Modify: `crates/tplot-render/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/halfblocks.rs
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

    #[test]
    fn empty_buffer_renders_only_newlines_and_resets() {
        let buf = PixelBuffer::new(4, 2);
        let s = render_halfblocks(&buf, caps());
        assert!(s.lines().count() >= 1);
        // No color escape sequences for empty cells.
        assert!(!s.contains("\x1b[38;2"));
    }

    #[test]
    fn upper_pixel_only_emits_upper_half_block() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set(0, 0, ORANGE);
        let s = render_halfblocks(&buf, caps());
        // ▀ (upper half block) with orange foreground.
        assert!(s.contains('\u{2580}'));
        assert!(s.contains("\x1b[38;2;238;123;61m"));
    }

    #[test]
    fn both_pixels_set_emit_full_block_when_same_color() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.set(0, 0, ORANGE);
        buf.set(0, 1, ORANGE);
        let s = render_halfblocks(&buf, caps());
        assert!(s.contains('\u{2588}'));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-render/src/lib.rs (append)
pub mod halfblocks;
pub use halfblocks::render_halfblocks;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-render --lib halfblocks
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-render/src/halfblocks.rs (above tests)
use crate::ansi::{bg, fg, reset};
use std::fmt::Write as _;
use tplot_core::PixelBuffer;
use tplot_protocol::{Capabilities, RgbColor};

const UPPER: char = '\u{2580}'; // ▀
const LOWER: char = '\u{2584}'; // ▄
const FULL:  char = '\u{2588}'; // █

pub fn render_halfblocks(buf: &PixelBuffer, caps: Capabilities) -> String {
    let cells_w = buf.cell_width();
    let cells_h = buf.cell_height();
    let mut out = String::with_capacity(cells_w * cells_h * 12);

    for cy in 0..cells_h {
        let py_top = cy * 2;
        let py_bot = py_top + 1;

        for cx in 0..cells_w {
            let top = buf.get(cx, py_top);
            let bot = buf.get(cx, py_bot);

            match (top, bot) {
                (None, None) => out.push(' '),
                (Some(t), Some(b)) if t == b => {
                    write!(out, "{}{}{}", fg(t, caps.color_depth), FULL, reset()).unwrap();
                }
                (Some(t), Some(b)) => {
                    write!(out, "{}{}{}{}",
                        fg(t, caps.color_depth),
                        bg(b, caps.color_depth),
                        UPPER,
                        reset()).unwrap();
                }
                (Some(t), None) => {
                    write!(out, "{}{}{}", fg(t, caps.color_depth), UPPER, reset()).unwrap();
                }
                (None, Some(b)) => {
                    write!(out, "{}{}{}", fg(b, caps.color_depth), LOWER, reset()).unwrap();
                }
            }
        }
        out.push('\n');
    }
    out
}

#[allow(dead_code)]
fn _typecheck(_: RgbColor) {} // keep RgbColor import live for future variants
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-render --lib halfblocks
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add half-blocks renderer with truecolor and per-cell merging"
```

---

## Task 14: `tplot-story` — Focal series detection

**Files:**
- Create: `crates/tplot-story/src/focal.rs`
- Modify: `crates/tplot-story/src/lib.rs`
- Modify: `crates/tplot-story/Cargo.toml`

- [ ] **Step 1: Update Cargo.toml**

```toml
# crates/tplot-story/Cargo.toml — append [dependencies]
tplot-protocol = { workspace = true }
tplot-core     = { workspace = true }
```

- [ ] **Step 2: Write failing tests**

```rust
// crates/tplot-story/src/focal.rs
#[cfg(test)]
mod tests {
    use super::*;

    fn rows(values: &[(&str, f64)]) -> Vec<SeriesPoint> {
        values.iter().map(|(k, v)| SeriesPoint { key: k.to_string(), value: *v }).collect()
    }

    #[test]
    fn picks_clear_max_when_dominant() {
        let r = pick_focal(&rows(&[("a", 10.0), ("b", 11.0), ("c", 9.5), ("d", 72.0), ("e", 12.0)]));
        assert_eq!(r.choice, FocalChoice::Series("d".into()));
        assert!(r.trust_score > 1.5);
    }

    #[test]
    fn admits_no_focal_when_uniform() {
        let r = pick_focal(&rows(&[("a", 50.0), ("b", 51.0), ("c", 49.0), ("d", 50.5)]));
        assert_eq!(r.choice, FocalChoice::None);
        assert!(r.trust_score < 1.5);
    }

    #[test]
    fn empty_input_returns_none() {
        let r = pick_focal(&[]);
        assert_eq!(r.choice, FocalChoice::None);
    }
}
```

- [ ] **Step 3: Add module ref**

```rust
// crates/tplot-story/src/lib.rs
pub mod focal;
pub use focal::{pick_focal, FocalChoice, FocalResult, SeriesPoint};
```

- [ ] **Step 4: Run (expected fail)**

```bash
cargo test -p tplot-story --lib focal
```
Expected: FAIL.

- [ ] **Step 5: Implement**

```rust
// crates/tplot-story/src/focal.rs (above tests)
#[derive(Debug, Clone)]
pub struct SeriesPoint {
    pub key: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocalChoice {
    Series(String),
    None,
}

#[derive(Debug, Clone)]
pub struct FocalResult {
    pub choice: FocalChoice,
    pub trust_score: f64,
    /// "max" | "delta" | "outlier" — for the takeaway template.
    pub reason: &'static str,
}

const TRUST_THRESHOLD: f64 = 1.5;

/// Single-pass focal detection on an aggregated series. For v1 (single-series
/// horizontal bar) we score by max-vs-median dominance only; outlier and delta
/// signals come in plan 3 alongside line charts.
pub fn pick_focal(points: &[SeriesPoint]) -> FocalResult {
    if points.is_empty() {
        return FocalResult { choice: FocalChoice::None, trust_score: 0.0, reason: "empty" };
    }

    let mut sorted: Vec<f64> = points.iter().map(|p| p.value).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[sorted.len() / 2];

    let max_point = points.iter().max_by(|a, b| a.value.partial_cmp(&b.value)
        .unwrap_or(std::cmp::Ordering::Equal)).unwrap();

    let trust = if median.abs() < 1e-9 {
        // Median is zero — any nonzero leader dominates trivially.
        if max_point.value.abs() > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        max_point.value / median
    };

    if trust >= TRUST_THRESHOLD {
        FocalResult {
            choice: FocalChoice::Series(max_point.key.clone()),
            trust_score: trust,
            reason: "max",
        }
    } else {
        FocalResult {
            choice: FocalChoice::None,
            trust_score: trust,
            reason: "no-dominant-series",
        }
    }
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot-story --lib focal
```
Expected: 3 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add focal-series detection with trust-score gate"
```

---

## Task 15: `tplot-story` — Palette resolution

**Files:**
- Create: `crates/tplot-story/src/palette.rs`
- Modify: `crates/tplot-story/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-story/src/palette.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{Palette, RgbColor};

    #[test]
    fn focal_series_gets_focal_color() {
        let map = build_palette_map(
            &["NA","EMEA","LATAM","APAC","AU"],
            Some("EMEA"),
            Palette::Signature,
        );
        assert_eq!(map.get("EMEA"), Some(&RgbColor::from_hex("#ee7b3d").unwrap()));
    }

    #[test]
    fn other_series_share_context_color() {
        let map = build_palette_map(
            &["NA","EMEA","LATAM","APAC","AU"],
            Some("EMEA"),
            Palette::Signature,
        );
        let na    = map.get("NA").copied().unwrap();
        let latam = map.get("LATAM").copied().unwrap();
        assert_eq!(na, latam);
        assert_ne!(na, RgbColor::from_hex("#ee7b3d").unwrap());
    }

    #[test]
    fn no_focal_paints_everyone_in_context() {
        let map = build_palette_map(
            &["NA","EMEA","LATAM"],
            None,
            Palette::Signature,
        );
        let unique: std::collections::HashSet<_> = map.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-story/src/lib.rs (append)
pub mod palette;
pub use palette::build_palette_map;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-story --lib palette
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-story/src/palette.rs (above tests)
use std::collections::HashMap;
use tplot_protocol::{Palette, RgbColor};

pub fn build_palette_map(
    series_keys: &[&str],
    focal:       Option<&str>,
    palette:     Palette,
) -> HashMap<String, RgbColor> {
    let focal_color   = palette.focal_color();
    let context_color = palette.context_color();
    series_keys.iter().map(|k| {
        let color = if Some(*k) == focal { focal_color } else { context_color };
        (k.to_string(), color)
    }).collect()
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-story --lib palette
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add palette resolver: focal series → focal color, rest → context"
```

---

## Task 16: `tplot-story` — Takeaway generation

**Files:**
- Create: `crates/tplot-story/src/takeaway.rs`
- Modify: `crates/tplot-story/src/lib.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-story/src/takeaway.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_takeaway_names_focal_and_quantifies() {
        let t = bar_takeaway(Some("EMEA"), 72.0, 25.5, 5);
        assert!(t.contains("EMEA"));
        assert!(t.contains("standout") || t.contains("led") || t.contains("highest"));
    }

    #[test]
    fn no_focal_returns_neutral_message() {
        let t = bar_takeaway(None, 0.0, 0.0, 0);
        assert!(t.to_lowercase().contains("no series"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-story/src/lib.rs (append)
pub mod takeaway;
pub use takeaway::bar_takeaway;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-story --lib takeaway
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot-story/src/takeaway.rs (above tests)
/// Produce a one-line takeaway for a horizontal bar chart.
///
/// `focal_value` is the focal series' value; `median` is the median across
/// all series; `n_series` is the count for the "out of N" phrasing.
pub fn bar_takeaway(
    focal:        Option<&str>,
    focal_value:  f64,
    median:       f64,
    n_series:     usize,
) -> String {
    let Some(name) = focal else {
        return "No series stands out clearly — values are within ±50% of the median.".into();
    };
    if median.abs() < 1e-9 {
        return format!("{name} led with {focal_value:.0} — the standout in an otherwise flat field.");
    }
    let multiple = focal_value / median;
    if multiple >= 2.5 {
        format!("{name} stood out at {focal_value:.0} — over {multiple:.1}× the median across {n_series} series.")
    } else {
        format!("{name} led with {focal_value:.0}, the highest of {n_series} series.")
    }
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot-story --lib takeaway
```
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-story/
git commit -m "Add takeaway templates for bar charts"
```

---

## Task 17: `tplot-story` — Story-pass entry point

**Files:**
- Modify: `crates/tplot-story/src/lib.rs`

- [ ] **Step 1: Write failing tests in lib.rs**

```rust
// crates/tplot-story/src/lib.rs (replace contents below the existing pub mod lines)
pub mod focal;
pub mod palette;
pub mod takeaway;

pub use focal::{pick_focal, FocalChoice, FocalResult, SeriesPoint};
pub use palette::build_palette_map;
pub use takeaway::bar_takeaway;

use std::collections::HashMap;
use tplot_protocol::{FocusMode, Palette, RgbColor, StoryConfig};

#[derive(Debug, Clone)]
pub struct StoryAnnotated {
    pub focal: Option<String>,
    pub palette_map: HashMap<String, RgbColor>,
    pub takeaway: Option<String>,
}

/// Run the story-pass on a single-series bar chart with the given series
/// values. Returns a `StoryAnnotated` ready for the layout/render stages.
pub fn run_bar_story_pass(
    series:   &[SeriesPoint],
    config:   &StoryConfig,
    palette:  Palette,
) -> StoryAnnotated {
    if !config.enabled {
        // Neutral mode: every series gets the focal color (deliberately —
        // user opted out of the gray-down treatment).
        let map = series.iter()
            .map(|p| (p.key.clone(), palette.focal_color()))
            .collect();
        return StoryAnnotated {
            focal: None,
            palette_map: map,
            takeaway: config.annotation.clone(),
        };
    }

    let focal_choice = match &config.focus {
        FocusMode::Auto             => pick_focal(series),
        FocusMode::Series(name)     => FocalResult {
            choice: FocalChoice::Series(name.clone()),
            trust_score: f64::INFINITY,
            reason: "user-specified",
        },
        FocusMode::None             => FocalResult {
            choice: FocalChoice::None,
            trust_score: 0.0,
            reason: "user-disabled",
        },
    };

    let focal_name = match &focal_choice.choice {
        FocalChoice::Series(s) => Some(s.as_str()),
        FocalChoice::None      => None,
    };

    let keys: Vec<&str> = series.iter().map(|p| p.key.as_str()).collect();
    let palette_map = build_palette_map(&keys, focal_name, palette);

    let takeaway = if !config.takeaway {
        None
    } else if let Some(custom) = &config.annotation {
        Some(custom.clone())
    } else {
        let median = {
            let mut v: Vec<f64> = series.iter().map(|p| p.value).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            v.get(v.len() / 2).copied().unwrap_or(0.0)
        };
        let focal_value = focal_name.and_then(|n| series.iter()
            .find(|p| p.key == n).map(|p| p.value)).unwrap_or(0.0);
        Some(bar_takeaway(focal_name, focal_value, median, series.len()))
    };

    StoryAnnotated {
        focal: focal_name.map(String::from),
        palette_map,
        takeaway,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(&str, f64)]) -> Vec<SeriesPoint> {
        v.iter().map(|(k, v)| SeriesPoint { key: k.to_string(), value: *v }).collect()
    }

    #[test]
    fn end_to_end_default_config() {
        let series = pts(&[("NA", 179.0), ("EMEA", 193.0), ("LATAM", 78.0), ("APAC", 97.0), ("AU", 53.0)]);
        let s = run_bar_story_pass(&series, &StoryConfig::default(), Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("EMEA"));
        assert!(s.takeaway.unwrap().contains("EMEA"));
    }

    #[test]
    fn neutral_mode_skips_story_pass() {
        let series = pts(&[("a", 1.0), ("b", 2.0)]);
        let mut cfg = StoryConfig::default();
        cfg.enabled = false;
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert!(s.focal.is_none());
        let unique: std::collections::HashSet<_> = s.palette_map.values().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn user_focus_overrides_auto() {
        let series = pts(&[("a", 100.0), ("b", 1.0), ("c", 1.0)]);
        let mut cfg = StoryConfig::default();
        cfg.focus = FocusMode::Series("b".into());
        let s = run_bar_story_pass(&series, &cfg, Palette::Signature);
        assert_eq!(s.focal.as_deref(), Some("b"));
    }
}
```

- [ ] **Step 2: Run tests (expected pass)**

```bash
cargo test -p tplot-story --lib
```
Expected: all green.

- [ ] **Step 3: Commit**

```bash
git add crates/tplot-story/
git commit -m "Wire focal/palette/takeaway into run_bar_story_pass entry point"
```

---

## Task 18: `tplot` — CLI argument parsing

**Files:**
- Create: `crates/tplot/src/cli.rs`
- Modify: `crates/tplot/src/main.rs`

- [ ] **Step 1: Write failing test (CLI parses bar form)**

```rust
// crates/tplot/src/cli.rs
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_bar_with_xy() {
        let args = Cli::parse_from([
            "tplot", "bar", "sales.csv",
            "-x", "quarter",
            "-y", "revenue",
            "--group", "region",
        ]);
        match args.command {
            Command::Bar(b) => {
                assert_eq!(b.input, "sales.csv");
                assert_eq!(b.x, "quarter");
                assert_eq!(b.y, "revenue");
                assert_eq!(b.group.as_deref(), Some("region"));
            }
            _ => panic!("expected Bar"),
        }
    }

    #[test]
    fn neutral_flag_propagates() {
        let args = Cli::parse_from([
            "tplot", "bar", "sales.csv", "-x", "q", "-y", "r", "--neutral",
        ]);
        match args.command {
            Command::Bar(b) => assert!(b.common.neutral),
            _ => panic!(),
        }
    }
}
```

- [ ] **Step 2: Add module + use in main**

```rust
// crates/tplot/src/main.rs
mod cli;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::Bar(_) => {
            // Wired up in Task 19.
            eprintln!("bar command parsed; pipeline lands in next task");
            Ok(())
        }
        Command::Json => {
            eprintln!("json mode parsed; pipeline lands in next task");
            Ok(())
        }
    }
}
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot --lib cli
```
Expected: FAIL — `Cli` not found.

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/cli.rs (above tests)
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name    = "tplot",
    version,
    about   = "Storytelling-first chart engine for the terminal",
    arg_required_else_help = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Render a bar chart from a CSV/JSON file.
    Bar(BarArgs),
    /// Read a JSON ChartSpec from stdin and render it.
    Json,
}

#[derive(Args, Debug)]
pub struct BarArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Column for x-axis labels.
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column for y-axis values.
    #[arg(short = 'y')]
    pub y: String,
    /// Optional grouping column.
    #[arg(long)]
    pub group: Option<String>,
    /// Vertical bars instead of horizontal.
    #[arg(long)]
    pub vertical: bool,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug, Clone)]
pub struct CommonStoryArgs {
    /// Override auto-detected focal series.
    #[arg(long)]
    pub focus: Option<String>,
    /// Replace the auto-generated takeaway with this text.
    #[arg(long)]
    pub annotate: Option<String>,
    /// Skip the storytelling pass entirely.
    #[arg(long)]
    pub neutral: bool,
    /// Suppress the takeaway line (keep story styling).
    #[arg(long = "no-takeaway")]
    pub no_takeaway: bool,
    /// Force terminal width (default: detected).
    #[arg(long)]
    pub width: Option<usize>,
    /// Color palette: signature | editorial | colorblind-safe.
    #[arg(long, default_value = "signature")]
    pub palette: String,
}
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot --lib cli
```
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Add clap-derive CLI with bar subcommand and common story flags"
```

---

## Task 19: `tplot` — Bar pipeline integration

**Files:**
- Create: `crates/tplot/src/pipeline.rs`
- Create: `crates/tplot/src/commands/mod.rs`
- Create: `crates/tplot/src/commands/bar.rs`
- Modify: `crates/tplot/src/main.rs`

- [ ] **Step 1: Write the integration test**

```rust
// crates/tplot/src/commands/bar.rs (top)
#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_sales_csv_with_focal_emea() {
        let csv = include_str!("../../../../tests/fixtures/sales.csv");
        let df = parse_csv_str(csv).unwrap();
        let opts = RenderOptions {
            x: "quarter".into(),
            y: "revenue".into(),
            group: Some("region".into()),
            vertical: false,
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 14,
            palette_name: "signature".into(),
        };
        let out = render_bar(&df, &opts).unwrap();
        // Should contain at least one truecolor escape and the takeaway.
        assert!(out.contains("\x1b[38;2;238;123;61m"),
            "missing focal color escape: {out:?}");
        assert!(out.contains("EMEA"));
    }

    #[test]
    fn neutral_mode_omits_takeaway_when_no_focal() {
        let csv = "x,y\na,50\nb,51\nc,49\nd,50.5\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = RenderOptions {
            x: "x".into(),
            y: "y".into(),
            group: None,
            vertical: false,
            focus: None,
            annotate: None,
            neutral: true,
            no_takeaway: false,
            width: Some(60),
            height: 8,
            palette_name: "signature".into(),
        };
        let out = render_bar(&df, &opts).unwrap();
        assert!(!out.contains("standout"));
    }
}
```

- [ ] **Step 2: Add the wiring**

```rust
// crates/tplot/src/commands/mod.rs
pub mod bar;
pub use bar::{render_bar, RenderOptions};
```

```rust
// crates/tplot/src/pipeline.rs
//! Orchestration helpers shared between subcommands.
//!
//! The pipeline glues input parsing, the story-pass, layout, rasterization,
//! and rendering. Concrete chart subcommands (e.g., `commands::bar`) own the
//! decision of *which* layout/raster functions to call; the pipeline provides
//! the I/O scaffolding.
use anyhow::Result;
use std::io::{self, Read};
use tplot_core::{dataframe::DataFrame, input::{parse_csv_str, parse_json_str}};

pub fn read_dataframe(path: &str) -> Result<DataFrame> {
    let raw = if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(path)?
    };
    if raw.trim_start().starts_with('{') {
        Ok(parse_json_str(&raw)?.dataframe)
    } else {
        Ok(parse_csv_str(&raw)?)
    }
}

pub fn detected_terminal_size(width_override: Option<usize>) -> (usize, usize) {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let w = width_override.unwrap_or(w as usize).max(40);
    (w, (h as usize).max(8))
}
```

```rust
// crates/tplot/src/commands/bar.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::{Column, DataFrame, Series};
use tplot_core::layout::layout_horizontal_bar;
use tplot_core::rasterize::rasterize_bar;
use tplot_core::PixelBuffer;
use tplot_render::render_halfblocks;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_story::{run_bar_story_pass, SeriesPoint};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub x:            String,
    pub y:            String,
    pub group:        Option<String>,
    pub vertical:     bool,
    pub focus:        Option<String>,
    pub annotate:     Option<String>,
    pub neutral:      bool,
    pub no_takeaway:  bool,
    pub width:        Option<usize>,
    /// Used directly in tests; in production callers pass the detected height.
    pub height:       usize,
    pub palette_name: String,
}

pub fn render_bar(df: &DataFrame, opts: &RenderOptions) -> Result<String> {
    if opts.vertical {
        return Err(anyhow!("vertical bars land in plan 2"));
    }

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
    let mut story_cfg = StoryConfig::default();
    story_cfg.enabled  = !opts.neutral;
    story_cfg.takeaway = !opts.no_takeaway;
    if let Some(focus_name) = &opts.focus { story_cfg.focus = FocusMode::Series(focus_name.clone()); }
    if let Some(annotation) = &opts.annotate { story_cfg.annotation = Some(annotation.clone()); }
    let story = run_bar_story_pass(&series_points, &story_cfg, palette);

    // ----- layout ----------------------------------------------------------
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h      = opts.height;

    let agg_df = DataFrame::from_columns(vec![
        Column::new("__label__", Series::Strings(series_points.iter().map(|p| p.key.clone()).collect())),
        Column::new("__value__", Series::Numbers(series_points.iter().map(|p| p.value).collect())),
    ]).map_err(|e| anyhow!(e.to_string()))?;
    let layout = layout_horizontal_bar(&agg_df, "__label__", "__value__", None, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- rasterize -------------------------------------------------------
    // Buffer holds ONLY the plot area. Margins are added by the composer.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_bar(&layout, &story.palette_map, &mut buf);

    // ----- render to halfblocks (one cell row per source cell row) ---------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_halfblocks(&buf, caps);
    let bar_rows: Vec<&str> = body.lines().collect();

    // ----- compose ---------------------------------------------------------
    // For each bar's *top* cell row, build:
    //   "{label:>label_margin}  {colored_block}  {value:>value_margin-2}"
    // For other rows of the same bar, only the colored block is shown.
    let label_margin = layout.label_margin;
    let mut out = String::new();

    for (i, line) in bar_rows.iter().enumerate() {
        // Find the bar whose top cell row matches this row.
        let top_bar = layout.bars.iter().find(|b| (b.pixel_y / 2) == i);
        let any_bar = layout.bars.iter().find(|b| {
            let top    = b.pixel_y / 2;
            let bottom = (b.pixel_y + b.pixel_height.saturating_sub(1)) / 2;
            i >= top && i <= bottom
        });

        if let Some(bar) = top_bar {
            // First (top) row of this bar: full ornament.
            out.push_str(&format!("{:>w$}  ", bar.label, w = label_margin.saturating_sub(2)));
            out.push_str(line);
            out.push_str(&format!("  {:.0}", bar.value));
        } else if any_bar.is_some() {
            // Continuation row of a multi-row bar (only happens when a bar
            // spans 2+ cell rows; in v1 each bar is exactly 1 cell tall, so
            // this branch is rarely hit, but kept for robustness).
            out.push_str(&" ".repeat(label_margin));
            out.push_str(line);
        } else {
            // Pure spacer row between bars.
            out.push_str(&" ".repeat(label_margin));
            out.push_str(line);
        }
        out.push('\n');
    }

    // ----- takeaway --------------------------------------------------------
    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(label_margin));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}
```

- [ ] **Step 3: Wire into main**

```rust
// crates/tplot/src/main.rs (replace previous body)
mod cli;
mod commands;
mod pipeline;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use commands::{render_bar, RenderOptions};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::Bar(b) => {
            let df = pipeline::read_dataframe(&b.input)?;
            let (_, h) = pipeline::detected_terminal_size(b.common.width);
            let out = render_bar(&df, &RenderOptions {
                x:           b.x,
                y:           b.y,
                group:       b.group,
                vertical:    b.vertical,
                focus:       b.common.focus,
                annotate:    b.common.annotate,
                neutral:     b.common.neutral,
                no_takeaway: b.common.no_takeaway,
                width:       b.common.width,
                height:      h,
                palette_name: b.common.palette,
            })?;
            print!("{out}");
            Ok(())
        }
        Command::Json => {
            // Placeholder: JSON mode lands in Task 20.
            eprintln!("--json mode wired in next task");
            Ok(())
        }
    }
}
```

- [ ] **Step 4: Add the deps to tplot Cargo.toml**

```toml
# crates/tplot/Cargo.toml — append [dependencies]
crossterm = { workspace = true }
```

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot --lib
```
Expected: all green.

```bash
cargo run -p tplot -- bar tests/fixtures/sales.csv -x quarter -y revenue --group region
```
Expected: a colored horizontal bar chart, with EMEA in burnt-orange and a takeaway line below.

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Wire bar pipeline: input → story-pass → layout → render"
```

---

## Task 20: `tplot` — JSON mode

**Files:**
- Create: `crates/tplot/src/commands/json.rs`
- Modify: `crates/tplot/src/commands/mod.rs`
- Modify: `crates/tplot/src/main.rs`

- [ ] **Step 1: Write failing test**

```rust
// crates/tplot/src/commands/json.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bar_from_json_protocol() {
        let j = r#"{
            "kind": "bar",
            "orientation": "horizontal",
            "x": {"Column": "region"},
            "y": {"Column": "revenue"},
            "data": {
                "region":  ["NA","EMEA","LATAM","APAC","AU"],
                "revenue": [179, 193, 78, 97, 53]
            }
        }"#;
        let out = render_from_json(j, 80, 12).unwrap();
        assert!(out.contains("EMEA"));
        assert!(out.contains("\x1b[38;2;238;123;61m"));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod json;
pub use json::render_from_json;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot --lib commands::json
```
Expected: FAIL.

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/json.rs (above tests)
use crate::commands::{render_bar, RenderOptions};
use anyhow::{anyhow, Result};
use tplot_core::input::parse_json_str;
use tplot_protocol::{Axis, BarOrientation, ChartKind};

pub fn render_from_json(json: &str, canvas_w: usize, canvas_h: usize) -> Result<String> {
    let parsed = parse_json_str(json).map_err(|e| anyhow!(e.to_string()))?;
    let spec = parsed.spec.ok_or_else(|| anyhow!("missing chart spec in JSON"))?;
    match spec.kind {
        ChartKind::Bar { orientation: BarOrientation::Horizontal } => {
            let x = match spec.x {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline x axis not supported in v1; pass via `data` columns")),
            };
            let y = match spec.y {
                Axis::Column(c) => c,
                _ => return Err(anyhow!("inline y axis not supported in v1")),
            };
            let opts = RenderOptions {
                x, y,
                group:        spec.group,
                vertical:     false,
                focus:        match spec.story.focus {
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
            render_bar(&parsed.dataframe, &opts)
        }
        ChartKind::Bar { orientation: BarOrientation::Vertical } => {
            Err(anyhow!("vertical bars land in plan 2"))
        }
    }
}
```

- [ ] **Step 5: Wire into main**

```rust
// crates/tplot/src/main.rs — replace the Json branch
Command::Json => {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let (w, h) = pipeline::detected_terminal_size(None);
    let out = commands::render_from_json(&buf, w, h)?;
    print!("{out}");
    Ok(())
}
```

- [ ] **Step 6: Run (expected pass)**

```bash
cargo test -p tplot --lib commands::json
cat tests/fixtures/sales.json | cargo run -p tplot -- json
```
Expected: same chart as Task 19's CSV run.

- [ ] **Step 7: Commit**

```bash
git add crates/tplot/
git commit -m "Add --json mode: read ChartSpec from stdin"
```

---

## Task 21: End-to-end snapshot tests

**Files:**
- Create: `crates/tplot/tests/e2e_bar.rs`
- Modify: `crates/tplot/Cargo.toml` (add `insta` to dev-dependencies)

- [ ] **Step 1: Add insta**

```toml
# crates/tplot/Cargo.toml — append
[dev-dependencies]
insta = { workspace = true }
```

- [ ] **Step 2: Write the snapshot tests**

```rust
// crates/tplot/tests/e2e_bar.rs
use std::process::{Command, Stdio};

fn binary_path() -> &'static str { env!("CARGO_BIN_EXE_tplot") }

fn run(args: &[&str]) -> String {
    let out = Command::new(binary_path())
        .args(args)
        .stdout(Stdio::piped())
        .output()
        .expect("tplot binary failed to launch");
    String::from_utf8(out.stdout).expect("non-utf8 stdout")
}

fn strip_ansi(s: &str) -> String {
    // Minimal ANSI strip — good enough for snapshot diff readability.
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
fn bar_default_story_pass_snapshot() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--width", "80",
    ]);
    insta::assert_snapshot!("bar_default", strip_ansi(&out));
}

#[test]
fn bar_neutral_snapshot() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--neutral", "--width", "80",
    ]);
    insta::assert_snapshot!("bar_neutral", strip_ansi(&out));
}

#[test]
fn bar_user_focus_snapshot() {
    let out = run(&[
        "bar", "tests/fixtures/sales.csv",
        "-x", "quarter", "-y", "revenue", "--group", "region",
        "--focus", "NA", "--width", "80",
    ]);
    insta::assert_snapshot!("bar_focus_na", strip_ansi(&out));
}
```

The `tests/fixtures/sales.csv` lives at the workspace root, so add `--manifest-path` is **not** needed: tests run with the workspace root as their working directory.

- [ ] **Step 3: Run the tests once to write initial snapshots**

```bash
cargo test -p tplot --test e2e_bar
```

Expected: tests fail initially because no snapshot exists. Inspect the new `.snap.new` files in `crates/tplot/tests/snapshots/`. If the output looks correct, accept them:

```bash
cargo install cargo-insta --locked
cargo insta accept
```

- [ ] **Step 4: Re-run and confirm green**

```bash
cargo test -p tplot --test e2e_bar
```
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/tplot/tests/ crates/tplot/Cargo.toml
git commit -m "Add end-to-end snapshot tests for bar default/neutral/focus paths"
```

---

## Task 22: README skeleton

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write the README**

````markdown
# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plan 1)

- One chart type: horizontal bar.
- Half-blocks renderer (truecolor + 256/16/mono fallback).
- Story-pass: focal-series detection, gray-down palette, embedded takeaway line.
- CLI form (`tplot bar file.csv -x col -y col --group col`) and JSON form (`cat spec.json | tplot json`).

Vertical bars, line charts, scatter, area, histograms, sparklines, heatmaps, and box plots arrive in subsequent plans.

## Install

```bash
cargo install --path crates/tplot
```

## Quickstart

```bash
echo "region,revenue
NA,179
EMEA,193
LATAM,78
APAC,97
AU,53" | tplot bar - -x region -y revenue
```

## Flags

| Flag | Effect |
|---|---|
| `--focus <name>` | Override auto-focal point |
| `--annotate "text"` | Replace generated takeaway |
| `--neutral` | Skip storytelling pass |
| `--no-takeaway` | Keep story styling, drop the text line |
| `--palette signature\|editorial\|colorblind-safe` | Change accent color |
| `--width N` | Override terminal width |

## Development

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## License

MIT or Apache-2.0, at your option.
````

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "Add README with quickstart and Plan-1 scope notes"
```

---

## Task 23: Workspace lint pass

- [ ] **Step 1: Run clippy across the workspace**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: zero warnings. Fix anything that fires.

- [ ] **Step 2: Format**

```bash
cargo fmt --all
```

- [ ] **Step 3: Final test sweep**

```bash
cargo test --workspace
```
Expected: every test green.

- [ ] **Step 4: Commit any lint/format changes**

```bash
git add -A
git status
# If files were touched:
git commit -m "Lint and format pass at end of plan 1"
```

---

## Done — Plan 1 deliverable

You should now be able to:

```bash
cargo run -p tplot -- bar tests/fixtures/sales.csv -x quarter -y revenue --group region
```

…and see a designed horizontal bar chart with one focal color (EMEA in burnt orange), all other regions desaturated to gray, and an auto-generated takeaway line below the chart.

Try `--neutral` to see the un-styled version, `--focus NA` to override the focal series, and `--annotate "..."` to replace the takeaway.

**What's intentionally still missing** (lands in Plan 2 onward):
- Vertical bars (need Octants renderer)
- Multi-series grouped bars within a single chart
- Line, scatter, histogram, area, sparkline, heatmap, box plot
- `--graphics` image-protocol output
- OSC-based capability probing (Plan 5)

## Self-review notes

Coverage check against the spec:
- Section 4 architecture (5 crates) — Tasks 1–5 (protocol), 6–11 (core), 12–13 (render), 14–17 (story), 18–20 (binary). ✓
- Section 5 data flow — pipeline.rs in Task 19 walks input→story→layout→raster→render. ✓
- Section 6 rendering — half-blocks for horizontal bar; other glyphs deferred to later plans (explicit). ✓
- Section 7 storytelling — focal detection (T14), trust-score gate (T14), palette selection (T15), takeaway (T16), entry point (T17), CLI overrides (T18). ✓
- Section 8 CLI — `tplot bar` form and `tplot json` form both shipped. ✓
- Section 9 MVP charts — only chart 1a (horizontal bar) in this plan; rest deferred and noted. ✓
- Section 10 testing — unit tests in every task, snapshot tests in T21, integration tests in T19/T20. ✓
- Section 11 error handling — DataFrameError did-you-mean (T6), CsvError (T7), JsonError (T8), LayoutError (T10), anyhow at the binary level. ✓
- Section 12 distribution — covered by `cargo install` in README; full Homebrew/wheel story is Plan 6. ✓
