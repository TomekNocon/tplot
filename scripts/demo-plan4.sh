#!/usr/bin/env bash
# Focused walk-through of Plan 4 — the `tplot spark` sparkline.
# Run from project root:    bash scripts/demo-plan4.sh
# With pauses:              bash scripts/demo-plan4.sh --pause

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

# Print a label and run a sparkline inline on the same logical line.
inline() {
  printf '  %-32s ' "$1"
  shift
  "$@"
}

# ============================================================================
banner "1. Input formats"
# ============================================================================
section "Whitespace-separated numbers"
caption "Cleanest form for inline use."
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark -
maybe_pause

section "Comma-separated numbers (also works)"
echo "1, 3, 2, 5, 4, 7, 9, 8, 10, 6" | $TPLOT spark -
maybe_pause

section "Newline-separated numbers (one per line)"
{ for i in 1 3 2 5 4 7 9 8 10 6; do echo "$i"; done } | $TPLOT spark -
maybe_pause

section "CSV with -y column"
caption "Smart-mode detects CSV when the first row has non-numeric tokens."
$TPLOT spark tests/fixtures/sales.csv -y revenue
maybe_pause

# ============================================================================
banner "2. Real shell pipelines (the killer use case)"
# ============================================================================
section "Lines per Rust file in this repo"
find crates -name "*.rs" -not -path "*/target/*" -exec wc -l {} + 2>/dev/null \
  | awk '$2 != "total" && NF==2 {print $1}' | sort -n \
  | $TPLOT spark -
maybe_pause

section "File sizes (KB) of all .rs files"
find crates -name "*.rs" -not -path "*/target/*" -exec du -k {} + \
  | awk '{print $1}' | sort -n | $TPLOT spark -
maybe_pause

section "Free memory pages (top 30 from vm_stat)"
vm_stat 2>/dev/null | awk -F: 'NR>1 {gsub(/[ .]/,"",$2); print $2}' \
  | head -20 | $TPLOT spark -
maybe_pause

section "CPU% across all running processes"
ps -A -o %cpu= 2>/dev/null | sort -rn | head -25 | $TPLOT spark -
maybe_pause

section "Latency of pings (10 packets to localhost)"
caption "If you have ping installed; falls back to synthetic data."
if command -v ping >/dev/null; then
  ping -c 10 localhost 2>/dev/null \
    | awk -F'time=' '/time=/ {print $2}' | awk '{print $1}' \
    | $TPLOT spark -
else
  { for _ in $(seq 1 10); do
      awk 'BEGIN{srand(); print 0.05 + rand()*0.15}'
    done } | $TPLOT spark -
fi
maybe_pause

# ============================================================================
banner "3. Synthetic patterns — make sure the glyphs map correctly"
# ============================================================================
section "Strictly ascending"
{ for i in $(seq 1 20); do echo "$i"; done } | $TPLOT spark -
maybe_pause

section "Strictly descending"
{ for i in $(seq 20 -1 1); do echo "$i"; done } | $TPLOT spark -
maybe_pause

section "Sine wave"
{ for i in $(seq 0 30); do
    awk -v i="$i" 'BEGIN { printf "%.2f\n", 50 + 30*sin(i/3) }'
  done } | $TPLOT spark -
maybe_pause

section "Random walk"
awk 'BEGIN { srand(); v=50; for (i=0; i<40; i++) { v += (rand()-0.5)*10; print v } }' \
  | $TPLOT spark -
maybe_pause

section "Step function"
echo "10 10 10 10 50 50 50 50 5 5 5 5 90 90 90 90" | $TPLOT spark -
maybe_pause

# ============================================================================
banner "4. Composing several sparklines (dashboard style)"
# ============================================================================
section "Side-by-side metrics"
caption "Sparklines are one line each — you can stack them with their own labels."
inline "API p50 latency (ms)"  bash -c 'echo "42 45 41 43 44 41 42 40 41 42" | '"$TPLOT"' spark -'
inline "API p99 latency (ms)"  bash -c 'echo "180 175 230 210 320 280 240 260 220 200" | '"$TPLOT"' spark -'
inline "DB p50 latency (ms)"   bash -c 'echo "12 14 11 13 15 14 13 12 13 12" | '"$TPLOT"' spark -'
inline "Error rate (%)"        bash -c 'echo "0 0 0 1 0 0 2 1 0 0" | '"$TPLOT"' spark -'
inline "Cache hit rate (%)"    bash -c 'echo "92 93 91 90 89 88 87 89 91 93" | '"$TPLOT"' spark -'
maybe_pause

section "Daily commits over 30 days"
inline "Mon"  bash -c '{ for _ in $(seq 1 30); do echo $((RANDOM % 8)); done } | '"$TPLOT"' spark -'
inline "Tue"  bash -c '{ for _ in $(seq 1 30); do echo $((RANDOM % 8)); done } | '"$TPLOT"' spark -'
inline "Wed"  bash -c '{ for _ in $(seq 1 30); do echo $((RANDOM % 8)); done } | '"$TPLOT"' spark -'
inline "Thu"  bash -c '{ for _ in $(seq 1 30); do echo $((RANDOM % 8)); done } | '"$TPLOT"' spark -'
inline "Fri"  bash -c '{ for _ in $(seq 1 30); do echo $((RANDOM % 8)); done } | '"$TPLOT"' spark -'
maybe_pause

# ============================================================================
banner "5. Palettes and color flags"
# ============================================================================
section "signature (default — burnt orange)"
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark - --palette signature
maybe_pause

section "editorial (azure blue)"
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark - --palette editorial
maybe_pause

section "colorblind-safe"
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark - --palette colorblind-safe
maybe_pause

section "--no-color (CI-friendly plain output)"
caption "No ANSI escapes. Drop into log files without color noise."
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark - --no-color
maybe_pause

# ============================================================================
banner "6. JSON-driven mode (programmatic drivers)"
# ============================================================================
section "Sparkline via JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "sparkline",
  "x": "__index__",
  "y": "v",
  "data": {
    "v": [1, 3, 2, 5, 4, 7, 9, 8, 10, 6, 12, 11, 14, 13]
  }
}
EOF
maybe_pause

# ============================================================================
banner "7. Edge cases"
# ============================================================================
section "Single value"
caption "No range — engine renders the mid-glyph (▄)."
echo "42" | $TPLOT spark -
maybe_pause

section "All identical"
echo "5 5 5 5 5 5 5 5" | $TPLOT spark -
maybe_pause

section "Two values"
echo "1 100" | $TPLOT spark -
maybe_pause

section "Floats with high precision"
echo "0.123 0.456 0.789 0.234 0.567 0.890" | $TPLOT spark -
maybe_pause

# ============================================================================
banner "8. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Empty input"
echo "" | $TPLOT spark - 2>&1 || true
maybe_pause

section "Non-numeric tokens (none parse → empty)"
echo "foo bar baz" | $TPLOT spark - 2>&1 || true
maybe_pause

section "CSV without -y column specified"
caption "Smart-mode detects CSV but you didn't tell it which column to read."
echo "t,v
1,42
2,58
3,71" | $TPLOT spark - 2>&1 || true
maybe_pause

section "CSV with non-numeric column"
echo "t,v
foo,42
bar,58" | $TPLOT spark - -y t 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 4 (sparkline)"
printf '\n\033[2mFor full Plans 1+2+3+4 walk-through:  bash scripts/demo.sh\033[0m\n'
printf '\033[2mPlan 2 alone:  bash scripts/demo-plan2.sh\033[0m\n'
printf '\033[2mPlan 3 alone:  bash scripts/demo-plan3.sh\033[0m\n\n'
