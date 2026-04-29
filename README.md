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
