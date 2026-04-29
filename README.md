# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plans 1 + 2 + 3 + 4 + 4.5 + 5)

- Eight chart types: horizontal bar, vertical bar, histogram, line, scatter, sparkline, heatmap, box plot.
- Renderers:
  - half-blocks (truecolor + 256/16/mono fallback) for horizontal bars, heatmaps, and box plots
  - vertical-block elements `▁▂▃▄▅▆▇█` for vertical bars, histograms, and sparklines
  - Braille (2×4 dots/cell) for line and scatter
- Story-pass with chart-specific focal detection (max-value for bars, modal-bin for histograms, largest-delta for lines, point-count for scatter, hottest cell for heatmaps, widest-IQR for box plots), gray-down palette, and embedded takeaway lines. Trust-score gate refuses to highlight when no series clearly dominates. Sparklines stay deliberately minimal — single colored line, no story overhead.
- CLI: `tplot bar [--vertical]`, `tplot hist`, `tplot line`, `tplot scatter`, `tplot spark`, `tplot heatmap`, `tplot box`, `tplot json` (stdin).

Stacked area arrives in a later plan. `--graphics` image-protocol output and full capability detection ship after that.

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
