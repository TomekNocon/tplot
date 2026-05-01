#!/usr/bin/env bash
# Walk-through of Plan 6 — error-message polish, theme detection, narrow-terminal handling.
# Run from project root:    bash scripts/demo-plan6.sh
# With pauses:              bash scripts/demo-plan6.sh --pause

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

# Side-by-side annotation: "Before" was the v1 (pre-polish) error;
# "After" is what the binary produces today.
before() { printf '\033[2;31m  Before:\033[0m %s\n' "$1"; }
after()  { printf '\033[2;32m  After: \033[0m %s\n' "$1"; }
runerr() { "$@" 2>&1 || true; }

# ============================================================================
banner "1. Did-you-mean — column typos across chart types"
# ============================================================================
section "Bar chart with a typo'd group column"
caption "Run with --group regin (typo). Engine should suggest the closest match."
before "Error: no data rows"
echo
after "$(runerr $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group regin)"
maybe_pause

section "Histogram with a typo'd column"
caption "Run hist with -x m (should suggest 'ms')."
{ echo "ms"; for _ in $(seq 1 30); do echo $((40 + RANDOM % 30)); done } > /tmp/_p6_hist.csv
before "Error: no data rows"
echo
after "$(runerr $TPLOT hist /tmp/_p6_hist.csv -x m)"
maybe_pause

section "Line chart with typo'd group column"
{ echo "t,v,g"
  for i in 1 2 3 4 5; do
    echo "$i,$((i * 5)),A"
    echo "$i,$((i * 8)),B"
  done
} > /tmp/_p6_line.csv
before "Error: no data rows"
echo
after "$(runerr $TPLOT line /tmp/_p6_line.csv -x t -y v --group group)"
maybe_pause

section "Stacked area with multiple typos"
{ echo "month,rev,region"
  for m in 1 2 3 4; do
    echo "$m,$((m * 10)),NA"
    echo "$m,$((m * 5)),EMEA"
  done
} > /tmp/_p6_area.csv
before "Error: no data rows"
echo
after "$(runerr $TPLOT area /tmp/_p6_area.csv -x mont -y rev --group region)"
maybe_pause

# ============================================================================
banner "2. Type-error guidance — lists available numeric columns"
# ============================================================================
section "Bar chart with a string column where a numeric one is required"
caption "Run with -y quarter (string). Engine should list which columns ARE numeric."
before "Error: y column \`quarter\` must be numeric"
echo
after "$(runerr $TPLOT bar tests/fixtures/sales.csv -x revenue -y quarter --group region)"
maybe_pause

section "Histogram on a non-numeric column"
{ echo "name,score,age"
  echo "alice,98,30"
  echo "bob,72,25"
  echo "carol,85,28"
} > /tmp/_p6_people.csv
before "Error: column \`name\` must be numeric"
echo
after "$(runerr $TPLOT hist /tmp/_p6_people.csv -x name)"
maybe_pause

section "Box plot with a string column for y"
before "Error: y column \`name\` must be numeric"
echo
after "$(runerr $TPLOT box /tmp/_p6_people.csv -x age -y name)"
maybe_pause

# ============================================================================
banner "3. Sparkline — CSV without -y now lists alternatives"
# ============================================================================
section "Pipe a CSV through spark with no -y flag"
caption "Engine detects CSV-format input but doesn't know which column to plot — now lists the numeric ones."
before "input looks like CSV — pass \`-y <column>\` to pick the numeric column"
echo
after "$(cat /tmp/_p6_people.csv | runerr $TPLOT spark -)"
maybe_pause

# ============================================================================
banner "4. Narrow terminal handling"
# ============================================================================
section "Force --width 30 (below the 40-cell minimum)"
caption "Before: rendered a degraded chart silently. Now: clear width-recommendation error."
before "(silently produced an unreadable chart at 30 cols)"
echo
after "$(runerr $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 30)"
maybe_pause

section "Same idea on a histogram"
after "$(runerr $TPLOT hist /tmp/_p6_hist.csv -x ms --width 25)"
maybe_pause

section "Exactly 40 (the lower bound) still works"
caption "40 is the minimum-but-legible threshold."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 40
maybe_pause

# ============================================================================
banner "5. Theme detection — forced via \$COLORFGBG"
# ============================================================================
section "Default (no override) — uses the auto-detected theme"
caption "Whatever \$COLORFGBG is set to in your shell, the engine uses that."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60
maybe_pause

section "Forced DARK theme — \$COLORFGBG=15;0 (white fg on black bg)"
caption "Context gray rendered as #767676 — visible against dark backgrounds."
COLORFGBG="15;0" $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60
maybe_pause

section "Forced LIGHT theme — \$COLORFGBG=0;15 (black fg on white bg)"
caption "Context gray rendered as #4a4a4a — darker, so it's visible against light backgrounds."
COLORFGBG="0;15" $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60
maybe_pause

section "Same data, all four palettes, light theme"
caption "Each palette's focal color stays vivid; only the context gray adapts."
for p in signature editorial colorblind-safe; do
  printf '\033[2m  palette: %s\033[0m\n' "$p"
  COLORFGBG="0;15" $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60 --palette "$p"
  echo
done
maybe_pause

# ============================================================================
banner "6. Sanity — well-formed inputs still produce great output"
# ============================================================================
section "Story-mode default still works perfectly"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region
maybe_pause

section "Histogram still detects modal bin"
{ echo "ms"; for _ in $(seq 1 50); do echo $((40 + RANDOM % 30)); done; for _ in $(seq 1 5); do echo $((100 + RANDOM % 30)); done } \
  | $TPLOT hist - -x ms
maybe_pause

# Cleanup
rm -f /tmp/_p6_hist.csv /tmp/_p6_line.csv /tmp/_p6_area.csv /tmp/_p6_people.csv

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 6 (polish & error messages)"
printf '\n\033[2mPlan 6 made existing failure paths friendlier without changing the success paths.\033[0m\n\n'
