#!/usr/bin/env bash
# Focused walk-through of Plan-3 features: line + scatter (Braille rendering).
# Run from project root:    bash scripts/demo-plan3.sh
# With pauses:              bash scripts/demo-plan3.sh --pause

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

# Reusable datasets:
gen_two_series_lines() {
  echo "t,v,g"
  for t in 1 2 3 4 5 6 7 8 9 10; do
    a=$(( 50 + (RANDOM % 6) - 3 ))                 # A: noisy flat around 50
    b=$(( t * t * 2 ))                              # B: clearly growing (quadratic)
    echo "$t,$a,A"
    echo "$t,$b,B"
  done
}

gen_sin_wave() {
  echo "t,v"
  for i in $(seq 0 50); do
    # sin(i/8) scaled and shifted, with mild noise
    awk -v i="$i" 'BEGIN { v = 50 + 30*sin(i/8); printf "%d,%.2f\n", i, v + (rand()-0.5)*4 }'
  done
}

gen_arc_scatter() {
  echo "x,y"
  for x in $(seq 1 20); do
    awk -v x="$x" 'BEGIN { y = 50 - (x-10)^2; if (y<0) y=0; printf "%d,%.0f\n", x, y + (rand()-0.5)*8 }'
  done
}

gen_clustered_scatter() {
  echo "x,y,cluster"
  # cluster A: tight cloud near (3, 30)
  for _ in $(seq 1 20); do
    x=$(( 1 + RANDOM % 6 )); y=$(( 25 + RANDOM % 12 )); echo "$x,$y,A"
  done
  # cluster B: tight cloud near (15, 70)
  for _ in $(seq 1 20); do
    x=$(( 13 + RANDOM % 6 )); y=$(( 65 + RANDOM % 12 )); echo "$x,$y,B"
  done
  # cluster C: sparse outliers (just 4 points)
  for _ in $(seq 1 4); do
    x=$(( RANDOM % 20 )); y=$(( RANDOM % 80 )); echo "$x,$y,C"
  done
}

# ============================================================================
banner "1. Line charts — single series"
# ============================================================================
section "Smooth sine wave (50 points)"
caption "Notice the Braille dots producing genuinely smooth curves at sub-cell resolution."
gen_sin_wave | $TPLOT line - -x t -y v
maybe_pause

section "Same data, --neutral (story-pass disabled)"
gen_sin_wave | $TPLOT line - -x t -y v --neutral
maybe_pause

# ============================================================================
banner "2. Line charts — multi-series with focal-trend detection"
# ============================================================================
section "Two series — flat A and growing B"
caption "B clearly dominates by delta (last-first). Engine highlights it; A recedes to gray."
gen_two_series_lines | $TPLOT line - -x t -y v --group g
maybe_pause

section "Override: --focus A"
caption "Force A as focal. Takeaway adjusts."
gen_two_series_lines | $TPLOT line - -x t -y v --group g --focus A
maybe_pause

section "Custom takeaway"
gen_two_series_lines | $TPLOT line - -x t -y v --group g \
  --annotate "B's quadratic growth dwarfs A's noise — investigate B's drivers"
maybe_pause

# ============================================================================
banner "3. Line charts on real data"
# ============================================================================
section "Synthetic latency over time"
{ echo "t,ms"; for i in $(seq 1 80); do
  awk -v i="$i" 'BEGIN { v = 50 + 20*sin(i/12) + (rand()-0.5)*15; printf "%d,%.0f\n", i, v }'
done } | $TPLOT line - -x t -y ms
maybe_pause

section "Cumulative commits (simulated)"
{ echo "day,commits"
  total=0
  for d in $(seq 1 30); do
    today=$(( RANDOM % 5 ))
    total=$(( total + today ))
    echo "$d,$total"
  done
} | $TPLOT line - -x day -y commits
maybe_pause

# ============================================================================
banner "4. Scatter — single series"
# ============================================================================
section "Inverted parabola (peak around x=10)"
caption "Single ungrouped series — engine should NOT impose a focal cluster."
gen_arc_scatter | $TPLOT scatter - -x x -y y
maybe_pause

section "Random uniform 100 points"
{ echo "x,y"; for _ in $(seq 1 100); do echo "$((RANDOM % 100)),$((RANDOM % 100))"; done } \
  | $TPLOT scatter - -x x -y y
maybe_pause

# ============================================================================
banner "5. Scatter — grouped clusters with focal-by-density"
# ============================================================================
section "Three clusters: dense A, dense B, sparse outliers C"
caption "A and B both have ~20 points; C has only 4. Story-pass picks the most-populous cluster as focal — but only if it dominates clearly."
gen_clustered_scatter | $TPLOT scatter - -x x -y y --group cluster
maybe_pause

section "Override: --focus C (highlight the sparse outliers)"
gen_clustered_scatter | $TPLOT scatter - -x x -y y --group cluster --focus C
maybe_pause

# ============================================================================
banner "6. Same data, three views (line vs scatter vs neither)"
# ============================================================================
section "View 1: Line"
{ echo "t,v"; for i in $(seq 1 30); do
  awk -v i="$i" 'BEGIN { v = 30 + 15*sin(i/4); printf "%d,%.0f\n", i, v }'
done } | $TPLOT line - -x t -y v
maybe_pause

section "View 2: Scatter (same points, no connecting lines)"
{ echo "t,v"; for i in $(seq 1 30); do
  awk -v i="$i" 'BEGIN { v = 30 + 15*sin(i/4); printf "%d,%.0f\n", i, v }'
done } | $TPLOT scatter - -x t -y v
maybe_pause

# ============================================================================
banner "7. Width adaptation"
# ============================================================================
section "Line chart at --width 50"
gen_sin_wave | $TPLOT line - -x t -y v --width 50
maybe_pause

section "Line chart at --width 130"
gen_sin_wave | $TPLOT line - -x t -y v --width 130
maybe_pause

section "Scatter at --width 50"
gen_arc_scatter | $TPLOT scatter - -x x -y y --width 50
maybe_pause

# ============================================================================
banner "8. Palettes on the new chart types"
# ============================================================================
section "Line — editorial palette (azure)"
gen_two_series_lines | $TPLOT line - -x t -y v --group g --palette editorial
maybe_pause

section "Scatter — colorblind-safe (deuteranopia-safe blue)"
gen_clustered_scatter | $TPLOT scatter - -x x -y y --group cluster --palette colorblind-safe
maybe_pause

# ============================================================================
banner "9. JSON-driven mode (line + scatter via stdin)"
# ============================================================================
section "Line chart from JSON spec"
caption "This is the engine's true API for programmatic drivers (Python, Node, etc.)."
cat <<'EOF' | $TPLOT json
{
  "kind": "line",
  "x": "t",
  "y": "v",
  "group": "g",
  "data": {
    "t": [1,2,3,4,5,1,2,3,4,5],
    "v": [10,11,9,12,11,5,30,80,150,220],
    "g": ["A","A","A","A","A","B","B","B","B","B"]
  }
}
EOF
maybe_pause

# ============================================================================
banner "10. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Line on a non-numeric x column"
$TPLOT line tests/fixtures/sales.csv -x region -y revenue 2>&1 || true
maybe_pause

section "Scatter on a non-numeric y column"
$TPLOT scatter tests/fixtures/sales.csv -x revenue -y quarter 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 3 (Braille + line/scatter)"
printf '\n\033[2mFor full Plans 1+2+3 walk-through:  bash scripts/demo.sh\033[0m\n'
printf '\033[2mFor Plan 2 alone (vertical bars + histograms):  bash scripts/demo-plan2.sh\033[0m\n\n'
