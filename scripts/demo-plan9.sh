#!/usr/bin/env bash
# Walk-through of Plan 9 — the `tplot candle` OHLC candlestick chart.
# Run from project root:    bash scripts/demo-plan9.sh
# With pauses:              bash scripts/demo-plan9.sh --pause

set -u

TPLOT="${TPLOT:-./target/release/tplot}"
PAUSE=0
[[ "${1:-}" == "--pause" || "${1:-}" == "-p" ]] && PAUSE=1

if [[ ! -x "$TPLOT" ]]; then
  echo "Building release binary first…" >&2
  cargo build --release -p tplot >/dev/null 2>&1 || { echo "Build failed."; exit 1; }
fi

# ---------------------------------------------------------------- helpers ---
banner() {
  printf '\n\033[1;48;2;30;30;30;38;2;238;123;61m  %s  \033[0m\n' "$1"
}
section() {
  printf '\n\033[2m── %s ──────────────────────────────────────\033[0m\n\n' "$1"
}
caption() {
  printf '\033[2;3m%s\033[0m\n\n' "$1"
}
maybe_pause() {
  if (( PAUSE )); then
    printf '\n\033[2m[press enter to continue]\033[0m'
    read -r _
  fi
}

# Generate a random walk of OHLC bars.
# Args: 1=count  2=starting_price  3=trend_per_step (positive=bullish)
# Note: avoids `close` (BSD-awk reserved word) by using `op`, `hi`, `lo`, `cl`.
gen_ohlc() {
  local count="$1" price="$2" trend="$3"
  echo "date,o,h,l,c"
  awk -v n="$count" -v price="$price" -v trend="$trend" 'BEGIN {
    srand()
    for (i = 1; i <= n; i++) {
      op = price
      # Random intraday swing of up to ±5% with a little trend bias.
      swing_hi = op * (1 + (rand() * 0.04 + 0.005))
      swing_lo = op * (1 - (rand() * 0.04 + 0.005))
      # Close drifts toward open + trend, with random noise.
      cl = op + trend + (rand() - 0.5) * op * 0.03
      # Make sure hi/lo bracket open and close.
      hi = (cl > op ? cl : op) > swing_hi ? (cl > op ? cl : op) : swing_hi
      lo = (cl < op ? cl : op) < swing_lo ? (cl < op ? cl : op) : swing_lo
      printf "d%02d,%.0f,%.0f,%.0f,%.0f\n", i, op, hi, lo, cl
      price = cl
    }
  }'
}

# ============================================================================
banner "1. The basics — the up/down convention"
# ============================================================================
section "Hand-crafted small example (8 days)"
caption "Up days = green (close >= open), down days = red. Wick lines extend high to low."
echo "date,o,h,l,c
d1,100,112,98,110
d2,110,113,99,105
d3,105,118,104,115
d4,115,117,113,115
d5,115,121,110,118
d6,118,119,114,116
d7,116,125,116,124
d8,124,127,120,121" | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

# ============================================================================
banner "2. Market regimes — bull, bear, sideways"
# ============================================================================
section "Bull market — clear upward trend"
caption "Mostly green candles with each day closing higher than the last."
gen_ohlc 18 100 1.5 | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

section "Bear market — sustained decline"
caption "Mostly red candles, market grinding down."
gen_ohlc 18 200 -1.8 | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

section "Sideways/choppy — no clear direction"
caption "Mix of up and down with little net movement."
gen_ohlc 18 100 0 | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

# ============================================================================
banner "3. Edge cases"
# ============================================================================
section "Doji days (open == close, body collapses to a line)"
caption "When close exactly equals open, the body becomes thin — convention treats doji as up (green)."
echo "date,o,h,l,c
d1,100,108,95,100
d2,100,102,95,100
d3,100,110,90,100
d4,100,103,98,100" | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

section "Long wicks (large intraday range, small open-close)"
caption "Body is small but high/low extend far on either side — typical of volatile days."
echo "date,o,h,l,c
d1,100,140, 60,102
d2,102,135, 70,101
d3,101,150, 50,103
d4,103,130, 75,104" | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

section "Single candle — minimal valid input"
echo "date,o,h,l,c
d1,100,115,95,108" | $TPLOT candle - -x date --open o --high h --low l --close c
maybe_pause

# ============================================================================
banner "4. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
gen_ohlc 6 100 1 | $TPLOT candle - -x date --open o --high h --low l --close c --width 50
maybe_pause

section "Wide (--width 130)"
gen_ohlc 24 100 0.8 | $TPLOT candle - -x date --open o --high h --low l --close c --width 130
maybe_pause

# ============================================================================
banner "5. Graphics protocol output"
# ============================================================================
section "Force iTerm2 inline-image output (first 80 chars of OSC)"
caption "Same chart as PNG. Renders inline in iTerm2; here we show the escape header."
gen_ohlc 12 100 1 | $TPLOT candle - -x date --open o --high h --low l --close c \
  --graphics iterm2 | head -c 80 | cat -v
echo
maybe_pause

section "Auto-pick (uses Capabilities.graphics_protocol)"
caption "If your terminal supports an image protocol, this renders as a real PNG. Otherwise text."
gen_ohlc 12 100 1 | $TPLOT candle - -x date --open o --high h --low l --close c --graphics auto | head -c 80 | cat -v
echo
maybe_pause

# ============================================================================
banner "6. Real-world data — daily Bitcoin-style OHLC (synthesized)"
# ============================================================================
section "30 days of synthetic 'BTC' price action"
{
  echo "day,o,h,l,c"
  awk 'BEGIN {
    srand(42)
    price = 50000
    for (i = 1; i <= 30; i++) {
      op  = price
      vol = price * (0.02 + rand() * 0.04)
      hi  = op + vol * (0.5 + rand())
      lo  = op - vol * (0.5 + rand())
      cl  = op + (rand() - 0.45) * vol * 1.5   # slight upward bias
      hi  = (cl > hi ? cl : hi)
      lo  = (cl < lo ? cl : lo)
      printf "%d,%.0f,%.0f,%.0f,%.0f\n", i, op, hi, lo, cl
      price = cl
    }
  }'
} | $TPLOT candle - -x day --open o --high h --low l --close c --width 110
maybe_pause

# ============================================================================
banner "7. JSON-driven mode"
# ============================================================================
section "Candlestick from a JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "candlestick",
  "open":  "o",
  "high":  "h",
  "low":   "l",
  "close": "c",
  "x":     "date",
  "y":     "c",
  "data": {
    "date": ["d1","d2","d3","d4","d5","d6"],
    "o":    [100, 110, 105, 115, 115, 118],
    "h":    [112, 113, 118, 117, 121, 119],
    "l":    [ 98,  99, 104, 113, 110, 114],
    "c":    [110, 105, 115, 115, 118, 116]
  }
}
EOF
maybe_pause

# ============================================================================
banner "8. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Missing one of the OHLC columns"
echo "date,o,h,l,c
d1,100,112,98,110" | $TPLOT candle - -x date --open o --high h --low low --close c 2>&1 || true
maybe_pause

section "Non-numeric OHLC column"
echo "date,o,h,l,c
d1,foo,bar,baz,qux" | $TPLOT candle - -x date --open o --high h --low l --close c 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 9 (candlestick — 10 chart types total)"
printf '\n\033[2mNext on the v1.5 list: treemap, then violin/ridgeline, then sankey.\033[0m\n\n'
