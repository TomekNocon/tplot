# TerminalPlot (`tplot`) — Design Spec

**Date:** 2026-04-29
**Status:** Draft, awaiting user approval

## 1. Positioning

A storytelling-first chart engine for the terminal — Rust, fast, opinionated by default.

> *"Plotly's quality, Cole Knaflic's discipline, in your terminal."*

Most terminal chart libraries (plotext, youplot, asciichart, gnuplot dumb-terminal) are *brushes*: they hand the user a palette and equal-weight rainbow defaults. The result is technically correct charts that nobody reads carefully because nothing in the chart says where to look. `tplot` ships with the *Storytelling with Data* playbook applied by default — gray-down + focal color + decluttered axes + an embedded "so what" line — so a single command produces a designed chart, not a generated one.

## 2. Goals

- **Visually appealing by default.** Out-of-box output looks designed: one focal series, calm context, clear takeaway.
- **Fast.** Single static binary, ≤10ms cold start. Inline, conversational use is the target.
- **Right glyph for the chart.** Hybrid sub-cell rendering — Braille for curves/scatter, Octants for fills, half-blocks for solid bars — auto-selected.
- **Library access from any language.** JSON-driven engine; Python wrapper ships in v2.
- **Graceful degradation.** Truecolor → 256 → 16 → mono; Octants → half-blocks; image-protocol → text. Always renders something legible.

## 3. Non-goals (v1)

- Diagrams, flowcharts, trees, network graphs (deferred to v2 with diagrams scope).
- Interactive TUI (zoom, pan, hover) — output is one-shot, like `cat`.
- Geographic plots, candlesticks, sankeys, treemaps (deferred to v1.5+).
- Animation / live-updating dashboards.
- Reading directly from databases or remote APIs (input is files / stdin / JSON).

## 4. Architecture

Five crates. Three libraries meet at one binary; nothing else is shared.

```
tplot (binary)
  ├── tplot-core      pure data → pixel-buffer
  ├── tplot-render    pixel-buffer → glyph string
  ├── tplot-story     SWD treatment (focal, declutter, takeaway)
  └── tplot-protocol  JSON schema for --json mode
```

**Why this split:**
- `tplot-story` evolves fastest (heuristics get tuned). Isolating it means iteration doesn't touch rendering math.
- `tplot-render` is glyph bookkeeping; nobody adding a new chart type should need to touch it.
- `tplot-core` is pure functions over data and sub-pixel buffers — fully testable without I/O or terminal.
- `tplot-protocol` is shared types only; downstream language wrappers depend on this crate alone.

**Library independence:** `tplot-core`, `tplot-render`, `tplot-story` do not depend on each other. They meet only at the `tplot` binary. This is enforced by `Cargo.toml` and verified by a CI check.

## 5. Data flow (one chart, end to end)

```
1. INPUT          file/stdin → DataFrame
2. PARSE          CLI args + DataFrame → ChartSpec
3. STORY-PASS     ChartSpec → StoryAnnotated   (skipped if --neutral)
4. LAYOUT         StoryAnnotated + term size → Layout
5. RASTERIZE      Layout → PixelBuffer (sub-pixel, 2×4 per cell, RGB)
6. GLYPH-PICK     PixelBuffer + chart kind + term caps → glyph cells
7. EMIT           glyph cells → ANSI string → stdout
```

**Key design choices:**

- **Universal DataFrame** in stage 1 — every input format converges to one shape. Adding parquet later is a 50-line input plugin, not a pipeline change.
- **Story-pass is its own stage** — `--neutral` literally skips it. Story heuristics can be A/B-tested without touching layout.
- **Sub-pixel buffer is the universal interchange** — same pixels render as Braille, Octants, half-blocks, or PNG depending on the downsampler.
- **Glyph-pick walks regions, not the whole buffer** — a chart can have a Braille line plot AND an Octant fill region in the same image (e.g., area-with-line). Each region maps to its strongest glyph.

## 6. Rendering strategy (hybrid)

| Chart type | Primary glyph | Rationale |
|---|---|---|
| Bar (horizontal) | Half-blocks + truecolor | Solid chunky color; reads cleanly against text labels |
| Bar (vertical) | Octants | 4× vertical sub-cell resolution |
| Line (multi-series) | Braille | Smooth interpolated curves |
| Scatter | Braille | 8 points/cell density encoding |
| Histogram | Octants | Crisp bin edges, solid fills |
| Sparkline | Block octants `▁▂▃▄▅▆▇█` | Inline, single-line, universal font support |
| Heatmap | Sextants | 2D density, 2×3 sub-cell |
| Area (stacked) | Octants | Solid fills at sub-cell resolution |
| Box plot | Half-blocks | Whiskers + IQR boxes; chunky reads correct |

**Capability detection chain:**

1. Probe `$TERM`, `$COLORTERM`, `$TERM_PROGRAM` for color depth and graphics-protocol support.
2. Optional OSC `XTGETTCAP` query (50ms timeout) to confirm Octant/Braille font support.
3. Build a `Capabilities { color_depth, glyph_set, graphics_protocol }` once at startup.
4. Each region's glyph-pick consults this struct; falls back to `Capabilities::conservative()` if probing failed.

**Graphics protocol (`--graphics`):** opt-in escalation to Kitty / iTerm2 / Sixel image protocols when the terminal supports them. Same `PixelBuffer` is encoded as PNG and emitted as inline image escapes. Falls back to text rendering if the requested terminal doesn't support any image protocol; if `--graphics` was explicit, fail with a clear message rather than silently degrading.

## 7. Storytelling layer (the differentiator)

Applied by default. Disable with `--neutral`.

**Story-pass operations** (in order):

1. **Focal-series detection.** Scan numeric series for the largest *delta* (range), *outlier* (z-score > 2), or *max value*, in that priority. Compute a *trust score* — the dominance of the leader vs the median (must exceed 1.5×).
2. **Trust-score gate.** If trust score is too low → no focal series; fall back to neutral palette; takeaway says "no series stands out clearly."
3. **Palette selection.** One signature focal color (default: burnt orange `#ee7b3d`); all non-focal series desaturated to grayscale (`#8b949e` / `#484f58` for layered context). Theme-aware (dark vs light backgrounds).
4. **Declutter pass.** Remove gridlines beyond what's needed; remove chart frame; left-align labels; thin axis ticks to ~5 evenly spaced. Inspired directly by Knaflic, *Storytelling with Data*, ch. 3.
5. **Takeaway generation.** Templated one-liner naming the focal series and its quantitative story (e.g., "EMEA grew +47% — the standout in an otherwise flat year"). Templates per chart type. Suppressed by `--no-takeaway`.

**User overrides:**
- `--focus <series>` — override auto-detected focal point.
- `--annotate "text"` — replace generated takeaway.
- `--neutral` — skip the entire story-pass.
- `--palette <name>` — override signature color (built-in: `signature`, `editorial`, `colorblind-safe`).
- `--no-takeaway` — keep story styling, drop the text line.

**Trust-before-magic principle:** if the engine can't confidently identify what's interesting, it admits it rather than imposing a wrong story. This is the design difference between "smart defaults" and "lying with color."

## 8. CLI ergonomics

Two surfaces — friction-free for inline use, structured for programmatic use.

### Inline form
```bash
tplot bar sales.csv -x quarter -y revenue --group region
tplot line metrics.json -x time -y latency.p95
tplot scatter users.csv -x signup_age -y session_count
echo '[1,3,2,5,4,7]' | tplot spark
cat /var/log/app.log | tplot hist
```

### Universal flags
| Flag | Purpose |
|---|---|
| `--focus <name>` | Override auto-focal point |
| `--annotate "text"` | Custom takeaway line |
| `--neutral` | Skip story-pass |
| `--no-takeaway` | Story styling without the text |
| `--graphics` | Force image-protocol rendering |
| `--width N` | Override terminal cell width |
| `--theme dark\|light\|auto` | Theme override |
| `--palette <name>` | Color palette override |
| `--out png\|svg\|txt\|-` | Render to file instead of stdout |
| `--no-color` | Honor `$NO_COLOR` semantics |

### JSON-driven form
```bash
tplot --json <<EOF
{ "kind": "bar",
  "x": ["Q1","Q2","Q3","Q4"],
  "y": [42, 58, 71, 52],
  "story": { "focus": "auto", "takeaway": "auto" } }
EOF
```

The JSON schema is the engine's true API. The CLI is a convenience layer that builds JSON internally.

### Python wrapper (v2)
```python
import tplot
tplot.bar(df, x="quarter", y="revenue", group="region")  # auto-renders to terminal
```
Subprocess wrapper around the binary. No PyO3 / no compiled extension. `pip install tplot` pulls the binary via prebuilt wheels.

## 9. MVP chart set

Eight charts ship in v1, prioritized by frequency × differentiation:

1. Bar (horizontal + vertical)
2. Line (multi-series, time-series defaults)
3. Scatter
4. Histogram
5. Sparkline
6. Heatmap
7. Area (stacked)
8. Box plot

**Deferred to v1.5+:** violin, ridgeline, treemap, sankey, candlestick, geographic.
**Deferred to v2 (diagrams scope):** flowchart, tree, state diagram, network graph.

## 10. Testing strategy

| Layer | Test type | Coverage target |
|---|---|---|
| `tplot-core` | Unit + property | Tick selection, bin-width algos, layout under random data shapes (proptest) |
| `tplot-render` | Snapshot (golden file) | ~80 snapshots: chart × renderer × color-mode combos. ANSI stripped for diff readability; colored copy stored separately for visual review |
| `tplot-story` | Unit | Each rule (focal-detect, trust-score, palette, takeaway template) a separate test with curated fixtures |
| `tplot` (CLI) | Integration | One test per documented README example. Compares stdout against fixtures |
| `tplot-protocol` | Schema | JSON validates against schema; serialize/deserialize round-trip |

**Snapshot workflow:** `cargo insta review` (or `cargo test -- --update-snapshots`) to refresh; reviewers see diff in PR. Same pattern as `insta` (Rust) / Jest snapshots (JS).

**Visual regression:** stripped-ANSI fixtures show up cleanly in PR diffs. Image-protocol output uses a smaller binary fixture set with hash comparison.

**TDD where it earns its keep:** the story-pass heuristics and the layout math are pure-functional and deserve tests-first. Glyph rendering is most efficiently developed against snapshots — write the chart, eyeball the output, lock it in.

## 11. Error handling

| Class | Strategy | Exit code |
|---|---|---|
| Bad CSV / type mismatch | Row/col location with colored caret at the offending cell | 2 |
| Missing column (`-x foo` not present) | "did you mean: `<closest column>`" | 2 |
| Terminal too narrow (< 40 cols) | Fail with width recommendation | 3 |
| Terminal too narrow (40–60 cols) | Render minimum legible; warn once on stderr | 0 |
| `$NO_COLOR` set | Story-pass uses bold/underline for focal series; charts still render | 0 |
| `--graphics` requested but unsupported | Fail with detected-protocol-list message | 4 |
| Story can't find focal point (low trust score) | Fall back to neutral palette; takeaway says "no series stands out" | 0 |
| Glyph not in terminal font | Auto-fallback to half-blocks; no warning | 0 |

Errors are designed to look like compiler output — clear location, clear suggestion, no panic stacktraces.

## 12. Distribution & dependencies

**Distribution:**
- Single static binary via `cargo install tplot`.
- Homebrew formula (`brew install tplot`).
- Prebuilt artifacts (Linux x86_64/arm64, macOS Intel/Apple Silicon, Windows x86_64) attached to GitHub releases.
- Python wheel (v2) — prebuilt wheels per-arch; pure subprocess wrapper.

**Runtime dependencies:** none. Single static binary.

**Build dependencies (selected):**
- `clap` — CLI argument parsing.
- `serde` + `serde_json` — JSON protocol.
- `csv` — input parsing.
- `polars` (optional, behind `dataframes` feature) — for parquet/feather support. Default-off to keep binary small.
- `crossterm` — terminal capability detection (color depth, size).
- `image` + `png` — image-protocol rendering.
- `insta` — snapshot tests.
- `proptest` — property tests.

**Binary size target:** ≤8 MB stripped (default features), ≤25 MB with `dataframes` feature.

## 13. Out of scope (v1)

- Diagrams (flowchart, tree, network) — v2.
- Interactive TUI — non-goal.
- Animation / live-updating — non-goal.
- Geographic plots — v1.5+.
- Bidirectional / RTL text in labels — v1.5+.
- Database/HTTP input adapters — v1.5+ if demanded; users pipe via stdin in v1.
- Plugin system for custom chart types — v2 if real demand emerges.

## 14. Open questions / risks

1. **Octant font support.** Unicode 16 is recent (Sept 2024); some users may have older fonts that render Octants as boxes. Mitigation: fallback chain to half-blocks, and a `tplot doctor` command that probes the user's font and reports gaps.
2. **Auto-takeaway quality.** Heuristic templates can produce stilted prose. Mitigation: trust-score gate (admit ambiguity rather than guess), and easy `--annotate` override.
3. **Theme detection on Linux.** No standard way to query terminal background. Mitigation: `$COLORFGBG`, then `$TERM_BACKGROUND`, then `--theme` flag, then sensible default for unknown.
4. **Tmux + image protocols.** Tmux often strips graphics escapes. Mitigation: detect `$TMUX` and warn when `--graphics` is requested inside it.
