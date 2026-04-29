#!/usr/bin/env bash
# Walks through everything tplot can do in v1.
# Run from project root:    bash scripts/demo.sh
# Or with pauses:            bash scripts/demo.sh --pause

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

# ============================================================================
banner "1. Story-mode default — the SWD treatment in action"
# ============================================================================
section "Default — engine auto-detects EMEA as focal"
caption "EMEA dominates (~2× the median); engine paints it burnt-orange and grays the rest."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled. Equal weight, no takeaway. Plotly-style."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --neutral
maybe_pause

section "Override: --focus NA"
caption "User picks the focal point manually."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --focus NA
maybe_pause

section "Custom takeaway via --annotate"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region \
  --annotate "Q4 saw EMEA pull ahead — close the gap before Q1"
maybe_pause

# ============================================================================
banner "2. Palettes"
# ============================================================================
section "signature (default — burnt orange)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --palette signature
maybe_pause

section "editorial (azure blue)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --palette editorial
maybe_pause

section "colorblind-safe (deuteranopia-safe blue)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --palette colorblind-safe
maybe_pause

# ============================================================================
banner "3. The trust-score gate — the differentiator vs every other chart lib"
# ============================================================================
section "Boring data (everything ±5% of median) → engine refuses to fake a story"
caption "No series clearly dominates. Engine admits it instead of imposing a focal color."
echo "metric,value
api_p50,48
api_p95,51
api_p99,49
db_p50,50
db_p95,52
cache_p50,47" | $TPLOT bar - -x metric -y value
maybe_pause

section "One thing clearly stands out → engine highlights it"
caption "/api/payments has 35× more errors than the median. Engine flags it."
echo "endpoint,errors_today
/api/users,3
/api/orders,5
/api/payments,247
/api/search,7
/api/health,2" | $TPLOT bar - -x endpoint -y errors_today
maybe_pause

# ============================================================================
banner "4. Real shell-pipeline data"
# ============================================================================
section "Lines of Rust per source file in this project"
{
  echo "file,lines"
  find crates -name "*.rs" -not -path "*/target/*" | xargs wc -l 2>/dev/null \
    | awk '$2 != "total" && NF==2 {
        n=split($2,a,"/")
        # Disambiguate: use parent_dir/filename (e.g. layout/bar.rs vs rasterize/bar.rs)
        label = (n>=2 ? a[n-1]"/"a[n] : a[n])
        print label","$1
      }' \
    | sort -t, -k2 -rn | head -10
} | $TPLOT bar - -x file -y lines
maybe_pause

section "Commits per author in this repo"
{
  echo "author,commits"
  git shortlog -sn --all 2>/dev/null \
    | awk '{ n=$1; $1=""; gsub(/^ +/,"",$0); print $0","n }'
} | $TPLOT bar - -x author -y commits
maybe_pause

section "Rust file count per crate"
{
  echo "crate,files"
  for d in crates/*/; do
    name=$(basename "$d")
    n=$(find "$d" -name "*.rs" -not -path "*/target/*" | wc -l | tr -d ' ')
    echo "$name,$n"
  done
} | $TPLOT bar - -x crate -y files
maybe_pause

# ============================================================================
banner "5. Synthetic 'use it inline in a script' patterns"
# ============================================================================
section "Funnel — landing → activation"
echo "stage,users
landing,12450
signup,3210
verify,2890
first_action,1980
day_7_retain,720" | $TPLOT bar - -x stage -y users
maybe_pause

section "Monthly active users (still horizontal — vertical bars come in Plan 2)"
echo "month,active_k
jan,12.4
feb,13.1
mar,14.8
apr,18.2
may,17.9
jun,21.4" | $TPLOT bar - -x month -y active_k
maybe_pause

# ============================================================================
banner "6. Width adaptation"
# ============================================================================
section "Auto-detect (uses your terminal width)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region
maybe_pause

section "Forced narrow (--width 40)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 40
maybe_pause

section "Forced wide (--width 140)"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 140
maybe_pause

# ============================================================================
banner "7. Vertical bars (Plan 2 — uses ▁▂▃▄▅▆▇█ for 8-step sub-cell smoothness)"
# ============================================================================
section "Same data, vertical orientation"
caption "EMEA still focal; column heights smoothed via lower-block elements."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --vertical
maybe_pause

section "Time-series style (months)"
echo "month,active_k
jan,12.4
feb,13.1
mar,14.8
apr,18.2
may,17.9
jun,21.4
jul,24.1
aug,20.0" | $TPLOT bar - -x month -y active_k --vertical
maybe_pause

# ============================================================================
banner "8. Histograms (Plan 2 — auto-binning + modal-bin focal)"
# ============================================================================
section "Bimodal latency data (peak around 50ms, secondary around 110ms)"
{ echo "ms"; for i in $(seq 1 80); do echo $((40 + RANDOM % 30)); done; for i in $(seq 1 12); do echo $((100 + RANDOM % 30)); done } | $TPLOT hist - -x ms
maybe_pause

section "Same data with --bins 12 (override Sturges' rule)"
{ echo "ms"; for i in $(seq 1 80); do echo $((40 + RANDOM % 30)); done; for i in $(seq 1 12); do echo $((100 + RANDOM % 30)); done } | $TPLOT hist - -x ms --bins 12
maybe_pause

section "Synthetic distribution — 1..100 uniform"
{ echo "n"; seq 1 100; } | $TPLOT hist - -x n
maybe_pause

# ============================================================================
banner "9. JSON-driven mode (the engine's true API)"
# ============================================================================
section "tplot json reads a ChartSpec from stdin"
caption "This is what Python/Node/anything-else uses to drive tplot as a library."
cat tests/fixtures/sales.json | $TPLOT json
maybe_pause

# ============================================================================
banner "10. Error handling — these three are SUPPOSED to fail (showing how errors look)"
# ============================================================================
section "Typo in column name (engine should suggest the closest valid column)"
caption "Running with -x quartrer (typo). Expect a 'did-you-mean' style message."
$TPLOT bar tests/fixtures/sales.csv -x quartrer -y revenue --group region 2>&1 || true
maybe_pause

section "Wrong column type (string column passed where numbers are needed)"
caption "Running with -y quarter (the labels column). Expect a clear type error."
$TPLOT bar tests/fixtures/sales.csv -x revenue -y quarter --group region 2>&1 || true
maybe_pause

section "Missing input file"
caption "Running on a path that doesn't exist. Expect a filesystem error."
$TPLOT bar nonexistent.csv -x foo -y bar 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — that's everything tplot v0.1 can do"
printf '\n\033[2mTo install globally:  cargo install --path crates/tplot\033[0m\n\n'
