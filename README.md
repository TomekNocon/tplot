# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plans 1 + 2 + 3 + 4 + 4.5 + 5 + 5.5 + 6 + 7)

- Nine chart types: horizontal bar, vertical bar, histogram, line, scatter, sparkline, heatmap, box plot, stacked area.
- Renderers:
  - half-blocks (truecolor + 256/16/mono fallback) for horizontal bars, heatmaps, box plots, and stacked area
  - vertical-block elements `▁▂▃▄▅▆▇█` for vertical bars, histograms, and sparklines
  - Braille (2×4 dots/cell) for line and scatter
- Story-pass with chart-specific focal detection (max-value for bars, modal-bin for histograms, largest-delta for lines, point-count for scatter, hottest cell for heatmaps, widest-IQR for box plots, largest-total for stacked area), gray-down palette, and embedded takeaway lines. Trust-score gate refuses to highlight when no series clearly dominates. Sparklines stay deliberately minimal — single colored line, no story overhead.
- CLI: `tplot bar [--vertical]`, `tplot hist`, `tplot line`, `tplot scatter`, `tplot spark`, `tplot heatmap`, `tplot box`, `tplot area`, `tplot json` (stdin).
- `tplot doctor` — prints a capability report (color depth, glyph set, theme, graphics-protocol detection). Run it once to see how `tplot` views your terminal.

`--graphics` image-protocol output ships in a later plan; `tplot doctor` already detects which protocol your terminal supports via OSC probes.

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
  • Run with --graphics for high-fidelity image-protocol rendering
    (planned in Plan 7b — pipes a PNG to Kitty escapes).
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
