# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plans 1 + 2 + 3 + 4 + 4.5 + 5 + 5.5 + 6 + 7 + 7b + 9 + 10)

- Fifteen chart types: horizontal bar, vertical bar, histogram, line, scatter, sparkline, heatmap, box plot, stacked area, candlestick, treemap, violin, ridgeline, sankey, table.
- Renderers:
  - half-blocks (truecolor + 256/16/mono fallback) for horizontal bars, heatmaps, box plots, and stacked area
  - vertical-block elements `▁▂▃▄▅▆▇█` for vertical bars, histograms, and sparklines
  - Braille (2×4 dots/cell) for line and scatter
- Story-pass with chart-specific focal detection (max-value for bars, modal-bin for histograms, largest-delta for lines, point-count for scatter, hottest cell for heatmaps, widest-IQR for box plots, largest-total for stacked area), gray-down palette, and embedded takeaway lines. Trust-score gate refuses to highlight when no series clearly dominates. Sparklines stay deliberately minimal — single colored line, no story overhead.
- CLI: `tplot bar [--vertical]`, `tplot hist`, `tplot line`, `tplot scatter`, `tplot spark`, `tplot heatmap`, `tplot box`, `tplot area`, `tplot candle`, `tplot tree`, `tplot violin`, `tplot ridge`, `tplot sankey`, `tplot table`, `tplot summary`, `tplot json` (stdin).
- `tplot doctor` — prints a capability report (color depth, glyph set, theme, graphics-protocol detection). Run it once to see how `tplot` views your terminal.
- `--graphics auto|kitty|iterm2|none` — emit a real PNG via the Kitty or iTerm2 inline-image protocol. `auto` picks the best protocol detected by probing; `none` (default) keeps the half-blocks/Braille text rendering.

## Install

### One-liner (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/TomekNocon/tplot/releases/latest/download/tplot-installer.sh | sh
```

This grabs the right prebuilt binary for your platform, drops it on your `PATH`, and is what most users want.

### Manual download

Pick the archive matching your platform from the [latest release](https://github.com/TomekNocon/tplot/releases/latest):

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `tplot-aarch64-apple-darwin.tar.xz` |
| macOS, Intel         | `tplot-x86_64-apple-darwin.tar.xz` |
| Linux, x86_64 (gnu)  | `tplot-x86_64-unknown-linux-gnu.tar.xz` |

```bash
curl -L https://github.com/TomekNocon/tplot/releases/latest/download/tplot-aarch64-apple-darwin.tar.xz \
  | tar xJ
sudo mv tplot /usr/local/bin/
```

Verify checksums against `SHA256SUMS` published with each release.

### From source

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

```bash
# Histogram of latency values
echo "ms"; seq 1 200 | awk '{print int(50 + 30*sin($1/8) + 30*(rand()-0.5))}' | tplot hist - -x ms
```

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

```bash
# Inline sparkline — perfect for piping into scripts
echo "1 3 2 5 4 7 9 8 10 6" | tplot spark -
```

Sparklines accept whitespace-separated numbers OR a CSV column via `-y`:
```bash
ps -A -o %cpu= | head -20 | tplot spark -    # CPU% per process
tplot spark metrics.csv -y latency_ms        # column from a CSV
```

```bash
# 2D heatmap of activity across day × hour
{ echo "hour,day,count"
  for d in Mon Tue Wed Thu Fri Sat Sun; do
    for h in 9 10 11 12 13 14 15 16 17; do
      echo "$h,$d,$((RANDOM % 50 + 5))"
    done
  done
} | tplot heatmap - -x hour -y day --value count

# With a different ramp
... | tplot heatmap - -x hour -y day --value count --ramp viridis
```

Heat ramps: `inferno` (default), `viridis`, `coolwarm`.

```bash
# Vertical box plots showing latency distribution per endpoint
echo "endpoint,ms
/users,48
/users,50
/users,51
/users,53
/orders,30
/orders,100
/orders,250
/orders,400" | tplot box - -x endpoint -y ms
```

The endpoint with the widest interquartile range is highlighted; the takeaway names its IQR and overall range.

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

The series with the largest total contribution is highlighted; the takeaway names its share of the cumulative total.

```bash
# OHLC candlestick chart from a stocks-style CSV
echo "date,o,h,l,c
d1,100,112,98,110
d2,110,113,99,105
d3,105,118,104,115
d4,115,117,113,115" | tplot candle - -x date --open o --high h --low l --close c
```

Up days (close ≥ open) render in green; down days in red. The thin vertical wick spans intraday low to high; the filled body covers open to close.

```bash
# Treemap of portfolio composition — each rectangle's area is proportional to its weight
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | tplot tree - -x asset -y weight
```

The largest holding is highlighted as the focal rectangle in burnt orange; remaining holdings render in gray. Big enough rectangles show their label inline; a legend lists every leaf with its value.

```bash
# Violin plot — distribution shape per group
echo "endpoint,ms
/users,48
/users,50
/users,52
/orders,30
/orders,80
/orders,250
/orders,400" | tplot violin - -x endpoint -y ms
```

Each violin's mirrored shape traces the kernel density estimate of the values in that group; the horizontal white line marks the median. The group with the widest IQR is highlighted in burnt orange (same focal heuristic as the box plot).

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

Each row is one group's KDE (per-group normalized so every ridge keeps a visible shape), stacked top-to-bottom in first-seen order with controlled overlap. The widest-IQR group is the focal in burnt orange; the rest are grayed-down (same focal heuristic as the box plot and violin).

```bash
# Sankey diagram of a user funnel
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000" | tplot sankey - --source src --target tgt --value flow
```

Nodes are placed in vertical layer columns (assigned via Kahn's topological sort), edges drawn as smoothstep-curved bands with width proportional to flow value. The largest single edge is highlighted in burnt orange; the rest are grayed-down. Errors out cleanly if the input forms a cycle (sankey requires a DAG).

```bash
# Pretty-printed table with inline value bars and focal-row highlighting
tplot table sales.csv --bars revenue --sort revenue --top 10

# Live process table
ps -A -o user,pid,rss,%cpu,comm | tplot table - --bars rss --sort rss --top 15
```

Box-drawn borders (square or `--rounded`), columns auto-typed (numbers right-aligned with thousands separators, booleans rendered as `✓`/`✗` and centered, text left-aligned). The `--bars` column gets an inline `▁▂▃▄▅▆▇█` bar proportional to its column max. The row with the highest value in the bars column lights up in burnt orange when it clearly dominates (trust-score gate); the rest stay context-gray.

```bash
# Single-line data summary — always inline-friendly in Claude Code
echo "v
10
35
80
210" | tplot summary - -y v
# → [4] ▁▃▅█ → median=57 max=210 (#4)

# Categorical summary
echo "lang,lines
rust,1832
shell,541
yaml,128" | tplot summary - -x lang -y lines
# → [3 cats] rust(1832) shell(541) yaml(128) — rust 3.4× median
```

The `summary` chart always emits exactly one line — packed with an embedded sparkline plus headline stats — so it renders fully inline in conversational tools (Claude Code) without triggering multi-line truncation. Two modes: sequence (`-y` only) treats the column as an ordered series; categorical (`-x` + `-y`) ranks categories with focal-orange highlighting on the dominant entry.

```bash
# Inspect what tplot detected about your terminal
$ tplot doctor
TerminalPlot — terminal diagnostic
==================================

Environment
  TERM         = xterm-256color
  TERM_PROGRAM = iTerm.app
  COLORTERM    = truecolor
  COLORFGBG    = (unset)

Detected capabilities
  Color depth       : truecolor (24-bit RGB)
  Glyph set         : Octants + Braille + half-blocks (full set)
  Theme             : dark
  Graphics protocol : Kitty

Recommendations
  • Run with --graphics auto for high-fidelity image rendering
    (encodes a PNG and emits Kitty escapes).
```

```bash
# Auto-pick the best graphics protocol your terminal supports
tplot bar metrics.csv -x quarter -y revenue --group region --graphics auto

# Force a specific protocol (useful when probing fails)
tplot bar metrics.csv -x quarter -y revenue --group region --graphics kitty
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
| `--graphics auto\|kitty\|iterm2\|none` | Emit a PNG via terminal graphics protocol (default `none`) |

## Development

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## License

MIT or Apache-2.0, at your option.
