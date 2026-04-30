#!/usr/bin/env bash
# Focused walk-through of Plan 4.5 — the `tplot heatmap` 2D density chart.
# Run from project root:    bash scripts/demo-plan4.5.sh
# With pauses:              bash scripts/demo-plan4.5.sh --pause

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
gen_day_hour_activity() {
  echo "hour,day,count"
  for d in Mon Tue Wed Thu Fri Sat Sun; do
    for h in 9 10 11 12 13 14 15 16 17 18 19 20; do
      # Fake "active users" pattern: ramp up morning, peak around 14, dip evening
      base=$(( (h - 9) * 5 + (h > 14 ? (20 - h) * 4 : 0) ))
      [[ "$d" == "Sat" || "$d" == "Sun" ]] && base=$(( base / 3 ))   # weekend dip
      noise=$(( RANDOM % 8 ))
      echo "$h,$d,$(( base + noise ))"
    done
  done
}

gen_correlation_matrix() {
  echo "a,b,corr"
  feats=(price size weight color rating reviews stock)
  for x in "${feats[@]}"; do
    for y in "${feats[@]}"; do
      if [[ "$x" == "$y" ]]; then
        c="100"
      else
        # Random correlation between -80 and 80
        c=$(( RANDOM % 161 - 80 ))
      fi
      echo "$x,$y,$c"
    done
  done
}

# ============================================================================
banner "1. The canonical use case — day × hour activity"
# ============================================================================
section "Default — inferno ramp"
caption "Dark navy = quiet, orange/red = busy. Mon morning typically peaks here."
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count
maybe_pause

section "Same data, viridis ramp"
caption "Perceptually uniform purple → blue → green → yellow. Better for absolute readings."
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --ramp viridis
maybe_pause

section "Same data, coolwarm ramp"
caption "Diverging palette — best when data is centered around a midpoint."
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --ramp coolwarm
maybe_pause

# ============================================================================
banner "2. Correlation matrix"
# ============================================================================
section "7×7 feature correlation matrix"
caption "Values from -80 to +80; coolwarm shows positive vs negative cleanly."
gen_correlation_matrix | $TPLOT heatmap - -x a -y b --value corr --ramp coolwarm
maybe_pause

section "Same with viridis (note: doesn't show sign as well as coolwarm)"
gen_correlation_matrix | $TPLOT heatmap - -x a -y b --value corr --ramp viridis
maybe_pause

# ============================================================================
banner "3. Real data — commits per weekday × hour-of-day from git log"
# ============================================================================
section "Activity heatmap of this repo's commit history"
caption "What time / day of the week do you commit most? (uses git's own date formatter)"
if git rev-parse --git-dir >/dev/null 2>&1; then
  {
    echo "hour,day,count"
    # Git formats the date itself — works on BSD and GNU without external date(1)
    git log --all --format='%cd' --date=format:'%a %H' 2>/dev/null \
      | sort | uniq -c \
      | awk '{ print $3","$2","$1 }'
  } | $TPLOT heatmap - -x hour -y day --value count
else
  echo "(not a git repo — skipping)"
fi
maybe_pause

# ============================================================================
banner "4. Sparse data (many missing cells)"
# ============================================================================
section "Only a handful of cells populated"
caption "Empty cells render as background (no glyph). The rest follow the ramp."
echo "x,y,v
a,1,5
b,1,15
c,2,30
d,3,80
e,3,50
f,4,10" | $TPLOT heatmap - -x x -y y --value v
maybe_pause

# ============================================================================
banner "5. Different grid sizes"
# ============================================================================
section "Small (3×3)"
echo "h,d,c
9,Mon,5
10,Mon,12
11,Mon,8
9,Tue,3
10,Tue,25
11,Tue,7
9,Wed,11
10,Wed,18
11,Wed,9" | $TPLOT heatmap - -x h -y d --value c
maybe_pause

section "Medium (12×7) — same as section 1 but narrower terminal"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --width 60
maybe_pause

section "Wide (12×7) at --width 130"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --width 130
maybe_pause

section "Very wide (24×7) — full day cycle"
{
  echo "hour,day,count"
  for d in Mon Tue Wed Thu Fri Sat Sun; do
    for h in $(seq 0 23); do
      hh=$(printf "%02d" "$h")
      base=$(( h > 6 && h < 22 ? (h - 6) * 3 : 1 ))
      noise=$(( RANDOM % 5 ))
      echo "$hh,$d,$(( base + noise ))"
    done
  done
} | $TPLOT heatmap - -x hour -y day --value count
maybe_pause

# ============================================================================
banner "6. Story flags — annotate, --no-takeaway"
# ============================================================================
section "Custom takeaway via --annotate"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count \
  --annotate "Weekend mornings are dead — peak shifts to weekday afternoons"
maybe_pause

section "--no-takeaway (just the chart, nothing below)"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --no-takeaway
maybe_pause

# ============================================================================
banner "7. JSON-driven mode"
# ============================================================================
section "Heatmap from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "heatmap",
  "value": "count",
  "x": "hour",
  "y": "day",
  "data": {
    "hour":  ["9","10","11","9","10","11","9","10","11"],
    "day":   ["Mon","Mon","Mon","Tue","Tue","Tue","Wed","Wed","Wed"],
    "count": [5, 12, 8, 3, 25, 7, 11, 18, 9]
  }
}
EOF
maybe_pause

# ============================================================================
banner "8. Edge cases"
# ============================================================================
section "All cells equal value"
echo "x,y,v
a,1,5
b,1,5
a,2,5
b,2,5" | $TPLOT heatmap - -x x -y y --value v
maybe_pause

section "Single cell"
echo "x,y,v
a,1,42" | $TPLOT heatmap - -x x -y y --value v
maybe_pause

section "Negative values (only meaningful with coolwarm)"
echo "a,b,corr
x,x,80
x,y,-50
x,z,30
y,x,-50
y,y,90
y,z,-20
z,x,30
z,y,-20
z,z,100" | $TPLOT heatmap - -x a -y b --value corr --ramp coolwarm
maybe_pause

# ============================================================================
banner "9. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Non-numeric value column"
echo "x,y,v
a,1,foo
b,2,bar" | $TPLOT heatmap - -x x -y y --value v 2>&1 || true
maybe_pause

section "Unknown ramp name"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day --value count --ramp magma 2>&1 || true
maybe_pause

section "Missing --value flag"
gen_day_hour_activity | $TPLOT heatmap - -x hour -y day 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 4.5 (heatmap)"
printf '\n\033[2mFor full Plans 1–4.5 walk-through:  bash scripts/demo.sh\033[0m\n'
printf '\033[2mPlan 4 alone (sparkline):  bash scripts/demo-plan4.sh\033[0m\n\n'
