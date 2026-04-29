# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plans 1 + 2 + 3)

- Five chart types: horizontal bar, vertical bar, histogram, line, scatter.
- Renderers:
  - half-blocks (truecolor + 256/16/mono fallback) for horizontal bars
  - vertical-block elements `▁▂▃▄▅▆▇█` for vertical bars and histograms
  - Braille (2×4 dots/cell) for line and scatter
- Story-pass with chart-specific focal detection (max-value for bars, modal-bin for histograms, largest-delta for lines, point-count for scatter), gray-down palette, and embedded takeaway lines. Trust-score gate refuses to highlight when no series clearly dominates.
- CLI: `tplot bar [--vertical]`, `tplot hist`, `tplot line`, `tplot scatter`, `tplot json` (stdin).

Stacked area, sparkline, heatmap, box plot arrive in Plan 4. `--graphics` image-protocol output and full capability detection in Plan 5.

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
