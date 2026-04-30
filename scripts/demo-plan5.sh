#!/usr/bin/env bash
# Focused walk-through of Plan 5 — the `tplot box` box plot.
# Run from project root:    bash scripts/demo-plan5.sh
# With pauses:              bash scripts/demo-plan5.sh --pause

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
gen_endpoint_latencies() {
  echo "endpoint,ms"
  # /health: tight cluster around 6ms
  for _ in $(seq 1 20); do echo "/health,$((4 + RANDOM % 6))"; done
  # /users: medium spread around 50ms
  for _ in $(seq 1 20); do echo "/users,$((40 + RANDOM % 25))"; done
  # /search: wider spread, occasional spikes
  for _ in $(seq 1 18); do echo "/search,$((60 + RANDOM % 60))"; done
  for _ in $(seq 1 4);  do echo "/search,$((150 + RANDOM % 80))"; done
  # /checkout: very wide spread (the focal candidate)
  for _ in $(seq 1 15); do echo "/checkout,$((100 + RANDOM % 100))"; done
  for _ in $(seq 1 5);  do echo "/checkout,$((300 + RANDOM % 200))"; done
}

gen_score_by_team() {
  echo "team,score"
  for _ in $(seq 1 25); do echo "alpha,$((70 + RANDOM % 15))"; done
  for _ in $(seq 1 25); do echo "beta,$((40 + RANDOM % 50))"; done
  for _ in $(seq 1 25); do echo "gamma,$((60 + RANDOM % 25))"; done
  for _ in $(seq 1 25); do echo "delta,$((75 + RANDOM % 12))"; done
}

# ============================================================================
banner "1. The canonical use case — endpoint latency distributions"
# ============================================================================
section "Default — auto-detect widest spread"
caption "Story-pass picks /checkout (clearly widest IQR) and grays the rest."
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled — every box in focal color, no takeaway."
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --neutral
maybe_pause

section "Override: --focus /users"
caption "Force /users as focal even though it doesn't have the widest IQR."
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --focus /users
maybe_pause

section "Custom takeaway"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms \
  --annotate "/checkout's tail is the smoking gun — investigate the slow path"
maybe_pause

# ============================================================================
banner "2. Score distributions across teams"
# ============================================================================
section "4 teams — beta has the widest spread"
caption "alpha and delta are tight high performers; beta is highly variable."
gen_score_by_team | $TPLOT box - -x team -y score
maybe_pause

# ============================================================================
banner "3. Real shell pipeline data"
# ============================================================================
section "File-size distribution by crate (KB)"
{
  echo "crate,kb"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec du -k {} + 2>/dev/null \
      | awk -v c="$name" '{ print c","$1 }'
  done
} | $TPLOT box - -x crate -y kb
maybe_pause

section "Lines-of-code distribution by crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec wc -l {} + 2>/dev/null \
      | awk -v c="$name" '$2 != "total" && NF==2 { print c","$1 }'
  done
} | $TPLOT box - -x crate -y lines
maybe_pause

section "Commit message length per author (chars)"
{
  echo "author,len"
  git log --format='%an|%s' 2>/dev/null \
    | awk -F'|' '{ printf "%s,%d\n", $1, length($2) }'
} | $TPLOT box - -x author -y len 2>&1 || echo "(no git history)"
maybe_pause

# ============================================================================
banner "4. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --width 50
maybe_pause

section "Wide (--width 130)"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --width 130
maybe_pause

# ============================================================================
banner "5. Palettes"
# ============================================================================
section "Editorial (azure)"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --palette editorial
maybe_pause

section "Colorblind-safe"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms --palette colorblind-safe
maybe_pause

# ============================================================================
banner "6. Edge cases"
# ============================================================================
section "Single group (no comparison context)"
caption "With one box, story-pass has no peer to compare against."
{ echo "x,v"; for _ in $(seq 1 20); do echo "only,$((50 + RANDOM % 30))"; done } \
  | $TPLOT box - -x x -y v
maybe_pause

section "All identical — IQR = 0"
echo "x,v
a,50
a,50
a,50
a,50
b,50
b,50
b,50
b,50" | $TPLOT box - -x x -y v
maybe_pause

section "Asymmetric distributions (right-skewed)"
caption "Long upper whiskers, short lower — typical of latency data."
{ echo "g,v"
  for _ in $(seq 1 30); do echo "skewed,$((20 + RANDOM % 15))"; done
  for _ in $(seq 1 5);  do echo "skewed,$((100 + RANDOM % 100))"; done
  for _ in $(seq 1 30); do echo "normal,$((50 + RANDOM % 20))"; done
} | $TPLOT box - -x g -y v
maybe_pause

section "Many groups (8 categories)"
{ echo "g,v"
  for g in a b c d e f g h; do
    spread=$((RANDOM % 50 + 10))
    center=$((RANDOM % 80 + 30))
    for _ in $(seq 1 15); do echo "$g,$((center + RANDOM % spread))"; done
  done
} | $TPLOT box - -x g -y v
maybe_pause

# ============================================================================
banner "7. Composing with other charts (same data, multiple views)"
# ============================================================================
section "View 1: Box plots — show distribution shape"
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms
maybe_pause

section "View 2: Horizontal bar — show totals per endpoint"
gen_endpoint_latencies | $TPLOT bar - -x endpoint -y ms
maybe_pause

section "View 3: Histogram — show overall distribution shape"
{ echo "all_ms"; gen_endpoint_latencies | tail -n +2 | awk -F, '{print $2}'; } \
  | $TPLOT hist - -x all_ms
maybe_pause

# ============================================================================
banner "8. JSON-driven mode"
# ============================================================================
section "Box plot from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "box_plot",
  "x": "endpoint",
  "y": "ms",
  "data": {
    "endpoint": ["/users","/users","/users","/users",
                 "/orders","/orders","/orders","/orders","/orders","/orders",
                 "/health","/health","/health","/health"],
    "ms":       [48, 50, 51, 52,
                 30, 60, 100, 250, 400, 80,
                 5, 6, 7, 8]
  }
}
EOF
maybe_pause

# ============================================================================
banner "9. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Non-numeric y column"
echo "x,v
a,foo
b,bar" | $TPLOT box - -x x -y v 2>&1 || true
maybe_pause

section "Missing column"
echo "endpoint,ms
/users,50" | $TPLOT box - -x endpont -y ms 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 5 (box plot)"
printf '\n\033[2mFor full Plans 1–5 walk-through:  bash scripts/demo.sh\033[0m\n'
printf '\033[2mPlan 4.5 (heatmap):  bash scripts/demo-plan4.5.sh\033[0m\n\n'
