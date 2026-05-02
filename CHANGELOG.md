# Changelog

All notable changes to `tplot` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While < 1.0.0, the CLI surface (flag names, output format) may change between
minor versions.

## [Unreleased]

## [0.1.1] - 2026-05-02

### Added

- Homebrew formula auto-published to `tomeknocon/homebrew-tap` on each
  release. Install via `brew install tomeknocon/tap/tplot` (or the
  shorter `brew install tplot` after a one-time `brew tap tomeknocon/tap`).

## [0.1.0] - 2026-05-01

### Added

- 16 chart types covering common analysis needs:
  bar, hist, line, scatter, sparkline, heatmap, box, area, candlestick,
  treemap, violin, ridgeline, sankey, table, summary.
- Storytelling layer: focal/context palette (burnt orange focal, gray
  context), trust-score gate (max ≥ 1.5× median), and embedded one-line
  takeaway. `--neutral`, `--focus`, `--annotate`, `--no-takeaway` flags
  for fine control.
- Hybrid renderers — half-blocks (`▀▄█`), vertical-blocks (`▁▂▃▄▅▆▇█`),
  and Braille (2×4 dots) — auto-selected based on chart type.
- Image protocols: Kitty graphics protocol, iTerm2 inline-image (OSC 1337),
  auto-detected via OSC capability probing with timeouts.
- Theme detection (light/dark) via `$COLORFGBG` and `$TERM_PROGRAM`.
- Single-line `tplot summary` chart, designed for inline rendering in
  Claude Code where multi-line output is collapsed behind Ctrl+O.
- JSON ChartSpec dispatch via `tplot json` for programmatic input.
- Per-renderer ANSI escape compaction (3.2-13.6× byte reduction in tests).
- Three palettes: `signature`, `editorial`, `colorblind-safe`.
- 348 tests across the 5-crate workspace.

[Unreleased]: https://github.com/TomekNocon/tplot/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/TomekNocon/tplot/releases/tag/v0.1.1
[0.1.0]: https://github.com/TomekNocon/tplot/releases/tag/v0.1.0
