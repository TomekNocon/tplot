#!/usr/bin/env bash
# Walk-through of Plan 12 — the `tplot sankey` flow diagram.
# Run from project root:    bash scripts/demo-plan12.sh
# With pauses:              bash scripts/demo-plan12.sh --pause

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
banner "1. The classic — user funnel"
# ============================================================================
section "5-layer funnel: landing → retention"
caption "Each stage thins out; biggest flow (landing → signup) is focal in burnt orange."
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000
verified,active,5500
verified,inactive,500
active,retain,4500
active,churn,1000" | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

section "Same funnel with --neutral"
caption "Story-pass disabled — every edge in focal color, no takeaway."
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000
verified,active,5500
verified,inactive,500
active,retain,4500
active,churn,1000" | $TPLOT sankey - --source src --target tgt --value flow --neutral
maybe_pause

section "Override: --focus 'signup → abandoned' (the leak)"
caption "Sometimes the SMALL flow is what matters most."
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000
verified,active,5500
verified,inactive,500
active,retain,4500
active,churn,1000" | $TPLOT sankey - --source src --target tgt --value flow \
  --focus "signup → abandoned"
maybe_pause

section "Custom takeaway"
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000
verified,active,5500
verified,inactive,500" | $TPLOT sankey - --source src --target tgt --value flow \
  --annotate "Bounce on landing is the biggest leak — fix the LP first"
maybe_pause

# ============================================================================
banner "2. Energy / resource flows"
# ============================================================================
section "Energy production → consumption"
echo "src,tgt,flow
solar,grid,500
wind,grid,300
gas,grid,200
grid,homes,400
grid,industry,400
grid,export,200
homes,heating,200
homes,appliances,150
homes,lighting,50" | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

section "Money flow — quarterly P&L"
echo "src,tgt,flow
revenue,cogs,400
revenue,gross_profit,600
gross_profit,opex,350
gross_profit,operating_income,250
operating_income,tax,75
operating_income,net_income,175
net_income,dividends,50
net_income,retained,125" | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

# ============================================================================
banner "3. Real shell-pipeline data"
# ============================================================================
section "Disk usage: top-level dirs → file-type breakdown"
caption "How is each of your top-level directories composed?"
{
  echo "src,tgt,flow"
  for d in crates docs scripts tests; do
    [[ -d "$d" ]] || continue
    rust_bytes=$(   find "$d" -name "*.rs"   -not -path "*/target/*" -exec du -k {} + 2>/dev/null | awk '{s+=$1} END{print s+0}')
    md_bytes=$(     find "$d" -name "*.md"   -exec du -k {} + 2>/dev/null | awk '{s+=$1} END{print s+0}')
    sh_bytes=$(     find "$d" -name "*.sh"   -exec du -k {} + 2>/dev/null | awk '{s+=$1} END{print s+0}')
    other_bytes=$(  find "$d" -type f -not -name "*.rs" -not -name "*.md" -not -name "*.sh" -not -path "*/target/*" -exec du -k {} + 2>/dev/null | awk '{s+=$1} END{print s+0}')
    [[ "$rust_bytes"  -gt 0 ]] && echo "$d,rust,$rust_bytes"
    [[ "$md_bytes"    -gt 0 ]] && echo "$d,markdown,$md_bytes"
    [[ "$sh_bytes"    -gt 0 ]] && echo "$d,shell,$sh_bytes"
    [[ "$other_bytes" -gt 0 ]] && echo "$d,other,$other_bytes"
  done
} | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

# ============================================================================
banner "4. Width adaptation"
# ============================================================================
section "Narrow (--width 60)"
echo "src,tgt,flow
a,b,100
a,c,80
b,d,60
b,e,40
c,d,30
c,e,50" | $TPLOT sankey - --source src --target tgt --value flow --width 60
maybe_pause

section "Wide (--width 130)"
echo "src,tgt,flow
a,b,100
a,c,80
b,d,60
b,e,40
c,d,30
c,e,50" | $TPLOT sankey - --source src --target tgt --value flow --width 130
maybe_pause

# ============================================================================
banner "5. Palettes"
# ============================================================================
section "Editorial (azure focal)"
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000" | $TPLOT sankey - --source src --target tgt --value flow --palette editorial
maybe_pause

section "Colorblind-safe"
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000" | $TPLOT sankey - --source src --target tgt --value flow --palette colorblind-safe
maybe_pause

# ============================================================================
banner "6. Graphics protocol output"
# ============================================================================
section "Force iTerm2 (first 80 chars of OSC)"
caption "Sankey diagrams render dramatically better as PNGs — smooth Bezier-style curves."
echo "src,tgt,flow
landing,signup,7000
landing,bounce,3000
signup,verified,6000
signup,abandoned,1000
verified,active,5500
verified,inactive,500" | $TPLOT sankey - --source src --target tgt --value flow \
  --graphics iterm2 | head -c 80 | cat -v
echo
maybe_pause

# ============================================================================
banner "7. JSON-driven mode"
# ============================================================================
section "Sankey from a JSON ChartSpec"
cat <<'EOF' | $TPLOT json
{
  "kind":   "sankey",
  "source": "src",
  "target": "tgt",
  "value":  "flow",
  "x":      "__node__",
  "y":      "__flow__",
  "data": {
    "src":  ["a","a","b","b","c"],
    "tgt":  ["b","c","d","e","e"],
    "flow": [60, 40, 30, 30, 30]
  }
}
EOF
maybe_pause

# ============================================================================
banner "8. Edge cases"
# ============================================================================
section "Two-node minimum (single edge)"
echo "src,tgt,flow
in,out,100" | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

section "Diamond pattern — multiple paths converging"
echo "src,tgt,flow
start,a,50
start,b,50
a,end,30
a,middle,20
b,end,40
b,middle,10
middle,end,30" | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

section "Wide fan-out (1 source → many)"
{ echo "src,tgt,flow"
  for c in red orange yellow green blue purple; do
    echo "rainbow,$c,$((20 + RANDOM % 30))"
  done
} | $TPLOT sankey - --source src --target tgt --value flow
maybe_pause

# ============================================================================
banner "9. Error cases (these are SUPPOSED to fail)"
# ============================================================================
section "Cycle in the graph (a → b → a)"
caption "Sankey requires a DAG. Cycles produce a clear error."
echo "src,tgt,flow
a,b,10
b,a,5" | $TPLOT sankey - --source src --target tgt --value flow 2>&1 || true
maybe_pause

section "Non-numeric flow column"
echo "src,tgt,flow
a,b,foo
c,d,bar" | $TPLOT sankey - --source src --target tgt --value flow 2>&1 || true
maybe_pause

section "Missing column"
echo "from,to,n
a,b,10" | $TPLOT sankey - --source src --target tgt --value n 2>&1 || true
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 12 (sankey — 14 chart types total, v1.5 chart-type list complete)"
printf '\n\033[2mNext: Plan 8 (distribution — Homebrew, GitHub releases, uv-installable Python wrapper).\033[0m\n\n'
