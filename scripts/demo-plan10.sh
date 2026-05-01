#!/usr/bin/env bash
# Walk-through of Plan 10 — the `tplot tree` treemap chart.
# Run from project root:    bash scripts/demo-plan10.sh
# With pauses:              bash scripts/demo-plan10.sh --pause

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
banner "1. The basics — portfolio composition"
# ============================================================================
section "6-asset portfolio"
caption "Each rectangle's area is proportional to its weight. Largest holding (AAPL) is focal."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight
maybe_pause

section "Same data with --neutral"
caption "Story-pass disabled — every rectangle in focal color, no inline-label highlighting bias."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --neutral
maybe_pause

section "Override: --focus META (small holding)"
caption "Force a small rect as focal — it stands out even though it's tiny."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --focus META
maybe_pause

# ============================================================================
banner "2. Real shell-pipeline data"
# ============================================================================
section "Disk usage by top-level project directory (KB)"
{
  echo "dir,kb"
  du -sk crates docs scripts target tests 2>/dev/null | awk '{print $2","$1}'
} | $TPLOT tree - -x dir -y kb
maybe_pause

section "Lines of code per Rust crate"
{
  echo "crate,lines"
  for d in crates/*/; do
    name=$(basename "$d")
    n=$(find "$d" -name "*.rs" -not -path "*/target/*" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
    echo "$name,$n"
  done
} | $TPLOT tree - -x crate -y lines
maybe_pause

section "File counts by extension across the workspace"
{
  echo "ext,count"
  find . -type f -not -path "*/target/*" -not -path "*/.git/*" -not -path "*/.superpowers/*" 2>/dev/null \
    | awk -F. 'NF>1 { print $NF }' \
    | sort | uniq -c \
    | awk '{ if ($1 > 1) print $2","$1 }'
} | $TPLOT tree - -x ext -y count
maybe_pause

# ============================================================================
banner "3. Different palettes"
# ============================================================================
section "signature (default — burnt orange focal)"
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --palette signature
maybe_pause

section "editorial (azure focal)"
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --palette editorial
maybe_pause

section "colorblind-safe"
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --palette colorblind-safe
maybe_pause

# ============================================================================
banner "4. Edge cases"
# ============================================================================
section "Single category — degenerate but valid"
caption "One rectangle fills the entire plot area."
echo "kind,n
only,42" | $TPLOT tree - -x kind -y n
maybe_pause

section "Two equal values — split 50/50"
echo "x,v
left,50
right,50" | $TPLOT tree - -x x -y v
maybe_pause

section "Highly skewed — one item dominates"
caption "Squarify packs the small items into the remaining narrow area."
echo "x,v
giant,90
a,2
b,2
c,2
d,2
e,2" | $TPLOT tree - -x x -y v
maybe_pause

section "Many categories (12 items)"
{ echo "lang,share"
  echo "rust,28"
  echo "python,18"
  echo "typescript,14"
  echo "go,11"
  echo "java,8"
  echo "cpp,6"
  echo "swift,4"
  echo "ruby,3"
  echo "php,3"
  echo "kotlin,2"
  echo "scala,2"
  echo "haskell,1"
} | $TPLOT tree - -x lang -y share
maybe_pause

# ============================================================================
banner "5. Width adaptation"
# ============================================================================
section "Narrow (--width 50)"
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --width 50
maybe_pause

section "Wide (--width 130)"
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --width 130
maybe_pause

# ============================================================================
banner "6. Graphics protocol output"
# ============================================================================
section "Force iTerm2 inline-image (first 80 chars of OSC)"
caption "PNG output where the rectangles are crisp; in iTerm2 you'd see a real image."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --graphics iterm2 | head -c 80 | cat -v
echo
maybe_pause

# ============================================================================
banner "7. JSON-driven mode"
# ============================================================================
section "Treemap from JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind": "treemap",
  "x": "asset",
  "y": "weight",
  "data": {
    "asset":  ["AAPL","MSFT","GOOG","NVDA","META","AMZN"],
    "weight": [30, 22, 18, 15, 9, 6]
  }
}
EOF
maybe_pause

# ============================================================================
banner "8. The visual gotcha — focal-vs-context blending"
# ============================================================================
section "Story-mode default groups all non-focal items into one gray"
caption "Story-pass principle: only the focal pops. But on a treemap, the inner boundaries between non-focal rectangles vanish."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight
maybe_pause

section "--neutral makes every rectangle distinct (in focal color)"
caption "All rects in focal-orange, but they all share that color, so still no boundaries between non-focal rects."
echo "asset,weight
AAPL,30
MSFT,22
GOOG,18
NVDA,15
META,9
AMZN,6" | $TPLOT tree - -x asset -y weight --neutral
maybe_pause

caption "If the boundary loss bothers you — distinct context shades and rect borders are tracked as v1.5 polish."
maybe_pause

# ============================================================================
banner "9. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Non-numeric value column"
echo "asset,weight
AAPL,foo
MSFT,bar" | $TPLOT tree - -x asset -y weight 2>&1 || true
maybe_pause

section "Missing column"
echo "asset,weight
AAPL,30" | $TPLOT tree - -x assets -y weight 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 10 (treemap — 11 chart types total)"
printf '\n\033[2mNext on the v1.5 list: violin + ridgeline (KDE-based distribution shapes), then sankey.\033[0m\n\n'
