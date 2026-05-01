#!/usr/bin/env bash
# Walk-through of Plan 11 — the `tplot violin` chart (KDE-based distribution shapes).
# Run from project root:    bash scripts/demo-plan11.sh
# With pauses:              bash scripts/demo-plan11.sh --pause

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
  # /health: tight cluster (low spread)
  for _ in $(seq 1 25); do echo "/health,$((4 + RANDOM % 5))"; done
  # /users: medium spread
  for _ in $(seq 1 25); do echo "/users,$((45 + RANDOM % 20))"; done
  # /search: wide spread with a tail
  for _ in $(seq 1 20); do echo "/search,$((60 + RANDOM % 50))"; done
  for _ in $(seq 1 5);  do echo "/search,$((150 + RANDOM % 80))"; done
  # /checkout: bimodal (fast path + slow path)
  for _ in $(seq 1 15); do echo "/checkout,$((80 + RANDOM % 30))"; done
  for _ in $(seq 1 10); do echo "/checkout,$((250 + RANDOM % 50))"; done
}

gen_team_scores() {
  echo "team,score"
  for _ in $(seq 1 30); do echo "alpha,$((75 + RANDOM % 12))"; done
  for _ in $(seq 1 30); do echo "beta,$((40 + RANDOM % 50))"; done
  for _ in $(seq 1 30); do echo "gamma,$((65 + RANDOM % 25))"; done
}

# ============================================================================
banner "1. The basics — distribution shape per category"
# ============================================================================
section "Endpoint latencies — 4 endpoints with very different distributions"
caption "Each violin shows the shape of one endpoint's latency distribution. The widest violin = widest IQR = focal."
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled — every violin in focal color, no auto-takeaway."
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms --neutral
maybe_pause

section "Override: --focus /health (tight cluster)"
caption "Force a tight, low-spread group as focal. Story still names it correctly."
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms --focus /health
maybe_pause

section "Custom takeaway"
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms \
  --annotate "/checkout's bimodal shape suggests a slow code path on some requests"
maybe_pause

# ============================================================================
banner "2. Team performance — clear vs noisy"
# ============================================================================
section "Three teams' scores"
caption "alpha is consistently high; beta has a wide range (good and bad days); gamma sits in between."
gen_team_scores | $TPLOT violin - -x team -y score
maybe_pause

# ============================================================================
banner "3. Distribution shapes — see KDE in action"
# ============================================================================
section "Normal-shaped distribution (single peak around mean)"
{ echo "kind,v"
  for _ in $(seq 1 80); do
    awk 'BEGIN { srand(); printf "normal,%d\n", 50 + 10*((rand()-0.5)+(rand()-0.5)+(rand()-0.5)) }'
  done
} | $TPLOT violin - -x kind -y v
maybe_pause

section "Bimodal — two peaks (fast path + slow path scenario)"
{ echo "kind,v"
  for _ in $(seq 1 40); do echo "bimodal,$((15 + RANDOM % 12))"; done
  for _ in $(seq 1 40); do echo "bimodal,$((75 + RANDOM % 12))"; done
} | $TPLOT violin - -x kind -y v
maybe_pause

section "Right-skewed (long tail) — typical of latency data"
{ echo "kind,v"
  for _ in $(seq 1 60); do echo "skewed,$((30 + RANDOM % 20))"; done
  for _ in $(seq 1 8);  do echo "skewed,$((100 + RANDOM % 50))"; done
  for _ in $(seq 1 2);  do echo "skewed,$((200 + RANDOM % 100))"; done
} | $TPLOT violin - -x kind -y v
maybe_pause

section "Uniform (flat) distribution"
{ echo "kind,v"; for _ in $(seq 1 100); do echo "uniform,$((10 + RANDOM % 90))"; done } \
  | $TPLOT violin - -x kind -y v
maybe_pause

# ============================================================================
banner "4. Comparison — box plot vs violin"
# ============================================================================
section "Box plot (5-number summary)"
caption "Compact, statistical. Median, quartiles, whiskers."
gen_endpoint_latencies | $TPLOT box - -x endpoint -y ms
maybe_pause

section "Violin (full distribution shape)"
caption "Same data — but you can SEE bimodality, skew, multimodality."
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms
maybe_pause

# ============================================================================
banner "5. Real shell-pipeline data"
# ============================================================================
section "File size distribution per crate (KB)"
{
  echo "crate,kb"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec du -k {} + 2>/dev/null \
      | awk -v c="$name" '{ print c","$1 }'
  done
} | $TPLOT violin - -x crate -y kb
maybe_pause

section "Lines-per-file distribution per crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    find "$d" -name "*.rs" -not -path "*/target/*" -exec wc -l {} + 2>/dev/null \
      | awk -v c="$name" '$2 != "total" && NF==2 { print c","$1 }'
  done
} | $TPLOT violin - -x crate -y lines
maybe_pause

# ============================================================================
banner "6. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
gen_team_scores | $TPLOT violin - -x team -y score --width 50
maybe_pause

section "Wide (--width 130)"
gen_team_scores | $TPLOT violin - -x team -y score --width 130
maybe_pause

# ============================================================================
banner "7. Palettes"
# ============================================================================
section "Editorial (azure focal)"
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms --palette editorial
maybe_pause

section "Colorblind-safe"
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms --palette colorblind-safe
maybe_pause

# ============================================================================
banner "8. Graphics protocol output"
# ============================================================================
section "Force iTerm2 (first 80 chars of OSC)"
gen_endpoint_latencies | $TPLOT violin - -x endpoint -y ms --graphics iterm2 \
  | head -c 80 | cat -v
echo
maybe_pause

# ============================================================================
banner "9. JSON-driven mode"
# ============================================================================
section "Violin from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "violin",
  "x": "endpoint",
  "y": "ms",
  "data": {
    "endpoint": ["/short","/short","/short","/short","/short",
                 "/long","/long","/long","/long","/long","/long","/long","/long"],
    "ms":       [48,50,51,52,53,
                 10,30,80,150,300,250,60,90]
  }
}
EOF
maybe_pause

# ============================================================================
banner "10. Edge cases"
# ============================================================================
section "Single group"
caption "One violin showing the full shape of a single dataset."
{ echo "kind,v"; for _ in $(seq 1 50); do echo "all,$((40 + RANDOM % 30))"; done } \
  | $TPLOT violin - -x kind -y v
maybe_pause

section "Single value per group (degenerate — KDE produces a sharp peak)"
echo "x,v
a,50
b,75" | $TPLOT violin - -x x -y v
maybe_pause

section "Many groups (8 categories)"
{ echo "g,v"
  for g in a b c d e f g h; do
    spread=$((RANDOM % 40 + 10))
    center=$((RANDOM % 80 + 30))
    for _ in $(seq 1 30); do echo "$g,$((center + RANDOM % spread))"; done
  done
} | $TPLOT violin - -x g -y v
maybe_pause

# ============================================================================
banner "11. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Non-numeric y column"
echo "x,v
a,foo
b,bar" | $TPLOT violin - -x x -y v 2>&1 || true
maybe_pause

section "Missing column"
echo "x,v
a,42" | $TPLOT violin - -x category -y v 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 11 (violin — 12 chart types total)"
printf '\n\033[2mNext on the v1.5 list: ridgeline (stacked KDEs over a categorical axis), then sankey.\033[0m\n\n'
