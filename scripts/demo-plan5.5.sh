#!/usr/bin/env bash
# Focused walk-through of Plan 5.5 — the `tplot area` stacked area chart.
# Run from project root:    bash scripts/demo-plan5.5.sh
# With pauses:              bash scripts/demo-plan5.5.sh --pause

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
gen_revenue_by_region() {
  echo "month,rev,region"
  for m in 1 2 3 4 5 6 7 8 9 10 11 12; do
    # NA grows fastest (the eventual focal series)
    na=$((  10 + m * 6 + RANDOM % 8 ))
    emea=$(( 5  + m * 2 + RANDOM % 5 ))
    apac=$(( 3  + m + RANDOM % 4 ))
    latam=$(( 2 + m / 2 + RANDOM % 3 ))
    echo "$m,$na,NA"
    echo "$m,$emea,EMEA"
    echo "$m,$apac,APAC"
    echo "$m,$latam,LATAM"
  done
}

gen_signups_by_source() {
  echo "day,signups,source"
  for d in $(seq 1 30); do
    organic=$((  20 + RANDOM % 15 ))
    paid=$((     d * 2 + RANDOM % 10 ))   # ramps up over time
    referral=$(( 5 + RANDOM % 8 ))
    direct=$((   8 + RANDOM % 6 ))
    echo "$d,$organic,organic"
    echo "$d,$paid,paid"
    echo "$d,$referral,referral"
    echo "$d,$direct,direct"
  done
}

# ============================================================================
banner "1. The canonical use case — cumulative breakdown over time"
# ============================================================================
section "Revenue by region, 12 months — NA dominates"
caption "Default story-pass picks the largest contributor as focal; others recede to gray."
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled — every series in focal color, no takeaway."
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --neutral
maybe_pause

section "Override: --focus EMEA"
caption "Force EMEA as focal (it's the second-largest contributor)."
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --focus EMEA
maybe_pause

section "Custom takeaway via --annotate"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region \
  --annotate "NA's growth rate is outpacing all other regions combined"
maybe_pause

# ============================================================================
banner "2. Marketing funnel — signups by source"
# ============================================================================
section "30-day signups, paid acquisition ramping up"
caption "Watch the paid layer grow faster than the others — it should win as focal."
gen_signups_by_source | $TPLOT area - -x day -y signups --group source
maybe_pause

# ============================================================================
banner "3. Real data — git commits by month + author"
# ============================================================================
section "Commits per author per month from this repo"
caption "Useful for spotting who carried which sprint."
if git rev-parse --git-dir >/dev/null 2>&1; then
  {
    echo "month,commits,author"
    git log --all --format='%cd|%an' --date=format:'%Y%m' 2>/dev/null \
      | awk -F'|' '
          { key = $1 "\x1f" $2; counts[key]++ }
          END {
            for (k in counts) {
              n = split(k, p, "\x1f")
              print p[1] "," counts[k] "," p[2]
            }
          }
        ' \
      | sort
  } | $TPLOT area - -x month -y commits --group author
else
  echo "(not a git repo)"
fi
maybe_pause

section "Lines of code per crate over commits (synthesized — uses git rev-list)"
caption "Shows how each crate has grown across the project's history."
if git rev-parse --git-dir >/dev/null 2>&1; then
  {
    echo "commit,lines,crate"
    # Walk the last ~10 commits and count LOC per crate at each point.
    n=0
    for sha in $(git rev-list --reverse HEAD 2>/dev/null | tail -12); do
      n=$((n + 1))
      for d in crates/*/; do
        [[ -d "$d" ]] || continue
        crate=$(basename "$d")
        lines=$(git --no-pager grep -h --count='' "$sha" -- "$d*.rs" 2>/dev/null \
                | awk -F: '{ s += $NF } END { print s+0 }')
        echo "$n,$lines,$crate"
      done
    done
  } | $TPLOT area - -x commit -y lines --group crate 2>&1 || echo "(skipped — git grep unsupported)"
else
  echo "(not a git repo)"
fi
maybe_pause

# ============================================================================
banner "4. Many series (8+ groups)"
# ============================================================================
section "Synthetic 8-language portfolio over 20 quarters"
{
  echo "q,share,lang"
  langs=(rust python typescript go java cpp swift kotlin)
  for q in $(seq 1 20); do
    for l in "${langs[@]}"; do
      base=$(( RANDOM % 50 + 10 ))
      echo "$q,$base,$l"
    done
  done
} | $TPLOT area - -x q -y share --group lang
maybe_pause

# ============================================================================
banner "5. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --width 50
maybe_pause

section "Wide (--width 130)"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --width 130
maybe_pause

# ============================================================================
banner "6. Palettes"
# ============================================================================
section "Editorial (azure focal)"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --palette editorial
maybe_pause

section "Colorblind-safe"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region --palette colorblind-safe
maybe_pause

# ============================================================================
banner "7. Comparison — same data, line vs area"
# ============================================================================
section "View 1: Line chart (each series as a separate line)"
gen_revenue_by_region | $TPLOT line - -x month -y rev --group region
maybe_pause

section "View 2: Stacked area (each series fills its cumulative slot)"
gen_revenue_by_region | $TPLOT area - -x month -y rev --group region
maybe_pause

# ============================================================================
banner "8. Edge cases"
# ============================================================================
section "Single series — degenerate but valid"
caption "With one group, area becomes a filled line chart."
{ echo "t,v,g"
  for i in $(seq 1 12); do echo "$i,$((i * 3 + RANDOM % 5)),only"; done
} | $TPLOT area - -x t -y v --group g
maybe_pause

section "Series with zero values in some periods"
echo "t,v,g
1,10,A
1,0,B
2,15,A
2,5,B
3,12,A
3,15,B
4,8,A
4,25,B
5,5,A
5,40,B" | $TPLOT area - -x t -y v --group g
maybe_pause

section "All series equal (uniform totals — no clear focal)"
{
  echo "t,v,g"
  for t in $(seq 1 8); do
    for g in A B C D; do
      echo "$t,10,$g"
    done
  done
} | $TPLOT area - -x t -y v --group g
maybe_pause

# ============================================================================
banner "9. JSON-driven mode"
# ============================================================================
section "Stacked area from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "stacked_area",
  "x": "month",
  "y": "rev",
  "group": "region",
  "data": {
    "month":  [1,1,1, 2,2,2, 3,3,3, 4,4,4, 5,5,5, 6,6,6],
    "rev":    [10,5,3, 20,7,5, 35,9,6, 50,12,8, 70,15,10, 90,18,12],
    "region": ["NA","EMEA","APAC", "NA","EMEA","APAC", "NA","EMEA","APAC",
               "NA","EMEA","APAC", "NA","EMEA","APAC", "NA","EMEA","APAC"]
  }
}
EOF
maybe_pause

# ============================================================================
banner "10. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Missing --group flag"
gen_revenue_by_region | $TPLOT area - -x month -y rev 2>&1 || true
maybe_pause

section "Non-numeric x column"
echo "month,v,g
jan,10,A
feb,15,A
mar,20,A" | $TPLOT area - -x month -y v --group g 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 5.5 (stacked area) — v1 MVP COMPLETE"
printf '\n\033[2mFor the full v1 walk-through:  bash scripts/demo.sh\033[0m\n\n'
