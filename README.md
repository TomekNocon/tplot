# TerminalPlot (`tplot`)

Storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

## What's in this version (Plan 1 + 2)

- Three chart types: horizontal bar, vertical bar, histogram.
- Renderers: half-blocks (truecolor + 256/16/mono fallback) for horizontal bars; vertical-block elements (`▁▂▃▄▅▆▇█`) for vertical bars and histograms.
- Story-pass: focal-series detection, gray-down palette, embedded takeaway line. Histograms get a modal-bin treatment with a "Most observations clustered in …" takeaway.
- CLI: `tplot bar [--vertical] FILE -x col -y col --group col`, `tplot hist FILE -x col [--bins N]`, `tplot json` (stdin).

Line / scatter / area / sparkline / heatmap / box plot arrive in subsequent plans.

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
