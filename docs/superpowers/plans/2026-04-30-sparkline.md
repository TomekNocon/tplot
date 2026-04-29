# TerminalPlot — Plan 4: Sparkline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tplot spark` to the binary — a one-line chart using `▁▂▃▄▅▆▇█` block-octant glyphs that's optimized for inline use in scripts. Pipe-friendly: accepts whitespace-separated numbers on stdin, or a single CSV column.

**Architecture:** Tiniest pipeline of any chart type — no axis labels, no takeaway, no multi-series. Just `[f64] → glyph string`. Uses the existing `tplot-render::vertical_blocks` GLYPHS table (the same `▁▂▃▄▅▆▇█` array Plan 2 introduced).

**Why no storytelling treatment:** sparklines are intrinsically minimal — a single line, no comparison context. The SWD principle "use color sparingly" already applies (default: one focal color, no chartjunk). Stripping the story-pass keeps the tool fast and the output 1-line-clean.

**Tech Stack:** Same as prior plans. No new dependencies.

**Inherited context (Plans 1 + 2 + 3):**
- 5 chart types working: horizontal bar, vertical bar, histogram, line, scatter.
- ANSI helpers, half-blocks renderer, vertical-block renderer, Braille renderer all in place.
- 108 tests passing, clippy clean.

---

## File Structure

```
crates/
├── tplot-protocol/src/
│   └── chart.rs                    EXTEND: add Sparkline variant
├── tplot/src/
│   ├── cli.rs                      EXTEND: SparkArgs subcommand
│   ├── commands/
│   │   ├── mod.rs                  EXTEND: re-export new command
│   │   ├── sparkline.rs            ★ NEW: sparkline pipeline
│   │   └── json.rs                 EXTEND: dispatch sparkline from JSON
│   └── main.rs                     EXTEND: Spark match arm
└── tplot/tests/
    └── e2e_sparkline.rs            ★ NEW: snapshot tests
```

No changes to `tplot-core`, `tplot-render`, or `tplot-story` — sparkline is pure binary-side glue over the existing block-octant glyphs.

---

## Task 1: Add Sparkline to ChartKind

**Files:**
- Modify: `crates/tplot-protocol/src/chart.rs`

- [ ] **Step 1: Write failing test (within existing tests module)**

```rust
// crates/tplot-protocol/src/chart.rs (within existing tests module)
#[test]
fn sparkline_spec_round_trip() {
    let spec = ChartSpec {
        kind: ChartKind::Sparkline,
        x: Axis::Column("__index__".into()),
        y: Axis::Column("value".into()),
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
    Histogram {
        #[serde(default)]
        bins: Option<usize>,
    },
    Line,
    Scatter,
    Sparkline,
}
```

- [ ] **Step 3: Run (expected pass)**

```bash
cargo test -p tplot-protocol --lib chart
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Sparkline variant to ChartKind"
```

---

## Task 2: CLI subcommand

**Files:**
- Modify: `crates/tplot/src/cli.rs`

The sparkline subcommand has a deliberately narrower flag set than other charts: no `--group`, no `--focus`, no `--annotate`, no story flags. Just an input path (or `-` for stdin), an optional `-y` to pick a column when input is CSV, and `--color` to override the default focal color.

- [ ] **Step 1: Write failing test (within existing tests module)**

```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_spark_subcommand() {
    let args = Cli::parse_from([
        "tplot", "spark", "metrics.csv", "-y", "latency",
    ]);
    match args.command {
        Command::Spark(s) => {
            assert_eq!(s.input, "metrics.csv");
            assert_eq!(s.y.as_deref(), Some("latency"));
            assert_eq!(s.palette, "signature");
        }
        _ => panic!("expected Spark"),
    }
}

#[test]
fn parses_spark_from_stdin() {
    let args = Cli::parse_from([
        "tplot", "spark", "-",
    ]);
    match args.command {
        Command::Spark(s) => {
            assert_eq!(s.input, "-");
            assert!(s.y.is_none());
        }
        _ => panic!("expected Spark"),
    }
}
```

- [ ] **Step 2: Add the variant**

```rust
// crates/tplot/src/cli.rs — modify Command enum, add SparkArgs

#[derive(Subcommand, Debug)]
pub enum Command {
    Bar(BarArgs),
    Hist(HistArgs),
    Line(LineArgs),
    Scatter(ScatterArgs),
    /// Render a one-line sparkline from a column or whitespace-separated numbers.
    Spark(SparkArgs),
    /// Read a JSON ChartSpec from stdin and render it.
    Json,
}

#[derive(Args, Debug)]
pub struct SparkArgs {
    /// Path to input. Use `-` for stdin. CSV with -y, or whitespace-separated numbers.
    pub input: String,
    /// Column name to extract (only meaningful for CSV input).
    #[arg(short = 'y')]
    pub y: Option<String>,
    /// Color palette: signature | editorial | colorblind-safe.
    #[arg(long, default_value = "signature")]
    pub palette: String,
    /// Skip color (output plain glyphs only).
    #[arg(long)]
    pub no_color: bool,
}
```

- [ ] **Step 3: Run (expected pass)**

```bash
cargo test -p tplot --lib cli
```

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Add spark subcommand to CLI argument parser"
```

---

## Task 3: Sparkline pipeline

**Files:**
- Create: `crates/tplot/src/commands/sparkline.rs`
- Modify: `crates/tplot/src/commands/mod.rs`

The pipeline:

1. **Parse input** with smart-mode dispatch:
   - Bytes start with `{` or `[` → reject (sparkline doesn't take JSON specs at the subcommand level).
   - Bytes look like CSV (have at least one comma in a non-numeric position) → parse as CSV; require `-y col` to pick the numeric column.
   - Otherwise → tokenize into f64s using whitespace and comma as separators. Skip blank tokens and tokens that fail to parse.
2. **Render** by mapping each value to a block-octant glyph. The existing `tplot-render::vertical_blocks::GLYPHS` table is private to that module — copy the same 9-glyph table here. Mapping: `floor((value - min) / (max - min) * 8)`, clamped to 0..=8. Single-token series produces all `▄`.
3. **Emit** the glyph string with optional truecolor escape, followed by a newline.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot/src/commands/sparkline.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whitespace_separated_numbers() {
        let nums = parse_input_numbers("1 3 2 5 4 7", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0, 4.0, 7.0]);
    }

    #[test]
    fn parses_newline_separated_numbers() {
        let nums = parse_input_numbers("1\n3\n2\n5\n", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0]);
    }

    #[test]
    fn parses_comma_separated_numbers() {
        let nums = parse_input_numbers("1, 3, 2, 5", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0]);
    }

    #[test]
    fn parses_csv_column_when_y_given() {
        let csv = "t,latency\n1,42\n2,58\n3,71\n";
        let nums = parse_input_numbers(csv, Some("latency")).unwrap();
        assert_eq!(nums, vec![42.0, 58.0, 71.0]);
    }

    #[test]
    fn rejects_csv_without_y_column() {
        let csv = "t,latency\n1,42\n2,58\n";
        assert!(parse_input_numbers(csv, None).is_err());
    }

    #[test]
    fn glyph_for_full_range() {
        // 9 evenly-spaced values produce all 9 glyphs in order.
        let nums: Vec<f64> = (0..9).map(|i| i as f64).collect();
        let s = render_glyphs(&nums);
        assert!(s.contains('\u{2581}')); // ▁
        assert!(s.contains('\u{2584}')); // ▄
        assert!(s.contains('\u{2588}')); // █
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(parse_input_numbers("", None).is_err());
    }

    #[test]
    fn single_value_renders_a_mid_glyph() {
        let s = render_glyphs(&[42.0]);
        // Single point → no range; render mid-glyph (4/8 = ▄).
        assert!(s.contains('\u{2584}'));
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod sparkline;
pub use sparkline::{render_sparkline, SparkOptions};
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot --lib commands::sparkline
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot/src/commands/sparkline.rs (above tests)
use crate::pipeline::detected_terminal_size;
use anyhow::{anyhow, Result};
use tplot_core::dataframe::Series;
use tplot_core::input::parse_csv_str;
use tplot_protocol::{Capabilities, Palette};
use tplot_render::ansi::{fg, reset};

const GLYPHS: [char; 9] = [
    ' ',
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
    '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

#[derive(Debug, Clone)]
pub struct SparkOptions {
    pub input:        String,   // path or "-"
    pub y:            Option<String>,
    pub palette_name: String,
    pub no_color:     bool,
}

/// Parse the input bytes into a vector of f64. Smart-detects format:
/// - Has a comma followed by a non-numeric token → CSV, requires `y` column.
/// - Otherwise → whitespace + comma tokenized into f64.
pub fn parse_input_numbers(s: &str, y_col: Option<&str>) -> Result<Vec<f64>> {
    if s.trim().is_empty() {
        return Err(anyhow!("empty input"));
    }

    if looks_like_csv(s) {
        let col = y_col.ok_or_else(|| anyhow!(
            "input looks like CSV — pass `-y <column>` to pick the numeric column"
        ))?;
        let df = parse_csv_str(s).map_err(|e| anyhow!(e.to_string()))?;
        let series = df.column(col).map_err(|e| anyhow!(e.to_string()))?.series();
        match series {
            Series::Numbers(v) => Ok(v.clone()),
            Series::Strings(_) => Err(anyhow!("column `{col}` must be numeric")),
        }
    } else {
        let mut nums = Vec::new();
        for tok in s.split(|c: char| c.is_whitespace() || c == ',') {
            let tok = tok.trim();
            if tok.is_empty() { continue; }
            if let Ok(v) = tok.parse::<f64>() {
                nums.push(v);
            }
        }
        if nums.is_empty() {
            return Err(anyhow!("no parseable numeric values in input"));
        }
        Ok(nums)
    }
}

/// Looks like a CSV if the first non-empty line contains a comma AND at
/// least one non-numeric token between commas (i.e., a header row).
fn looks_like_csv(s: &str) -> bool {
    let first = match s.lines().find(|l| !l.trim().is_empty()) {
        Some(l) => l,
        None => return false,
    };
    if !first.contains(',') { return false; }
    first.split(',').any(|t| {
        let t = t.trim();
        !t.is_empty() && t.parse::<f64>().is_err()
    })
}

/// Render a slice of f64 as a glyph-only string (no escape codes).
pub fn render_glyphs(nums: &[f64]) -> String {
    if nums.is_empty() { return String::new(); }
    let min = nums.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = nums.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let span = (max - min).max(1e-9);
    nums.iter().map(|v| {
        let idx = if (max - min).abs() < 1e-9 {
            4
        } else {
            (((v - min) / span) * 8.0).round().clamp(0.0, 8.0) as usize
        };
        GLYPHS[idx]
    }).collect()
}

pub fn render_sparkline(input: &str, opts: &SparkOptions) -> Result<String> {
    let nums = parse_input_numbers(input, opts.y.as_deref())?;
    let glyphs = render_glyphs(&nums);

    if opts.no_color {
        return Ok(format!("{glyphs}\n"));
    }

    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    Ok(format!("{}{glyphs}{}\n", fg(palette.focal_color(), caps.color_depth), reset()))
}

/// Convenience helper for callers that have already loaded the input bytes.
#[allow(dead_code)]
pub fn render_sparkline_from_numbers(nums: &[f64], opts: &SparkOptions) -> Result<String> {
    let _ = detected_terminal_size(None); // capability-detect side effect parity
    let glyphs = render_glyphs(nums);
    if opts.no_color { return Ok(format!("{glyphs}\n")); }
    let palette = Palette::from_name(&opts.palette_name)
        .map_err(|e| anyhow!(e.to_string()))?;
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    Ok(format!("{}{glyphs}{}\n", fg(palette.focal_color(), caps.color_depth), reset()))
}
```

Note: `tplot_render::ansi::{fg, reset}` need to be `pub` in the render crate. Plan 1's `lib.rs` already has `pub mod ansi;` and re-exports `fg`/`bg`/`reset`, so they should be accessible. If not, add `pub use ansi::{fg, bg, reset};` to render's `lib.rs`.

- [ ] **Step 5: Run (expected pass)**

```bash
cargo test -p tplot --lib commands::sparkline
```

- [ ] **Step 6: Commit**

```bash
git add crates/tplot/
git commit -m "Add sparkline pipeline with smart-mode input parsing"
```

---

## Task 4: Wire main.rs and JSON dispatch

**Files:**
- Modify: `crates/tplot/src/main.rs`
- Modify: `crates/tplot/src/commands/json.rs`

- [ ] **Step 1: Wire main.rs**

```rust
// crates/tplot/src/main.rs — add Spark match arm
match args.command {
    Command::Bar(b)     => { /* existing */ }
    Command::Hist(h)    => { /* existing */ }
    Command::Line(l)    => { /* existing */ }
    Command::Scatter(s) => { /* existing */ }
    Command::Spark(s) => {
        let raw = if s.input == "-" {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        } else {
            std::fs::read_to_string(&s.input)?
        };
        let out = commands::render_sparkline(&raw, &commands::SparkOptions {
            input:        s.input,
            y:            s.y,
            palette_name: s.palette,
            no_color:     s.no_color,
        })?;
        print!("{out}");
        Ok(())
    }
    Command::Json => { /* existing */ }
}
```

- [ ] **Step 2: Add Sparkline arm to JSON dispatch**

```rust
// crates/tplot/src/commands/json.rs — add to the match
ChartKind::Sparkline => {
    use tplot_core::dataframe::Series;
    let col = match spec.y {
        Axis::Column(c) => c,
        _ => return Err(anyhow!("inline y axis not supported in v1")),
    };
    let series = parsed.dataframe.column(&col)
        .map_err(|e| anyhow!(e.to_string()))?.series();
    let nums: Vec<f64> = match series {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("column `{col}` must be numeric")),
    };
    let opts = SparkOptions {
        input:        "-".into(),
        y:            Some(col),
        palette_name: "signature".into(),
        no_color:     false,
    };
    crate::commands::sparkline::render_sparkline_from_numbers(&nums, &opts)
}
```

Add the import at the top of `json.rs`:
```rust
use crate::commands::SparkOptions;
```

- [ ] **Step 3: Run + smoke test**

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Whitespace-separated numbers:
echo "1 3 2 5 4 7 9 8 10 6" | cargo run -p tplot -- spark -

# Newline-separated:
{ for i in $(seq 1 30); do echo $((40 + RANDOM % 30)); done } | cargo run -p tplot -- spark -

# CSV with -y column:
cargo run -p tplot -- spark tests/fixtures/sales.csv -y revenue
```

Expected: a single line of glyphs (`▁▂▃▄▅▆▇█` mix) in burnt orange, no axis labels, no takeaway. Just the chart.

- [ ] **Step 4: Commit**

```bash
git add crates/tplot/
git commit -m "Wire spark into main and JSON dispatch"
```

---

## Task 5: End-to-end snapshot tests

**Files:**
- Create: `crates/tplot/tests/e2e_sparkline.rs`

- [ ] **Step 1: Write the snapshot tests**

```rust
// crates/tplot/tests/e2e_sparkline.rs
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
fn sparkline_whitespace_input_snapshot() {
    let out = run_with_stdin(&["spark", "-"], "1 3 2 5 4 7 9 8 10 6");
    insta::assert_snapshot!("sparkline_whitespace", strip_ansi(&out));
}

#[test]
fn sparkline_csv_column_snapshot() {
    let csv = "t,v\n1,10\n2,30\n3,80\n4,200\n5,150\n6,90\n";
    let out = run_with_stdin(&["spark", "-", "-y", "v"], csv);
    insta::assert_snapshot!("sparkline_csv_column", strip_ansi(&out));
}

#[test]
fn sparkline_no_color_snapshot() {
    let out = run_with_stdin(&["spark", "-", "--no-color"], "1 3 2 5 4 7");
    insta::assert_snapshot!("sparkline_no_color", strip_ansi(&out));
}
```

- [ ] **Step 2: Run, inspect, accept**

```bash
cargo test -p tplot --test e2e_sparkline
# Inspect the .snap.new files. They should be one short line of block-element glyphs.
cargo insta accept
cargo test -p tplot --test e2e_sparkline
```

- [ ] **Step 3: Commit**

```bash
git add crates/tplot/tests/
git commit -m "Add end-to-end snapshot tests for sparkline"
```

---

## Task 6: README + lint pass

- [ ] **Step 1: Update README**

Replace the "What's in this version" section so it reads "Plans 1 + 2 + 3 + 4". Add this quickstart:

````markdown
```bash
# Inline sparkline — perfect for piping into scripts
echo "1 3 2 5 4 7 9 8 10 6" | tplot spark -
```

Sparklines accept whitespace-separated numbers OR a CSV column via `-y`:
```bash
ps -A -o %cpu= | head -20 | tplot spark -    # CPU% per process
tplot spark metrics.csv -y latency_ms        # column from a CSV
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
git add -A
git commit -m "Update README and lint pass for plan 4"
```

---

## Done — Plan 4 deliverable

You should now be able to:

```bash
# Inline numbers
echo "1 3 2 5 4 7 9 8 10 6" | tplot spark -

# A column from a CSV
tplot spark metrics.csv -y latency_ms

# Pipe from any tool that emits one number per line
ps -A -o %cpu= | tplot spark -

# Plain output (no ANSI escapes — useful in CI logs)
echo "1 3 2 5 4 7" | tplot spark - --no-color
```

**Still missing for v1** (later plans):
- Heatmap (Plan 5)
- Stacked area, box plot (Plan 6 or beyond)
- `--graphics` image-protocol output
- Distribution / Python wrapper

## Self-review notes

Coverage check:
- ChartKind::Sparkline (Task 1) ✓
- CLI subcommand (Task 2) ✓
- Pipeline with smart-input parsing (Task 3) ✓
- Main + JSON wiring (Task 4) ✓
- Snapshot tests (Task 5) ✓
- README + lint (Task 6) ✓
