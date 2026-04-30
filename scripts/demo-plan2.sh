#!/usr/bin/env bash
# Focused walk-through of the Plan-2 features: vertical bars + histograms.
# Run from project root:    bash scripts/demo-plan2.sh
# With pauses:              bash scripts/demo-plan2.sh --pause

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

# Reusable synthetic latency dataset: bimodal — peak around 50ms with a tail.
gen_latency() {
  echo "ms"
  for _ in $(seq 1 80); do echo $((40 + RANDOM % 30)); done
  for _ in $(seq 1 12); do echo $((100 + RANDOM % 30)); done
  for _ in $(seq 1 3);  do echo $((180 + RANDOM % 80)); done
}

# ============================================================================
banner "1. Vertical bars — story mode by default"
# ============================================================================
section "Sales data, vertical orientation"
caption "EMEA still focal (193 vs 97 median = ~2× dominance). Y-axis shows max/half/0."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical
maybe_pause

section "Override: --focus NA (user picks the focal column)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical --focus NA
maybe_pause

section "Custom takeaway"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical \
  --annotate "EMEA carried H2 — APAC needs the next push"
maybe_pause

section "Neutral mode (story-pass disabled)"
caption "Every bar in focal color, no takeaway. Plotly-style equal weight."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical --neutral
maybe_pause

# ============================================================================
banner "2. Vertical bars on real data"
# ============================================================================
section "Monthly active users (typical time-series shape)"
echo "month,active_k
jan,12.4
feb,13.1
mar,14.8
apr,18.2
may,17.9
jun,21.4
jul,24.1
aug,20.0
sep,22.5
oct,28.4
nov,31.2
dec,29.8" | $TPLOT bar - -x month -y active_k --vertical
maybe_pause

section "Funnel (vertical)"
echo "stage,users
landing,12450
signup,3210
verify,2890
first_action,1980
day_7_retain,720" | $TPLOT bar - -x stage -y users --vertical
maybe_pause

section "Lines of Rust per crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    n=$(find "$d" -name "*.rs" -not -path "*/target/*" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    echo "$name,$n"
  done
} | $TPLOT bar - -x crate -y lines --vertical
maybe_pause

# ============================================================================
banner "3. Histograms — auto-binned by Sturges' rule"
# ============================================================================
section "Bimodal latency (peak ~50ms, secondary ~110ms, tail to ~250ms)"
caption "Auto bin count = ceil(log2(95)+1) = 8 bins."
gen_latency | $TPLOT hist - -x ms
maybe_pause

section "Same data, --bins 15 (finer granularity)"
gen_latency | $TPLOT hist - -x ms --bins 15
maybe_pause

section "Same data, --bins 4 (coarser)"
gen_latency | $TPLOT hist - -x ms --bins 4
maybe_pause

section "Uniform distribution → engine should NOT impose a focal bin"
caption "100 uniformly distributed values. No bin clearly dominates. Engine refuses to lie."
{ echo "n"; seq 1 100; } | $TPLOT hist - -x n
maybe_pause

# ============================================================================
banner "4. Histograms on real data"
# ============================================================================
section "File sizes (KB) of all .rs files"
{
  echo "kb"
  find crates -name "*.rs" -not -path "*/target/*" -exec du -k {} + | awk '{print $1}'
} | $TPLOT hist - -x kb
maybe_pause

section "Word counts per .rs file"
{
  echo "words"
  find crates -name "*.rs" -not -path "*/target/*" | xargs wc -w 2>/dev/null \
    | awk '$2 != "total" && NF==2 {print $1}'
} | $TPLOT hist - -x words --bins 10
maybe_pause

# ============================================================================
banner "5. Comparison — same data, three orientations"
# ============================================================================
section "Horizontal bars"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region
maybe_pause

section "Vertical bars"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical
maybe_pause

section "Histogram of the revenue values themselves"
{
  echo "rev"
  awk -F, 'NR>1 {print $3}' tests/fixtures/sales.csv
} | $TPLOT hist - -x rev
maybe_pause

# ============================================================================
banner "6. Width adaptation — both new chart types"
# ============================================================================
section "Vertical bars at --width 50"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical --width 50
maybe_pause

section "Histogram at --width 50"
gen_latency | $TPLOT hist - -x ms --width 50
maybe_pause

section "Histogram at --width 120"
gen_latency | $TPLOT hist - -x ms --width 120
maybe_pause

# ============================================================================
banner "7. Palettes on the new chart types"
# ============================================================================
section "Vertical bars — editorial palette (azure)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical --palette editorial
maybe_pause

section "Histogram — colorblind-safe palette (deuteranopia-safe blue)"
gen_latency | $TPLOT hist - -x ms --palette colorblind-safe
maybe_pause

# ============================================================================
banner "8. Error cases for the new chart types (these are SUPPOSED to fail)"
# ============================================================================
section "Histogram on a non-numeric column"
$TPLOT hist tests/fixtures/sales.csv -x region 2>&1 || true
maybe_pause

section "--bins with absurd value"
gen_latency | $TPLOT hist - -x ms --bins 1 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — that's everything Plan 2 added"
printf '\n\033[2mFor the full Plan 1 + 2 walk-through:  bash scripts/demo.sh\033[0m\n\n'
