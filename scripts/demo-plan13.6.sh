#!/usr/bin/env bash
# Walk-through of Plan 13.6 — the `tplot summary` single-line chart.
# Designed specifically for inline use in Claude Code where multi-line output
# gets collapsed behind Ctrl+O. Every command in this demo emits ONE line.
# Run from project root:    bash scripts/demo-plan13.6.sh
# With pauses:              bash scripts/demo-plan13.6.sh --pause

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
banner "1. Sequence mode — y column treated as an ordered series"
# ============================================================================
section "Plain sequence — sparkline + median + max + index of max"
caption "The single-line format puts the entire story on one row: shape, headline stats, peak."
printf 'value\n10\n20\n35\n50\n80\n130\n210\n' | $TPLOT summary - -y value
maybe_pause

section "Trendless data — focal still picks the max but the takeaway is muted"
printf 'value\n50\n48\n51\n49\n50\n52\n50\n49\n51\n' | $TPLOT summary - -y value
maybe_pause

section "U-shaped data — early dip then rise"
printf 'temp\n22\n18\n14\n11\n10\n12\n16\n21\n26\n30\n' | $TPLOT summary - -y temp
maybe_pause

section "Random walk (10 values)"
{ echo "v"; for i in $(seq 1 10); do echo $((RANDOM % 100)); done; } | $TPLOT summary - -y v
maybe_pause

# ============================================================================
banner "2. Categorical mode — focal pops, others gray, ranked left to right"
# ============================================================================
section "5 categories — focal lights up if max ≥ 1.5× median"
caption "Notice 'rust' is in burnt orange because 1832 / 541 ≈ 3.4× — clear focal winner."
printf 'lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\n' \
  | $TPLOT summary - -x lang -y lines
maybe_pause

section "Q4 sales by region (revenue)"
caption "Real fixture data — one line tells you the story."
awk -F, 'NR==1 || /^Q4,/' tests/fixtures/sales.csv | $TPLOT summary - -x region -y revenue
maybe_pause

section "--top 3 (keep only the top three categories)"
printf 'lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\nmakefile,42\n' \
  | $TPLOT summary - -x lang -y lines --top 3
maybe_pause

# ============================================================================
banner "3. Story flags — focus, neutral, annotate, no-takeaway"
# ============================================================================
section "Default — auto-detected focal + auto takeaway"
printf 'team,points\nalpha,142\nbeta,98\ngamma,71\ndelta,45\nepsilon,28\n' \
  | $TPLOT summary - -x team -y points
maybe_pause

section "--neutral (no focal coloring, no takeaway)"
printf 'team,points\nalpha,142\nbeta,98\ngamma,71\ndelta,45\nepsilon,28\n' \
  | $TPLOT summary - -x team -y points --neutral
maybe_pause

section "--focus 'team=delta' (highlight a specific category, not the max)"
caption "Useful when you want to draw attention to a non-leading row."
printf 'team,points\nalpha,142\nbeta,98\ngamma,71\ndelta,45\nepsilon,28\n' \
  | $TPLOT summary - -x team -y points --focus 'team=delta'
maybe_pause

section "--annotate (replace the auto takeaway with custom copy)"
printf 'team,points\nalpha,142\nbeta,98\ngamma,71\ndelta,45\nepsilon,28\n' \
  | $TPLOT summary - -x team -y points --annotate "alpha is pulling away — investigate"
maybe_pause

section "--no-takeaway (keep focal styling, drop the trailing copy)"
printf 'team,points\nalpha,142\nbeta,98\ngamma,71\ndelta,45\nepsilon,28\n' \
  | $TPLOT summary - -x team -y points --no-takeaway
maybe_pause

# ============================================================================
banner "4. Real shell-pipeline data"
# ============================================================================
section "Top 5 processes by RSS"
caption "ps → tplot summary = a one-line live snapshot of memory pressure."
{
  echo "rss,comm"
  ps -A -o rss,comm 2>/dev/null \
    | awk 'NR>1 { n=split($2,a,"/"); print $1","a[n] }' \
    | sort -t, -k1 -nr \
    | head -5
} | $TPLOT summary - -x comm -y rss
maybe_pause

section "Lines of Rust per crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    n=$(find "$d" -name "*.rs" -not -path "*/target/*" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    echo "$name,$n"
  done
} | $TPLOT summary - -x crate -y lines
maybe_pause

section "Commits per author (top 5)"
{
  echo "author,commits"
  git shortlog -sn --all 2>/dev/null \
    | head -5 \
    | awk '{ n=$1; $1=""; gsub(/^ +/,"",$0); print $0","n }'
} | $TPLOT summary - -x author -y commits
maybe_pause

section "Disk usage by top-level dir"
{
  echo "dir,kb"
  du -sk crates docs scripts tests 2>/dev/null \
    | awk '{ n=split($2,a,"/"); print a[n]","$1 }'
} | $TPLOT summary - -x dir -y kb
maybe_pause

section "Sequence mode — last 12 commit message lengths"
caption "Everything is a sequence if you squint hard enough."
{
  echo "len"
  git log --pretty=format:'%s' -n 12 2>/dev/null | awk '{ print length($0) }'
} | $TPLOT summary - -y len
maybe_pause

# ============================================================================
banner "5. Palettes"
# ============================================================================
section "Default — signature (burnt orange focal)"
printf 'lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\n' \
  | $TPLOT summary - -x lang -y lines
maybe_pause

section "--palette editorial (azure focal)"
printf 'lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\n' \
  | $TPLOT summary - -x lang -y lines --palette editorial
maybe_pause

section "--palette colorblind-safe"
printf 'lang,lines\nrust,1832\nmarkdown,892\nshell,541\nyaml,128\ntoml,89\n' \
  | $TPLOT summary - -x lang -y lines --palette colorblind-safe
maybe_pause

# ============================================================================
banner "6. JSON-driven mode"
# ============================================================================
section "Sequence summary from a JSON ChartSpec"
caption "Use the sentinel x=\"__sequence__\" to opt into sequence mode."
cat <<'EOF' | $TPLOT json
{
  "kind": "summary",
  "x":    "__sequence__",
  "y":    "value",
  "data": {
    "value": [10, 20, 35, 50, 80, 130, 210]
  }
}
EOF
maybe_pause

section "Categorical summary with --top via JSON"
cat <<'EOF' | $TPLOT json
{
  "kind": "summary",
  "x":    "lang",
  "y":    "lines",
  "top":  3,
  "data": {
    "lang":  ["rust","markdown","shell","yaml","toml"],
    "lines": [1832, 892, 541, 128, 89]
  }
}
EOF
maybe_pause

# ============================================================================
banner "7. Edge cases"
# ============================================================================
section "Single-value sequence"
printf 'v\n42\n' | $TPLOT summary - -y v
maybe_pause

section "All-equal categorical (focal-trust gate refuses to highlight)"
caption "When no category clearly dominates, no focal coloring + muted takeaway."
printf 'team,score\nalpha,50\nbeta,50\ngamma,50\ndelta,50\n' \
  | $TPLOT summary - -x team -y score
maybe_pause

section "Two-category minimum"
printf 'team,score\nalpha,80\nbeta,20\n' | $TPLOT summary - -x team -y score
maybe_pause

section "Many categories, default --top 5 trims the tail"
{
  echo "city,pop"
  for c in tokyo delhi shanghai sao_paulo mumbai cairo dhaka beijing osaka karachi; do
    echo "$c,$((10 + RANDOM % 40))"
  done
} | $TPLOT summary - -x city -y pop
maybe_pause

# ============================================================================
banner "8. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Missing -y column"
printf 'a,b\n1,2\n3,4\n' | $TPLOT summary - -y nope 2>&1 || true
maybe_pause

section "Non-numeric -y column"
printf 'a,b\nfoo,bar\nbaz,qux\n' | $TPLOT summary - -y a 2>&1 || true
maybe_pause

section "Empty input"
printf '' | $TPLOT summary - -y v 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 13.6 (summary — 16 chart types total)"
printf '\n\033[2mWhy this chart matters: every command in this demo emitted exactly ONE line.\033[0m\n'
printf '\033[2mClaude Code renders single-line Bash output inline (no Ctrl+O collapse), so\033[0m\n'
printf '\033[2m`tplot summary` is the right default for in-conversation visual output.\033[0m\n\n'
printf '\033[2mNext: Plan 8 (distribution — Homebrew, GitHub releases, uv-installable Python wrapper).\033[0m\n\n'
