#!/usr/bin/env bash
# Walk-through of Plan 13 — the `tplot table` rich-style table chart.
# Run from project root:    bash scripts/demo-plan13.sh
# With pauses:              bash scripts/demo-plan13.sh --pause

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
banner "1. The basics — pretty-printing a CSV"
# ============================================================================
section "Plain table — no flags, just shows the data"
caption "Auto-detects column types (numbers right-aligned + thousands-separated)."
$TPLOT table tests/fixtures/sales.csv
maybe_pause

section "With --sort revenue (descending)"
$TPLOT table tests/fixtures/sales.csv --sort revenue
maybe_pause

section "With --top 5 (combine with sort)"
$TPLOT table tests/fixtures/sales.csv --sort revenue --top 5
maybe_pause

# ============================================================================
banner "2. The killer feature — inline value bars"
# ============================================================================
section "--bars revenue (eighths-precision proportional bars)"
caption "Each row gets a bar showing its value's share of the column max."
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 10
maybe_pause

section "Same data, --rounded corners"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 10 --rounded
maybe_pause

# ============================================================================
banner "3. Story flags — focal row, neutral, custom takeaway"
# ============================================================================
section "Default: focal = max (with trust-score gate)"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8
maybe_pause

section "--neutral (story-pass off — no row highlighted, no takeaway)"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8 --neutral
maybe_pause

section "--focus 'region=EMEA' (highlight specific row by column=value)"
caption "Useful when you want to draw attention to a specific cell."
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8 --focus 'region=EMEA'
maybe_pause

section "Custom takeaway"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8 \
  --annotate "Q4 EMEA was the standout — investigate what drove it"
maybe_pause

# ============================================================================
banner "4. Real shell-pipeline data"
# ============================================================================
section "Top 15 processes by memory (RSS)"
caption "ps → tplot table = a much better top-style display."
ps -A -o user,pid,rss,%cpu,comm 2>/dev/null \
  | awk 'NR==1 { print "user,pid,rss,cpu,comm"; next } { print $1","$2","$3","$4","$5 }' \
  | $TPLOT table - --bars rss --sort rss --top 15
maybe_pause

section "Disk usage by top-level project directory"
{
  echo "dir,kb"
  du -sk /Users/tomasznocon/Documents/TerminalPlot/{crates,docs,scripts,tests} 2>/dev/null \
    | awk '{ n=split($2,a,"/"); print a[n]","$1 }'
} | $TPLOT table - --bars kb --sort kb --rounded
maybe_pause

section "Lines of code per crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    n=$(find "$d" -name "*.rs" -not -path "*/target/*" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    echo "$name,$n"
  done
} | $TPLOT table - --bars lines --sort lines
maybe_pause

section "Recent commits per author (top 10)"
{
  echo "author,commits"
  git shortlog -sn --all 2>/dev/null \
    | head -10 \
    | awk '{ n=$1; $1=""; gsub(/^ +/,"",$0); print $0","n }'
} | $TPLOT table - --bars commits --sort commits
maybe_pause

# ============================================================================
banner "5. Type-aware formatting"
# ============================================================================
section "Booleans render as ✓/✗ (centered)"
caption "Detects strings like 'true', 'yes', '1', 'y', 't' as boolean-ish."
echo "asset,weight,active,risky
AAPL,30,true,false
MSFT,22,yes,no
GOOG,18,1,0
NVDA,15,true,true
META,9,false,yes
AMZN,6,t,f" | $TPLOT table - --bars weight --sort weight --rounded
maybe_pause

section "Large numbers get thousands separators"
echo "metric,value
revenue,1234567
cost,983450
profit,251117
expenses,99876
tax,50223" | $TPLOT table - --bars value --sort value
maybe_pause

section "Mixed columns (text + boolean + numeric)"
echo "name,active,score,timestamp
alice,true,98.5,2026-04-30
bob,false,72.0,2026-04-29
carol,yes,85.3,2026-04-30
david,no,41.7,2026-04-28
eve,t,90.1,2026-04-30" | $TPLOT table - --bars score --sort score
maybe_pause

# ============================================================================
banner "6. Width adaptation"
# ============================================================================
section "Narrow (--width 50) — long values truncated with …"
echo "name,description,score
short,a brief note,42
mid,this is a moderate length entry,58
long,this row has a particularly long description that will likely be truncated,71
verylong,extremely long entries demonstrate how the renderer handles overflow gracefully,89" \
  | $TPLOT table - --bars score --width 50
maybe_pause

section "Wide (--width 130)"
echo "name,description,score
short,a brief note,42
mid,this is a moderate length entry,58
long,this row has a particularly long description that will likely be truncated,71
verylong,extremely long entries demonstrate how the renderer handles overflow gracefully,89" \
  | $TPLOT table - --bars score --width 130
maybe_pause

# ============================================================================
banner "7. Palettes"
# ============================================================================
section "Editorial (azure focal)"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8 --palette editorial
maybe_pause

section "Colorblind-safe"
$TPLOT table tests/fixtures/sales.csv --bars revenue --sort revenue --top 8 --palette colorblind-safe
maybe_pause

# ============================================================================
banner "8. JSON-driven mode"
# ============================================================================
section "Table from a JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind":  "table",
  "bars":  "weight",
  "sort":  "weight",
  "top":   5,
  "x":     "__row__",
  "y":     "__col__",
  "data": {
    "asset":  ["AAPL","MSFT","GOOG","NVDA","META","AMZN","TSLA","NFLX"],
    "weight": [30, 22, 18, 15, 9, 6, 4, 2]
  }
}
EOF
maybe_pause

# ============================================================================
banner "9. Edge cases"
# ============================================================================
section "Single row"
echo "metric,value
revenue,42" | $TPLOT table - --bars value
maybe_pause

section "All-equal values (focal-trust gate refuses to highlight)"
caption "Trust score check: if no row clearly dominates, no focal coloring."
echo "team,score
alpha,50
beta,50
gamma,50
delta,50" | $TPLOT table - --bars score --sort score
maybe_pause

section "Many columns"
echo "id,name,team,role,active,joined,score,bonus,rank
1,alice,alpha,lead,true,2024,98,5000,1
2,bob,alpha,senior,true,2024,87,3000,3
3,carol,beta,lead,yes,2024,93,4500,2
4,dave,beta,senior,no,2025,75,2000,5
5,eve,gamma,lead,true,2025,82,3500,4" | $TPLOT table - --bars score --sort score --width 130
maybe_pause

# ============================================================================
banner "10. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Missing column via --sort"
echo "a,b
1,2
3,4" | $TPLOT table - --sort z 2>&1 || true
maybe_pause

section "Empty input"
echo "" | $TPLOT table - 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 13 (table — 15 chart types total)"
printf '\n\033[2mNext: Plan 8 (distribution — Homebrew, GitHub releases, uv-installable Python wrapper).\033[0m\n\n'
