#!/usr/bin/env bash
# Walk-through of Plan 11.5 — the `tplot ridge` ridgeline (joy-plot) chart.
# Run from project root:    bash scripts/demo-plan11.5.sh
# With pauses:              bash scripts/demo-plan11.5.sh --pause

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

# Reusable datasets.
gen_quarterly_latencies() {
  echo "quarter,ms"
  # q1: tight, low latency
  for _ in $(seq 1 30); do echo "q1,$((30 + RANDOM % 15))"; done
  # q2: slightly wider — load picked up
  for _ in $(seq 1 30); do echo "q2,$((40 + RANDOM % 30))"; done
  # q3: bimodal — fast path + occasional slow path appearing
  for _ in $(seq 1 20); do echo "q3,$((45 + RANDOM % 25))"; done
  for _ in $(seq 1 8);  do echo "q3,$((150 + RANDOM % 50))"; done
  # q4: even worse — heavy tail
  for _ in $(seq 1 18); do echo "q4,$((50 + RANDOM % 30))"; done
  for _ in $(seq 1 10); do echo "q4,$((200 + RANDOM % 80))"; done
  for _ in $(seq 1 4);  do echo "q4,$((350 + RANDOM % 100))"; done
}

gen_team_velocities() {
  echo "team,velocity"
  for _ in $(seq 1 25); do echo "alpha,$((20 + RANDOM % 8))"; done
  for _ in $(seq 1 25); do echo "beta,$((10 + RANDOM % 30))"; done
  for _ in $(seq 1 25); do echo "gamma,$((25 + RANDOM % 12))"; done
  for _ in $(seq 1 25); do echo "delta,$((5 + RANDOM % 50))"; done
}

# ============================================================================
banner "1. The basics — the joy-plot look"
# ============================================================================
section "Quarterly latencies — distributions stacked over time"
caption "Watch each quarter's distribution shape as the system scaled — getting wider with a heavier tail."
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled — every ridge in focal color."
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter --neutral
maybe_pause

section "Override: --focus q1 (the tight, fast-quarter ridge)"
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter --focus q1
maybe_pause

section "Custom takeaway"
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter \
  --annotate "Spread doubles each quarter — capacity is being tested earlier and earlier"
maybe_pause

# ============================================================================
banner "2. Team velocity comparison"
# ============================================================================
section "Four teams' weekly velocity"
caption "alpha steady, beta highly variable, gamma steady-but-higher, delta wildly variable."
gen_team_velocities | $TPLOT ridge - -x velocity --group team
maybe_pause

# ============================================================================
banner "3. Real shell-pipeline data"
# ============================================================================
section "File size distribution per Rust crate (KB)"
{
  echo "crate,kb"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec du -k {} + 2>/dev/null \
      | awk -v c="$name" '{ print c","$1 }'
  done
} | $TPLOT ridge - -x kb --group crate
maybe_pause

section "Lines-per-file distribution per crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec wc -l {} + 2>/dev/null \
      | awk -v c="$name" '$2 != "total" && NF==2 { print c","$1 }'
  done
} | $TPLOT ridge - -x lines --group crate
maybe_pause

# ============================================================================
banner "4. Distribution shape evolution — what ridgelines do best"
# ============================================================================
section "Multimodal distributions in different rows"
caption "Easy to see at a glance: row 1 unimodal, row 2 bimodal, row 3 trimodal, row 4 uniform."
{ echo "kind,v"
  for _ in $(seq 1 60); do echo "1-unimodal,$((40 + RANDOM % 20))"; done
  for _ in $(seq 1 30); do echo "2-bimodal,$((20 + RANDOM % 10))"; done
  for _ in $(seq 1 30); do echo "2-bimodal,$((70 + RANDOM % 10))"; done
  for _ in $(seq 1 25); do echo "3-trimodal,$((20 + RANDOM % 8))"; done
  for _ in $(seq 1 25); do echo "3-trimodal,$((50 + RANDOM % 8))"; done
  for _ in $(seq 1 25); do echo "3-trimodal,$((80 + RANDOM % 8))"; done
  for _ in $(seq 1 80); do echo "4-uniform,$((10 + RANDOM % 90))"; done
} | $TPLOT ridge - -x v --group kind
maybe_pause

# ============================================================================
banner "5. Comparison — violin vs ridgeline (same data)"
# ============================================================================
section "View 1: Violin — vertical, side by side"
caption "Each violin gets its own column; you compare across width."
gen_quarterly_latencies | $TPLOT violin - -x quarter -y ms
maybe_pause

section "View 2: Ridgeline — horizontal, stacked"
caption "Each ridge gets its own row; you compare top-to-bottom along the same x-axis."
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter
maybe_pause

# ============================================================================
banner "6. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
gen_team_velocities | $TPLOT ridge - -x velocity --group team --width 50
maybe_pause

section "Wide (--width 130)"
gen_team_velocities | $TPLOT ridge - -x velocity --group team --width 130
maybe_pause

# ============================================================================
banner "7. Palettes"
# ============================================================================
section "Editorial (azure focal)"
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter --palette editorial
maybe_pause

section "Colorblind-safe"
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter --palette colorblind-safe
maybe_pause

# ============================================================================
banner "8. Graphics protocol output"
# ============================================================================
section "Force iTerm2 (first 80 chars of OSC)"
gen_quarterly_latencies | $TPLOT ridge - -x ms --group quarter --graphics iterm2 \
  | head -c 80 | cat -v
echo
maybe_pause

# ============================================================================
banner "9. JSON-driven mode"
# ============================================================================
section "Ridgeline from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "ridgeline",
  "x": "ms",
  "y": "__count__",
  "group": "month",
  "data": {
    "month": ["jan","jan","jan","jan",
              "feb","feb","feb","feb","feb","feb",
              "mar","mar","mar","mar","mar","mar","mar"],
    "ms":    [40,50,55,60,
              30,45,80,100,130,160,
              5,40,80,150,250,300,350]
  }
}
EOF
maybe_pause

# ============================================================================
banner "10. Edge cases"
# ============================================================================
section "Many groups (8+ rows)"
{ echo "g,v"
  for g in row1 row2 row3 row4 row5 row6 row7 row8; do
    spread=$((10 + RANDOM % 30))
    center=$((30 + RANDOM % 50))
    for _ in $(seq 1 25); do echo "$g,$((center + RANDOM % spread))"; done
  done
} | $TPLOT ridge - -x v --group g
maybe_pause

section "Single group (one ridge)"
{ echo "g,v"; for _ in $(seq 1 50); do echo "only,$((40 + RANDOM % 30))"; done } \
  | $TPLOT ridge - -x v --group g
maybe_pause

# ============================================================================
banner "11. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Non-numeric x column"
echo "g,v
a,foo
b,bar" | $TPLOT ridge - -x v --group g 2>&1 || true
maybe_pause

section "Missing --group"
echo "g,v
a,1
b,2" | $TPLOT ridge - -x v 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 11.5 (ridgeline — 13 chart types total)"
printf '\n\033[2mLast on the v1.5 chart-type list: sankey (flow diagrams). After that, distribution (Plan 8).\033[0m\n\n'
