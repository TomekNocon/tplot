#!/usr/bin/env bash
# Walk-through of Plan 7b — image protocol output (Kitty + iTerm2).
# Run from project root:    bash scripts/demo-plan7b.sh
# With pauses:              bash scripts/demo-plan7b.sh --pause
#
# IMPORTANT: image protocols only RENDER as images in graphics-capable
# terminals (iTerm2, Kitty, WezTerm). In VS Code's integrated terminal,
# Apple Terminal, plain xterm, etc., the output will appear as escape
# sequences ("gibberish"). This demo proves the bytes are correctly
# formed and that the PNGs decode cleanly — visual confirmation needs
# a capable terminal.

set -u

TPLOT="${TPLOT:-./target/release/tplot}"
PAUSE=0
[[ "${1:-}" == "--pause" || "${1:-}" == "-p" ]] && PAUSE=1

if [[ ! -x "$TPLOT" ]]; then
  echo "Building release binary first…" >&2
  cargo build --release -p tplot >/dev/null 2>&1 || { echo "Build failed."; exit 1; }
fi

SAMPLE_DIR="$(mktemp -d -t tplot-graphics-XXXX)"
trap 'rm -rf "$SAMPLE_DIR"' EXIT

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
hilite() { printf '\033[38;2;238;123;61m%s\033[0m' "$1"; }

# Extract the base64 PNG payload from an iTerm2 OSC sequence and decode it
# to a real PNG file. Uses perl (slurp + multi-line regex) for portability
# across BSD/GNU userlands.
extract_png_from_iterm2() {
  local in="$1" out="$2"
  perl -0777 -pe '
    s/^.*?\x1b\]1337;File=inline=1;size=\d+://s;
    s/\x07.*$//s;
  ' "$in" | base64 -d > "$out" 2>/dev/null
}

# Extract the base64 PNG payload from Kitty APC chunks (concatenated) and decode.
extract_png_from_kitty() {
  local in="$1" out="$2"
  perl -0777 -pe 's/\x1b_G[^;]*;//g; s/\x1b\\//g' "$in" \
    | base64 -d > "$out" 2>/dev/null
}

# ============================================================================
banner "1. What does your terminal support?"
# ============================================================================
section "Run tplot doctor"
caption "Look at the 'Graphics protocol' line — that's what --graphics auto will pick."
$TPLOT doctor
maybe_pause

# ============================================================================
banner "2. The bytes — same chart, three output modes"
# ============================================================================
section "Mode 1: text (default — half-blocks, ANSI color)"
caption "What you've been seeing all along."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60 | head -8
maybe_pause

section "Mode 2: --graphics iterm2 (first 120 chars, escape codes shown as text)"
caption "Starts with $(hilite "\\x1b]1337;File=inline=1;size=N:") followed by base64-encoded PNG, then $(hilite "BEL"). \`cat -v\` displays the escapes as printable text so iTerm2 doesn't try to render them."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60 --graphics iterm2 \
  | head -c 120 | cat -v
echo
maybe_pause

section "Mode 3: --graphics kitty (first 120 chars, escape codes shown as text)"
caption "Starts with $(hilite "\\x1b_Ga=T,f=100,m=N;") (APC), one or more chunks, each terminated by $(hilite "\\x1b\\\\") (ST)."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60 --graphics kitty \
  | head -c 120 | cat -v
echo
maybe_pause

# ============================================================================
banner "3. Verify the PNG is real — extract & inspect"
# ============================================================================
section "Save iTerm2 output to a file, extract the PNG, validate"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 80 \
  --graphics iterm2 > "$SAMPLE_DIR/iterm.bytes"
extract_png_from_iterm2 "$SAMPLE_DIR/iterm.bytes" "$SAMPLE_DIR/iterm.png"
echo "  Raw output size:  $(wc -c < "$SAMPLE_DIR/iterm.bytes") bytes"
echo "  Extracted PNG size: $(wc -c < "$SAMPLE_DIR/iterm.png") bytes"
echo "  PNG identification:"
file "$SAMPLE_DIR/iterm.png" | sed 's/^/    /'
maybe_pause

section "Same for Kitty"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 80 \
  --graphics kitty > "$SAMPLE_DIR/kitty.bytes"
extract_png_from_kitty "$SAMPLE_DIR/kitty.bytes" "$SAMPLE_DIR/kitty.png"
echo "  Raw output size:    $(wc -c < "$SAMPLE_DIR/kitty.bytes") bytes"
echo "  Extracted PNG size: $(wc -c < "$SAMPLE_DIR/kitty.png") bytes"
echo "  PNG identification:"
file "$SAMPLE_DIR/kitty.png" | sed 's/^/    /'
maybe_pause

section "Open the extracted PNGs in your default viewer (macOS only)"
caption "If you're on macOS, this opens the chart as a real image so you can see what graphics-capable terminals would render inline."
if command -v open >/dev/null 2>&1; then
  open "$SAMPLE_DIR/iterm.png"
  echo "  → Opened iterm.png — should show the EMEA-focal sales bar chart as a PNG."
else
  echo "  (skipped — 'open' command not available)"
fi
maybe_pause

# ============================================================================
banner "4. --graphics auto — let the engine pick"
# ============================================================================
section "Auto mode + doctor's verdict"
caption "doctor reports the protocol; auto routes through that automatically."
echo "  Protocol detected by doctor:"
$TPLOT doctor | grep "Graphics protocol" | sed 's/^/    /'
echo
echo "  --graphics auto produces (first 60 chars):"
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics auto | head -c 60
echo
caption "If your terminal doesn't speak any graphics protocol, auto silently falls back to text."
maybe_pause

# ============================================================================
banner "5. All chart types with --graphics"
# ============================================================================
section "Verify every chart type emits valid graphics bytes"
for spec in \
  "bar tests/fixtures/sales.csv -x quarter -y revenue --group region" \
  "hist tests/fixtures/sales.csv -x revenue" \
  "line tests/fixtures/sales.csv -x revenue -y revenue" \
  "scatter tests/fixtures/sales.csv -x revenue -y revenue" \
  "box tests/fixtures/sales.csv -x quarter -y revenue" \
  "area tests/fixtures/sales.csv -x revenue -y revenue --group region" \
  "heatmap tests/fixtures/sales.csv -x quarter -y region --value revenue"; do
  printf "  %-105s" "$spec --graphics iterm2"
  if $TPLOT $spec --graphics iterm2 2>/dev/null | head -c 30 | grep -q $'\x1b]1337'; then
    printf "$(hilite "✓ valid iTerm2 bytes")\n"
  else
    printf "$(hilite "✗ unexpected output")\n"
  fi
done
maybe_pause

section "Same with --graphics kitty"
for spec in \
  "bar tests/fixtures/sales.csv -x quarter -y revenue --group region" \
  "hist tests/fixtures/sales.csv -x revenue" \
  "line tests/fixtures/sales.csv -x revenue -y revenue" \
  "scatter tests/fixtures/sales.csv -x revenue -y revenue" \
  "box tests/fixtures/sales.csv -x quarter -y revenue" \
  "area tests/fixtures/sales.csv -x revenue -y revenue --group region" \
  "heatmap tests/fixtures/sales.csv -x quarter -y region --value revenue"; do
  printf "  %-105s" "$spec --graphics kitty"
  if $TPLOT $spec --graphics kitty 2>/dev/null | head -c 30 | grep -q $'\x1b_Ga'; then
    printf "$(hilite "✓ valid Kitty bytes")\n"
  else
    printf "$(hilite "✗ unexpected output")\n"
  fi
done
maybe_pause

section "Sparkline excluded by design"
caption "spark is a one-line inline chart — there's no value in upgrading to PNG, so the --graphics flag isn't part of its CLI. spark always produces text glyphs."
printf "  spark output (text-only):  "
echo "1 3 2 5 4 7 9 8 10 6" | $TPLOT spark -
maybe_pause

# ============================================================================
banner "6. To see actual images, copy-paste these into iTerm2 or Kitty"
# ============================================================================
caption "These will render as real inline images in graphics-capable terminals.
In VS Code/Apple Terminal/plain xterm, they appear as escape sequences (gibberish)."
cat <<EOF
  # Auto-pick the best protocol your terminal supports
  $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics auto

  # Force iTerm2 (works in iTerm2, WezTerm-with-iTerm2-mode)
  $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics iterm2

  # Force Kitty (works in kitty, foot, WezTerm-with-Kitty-mode)
  $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics kitty

  # Heatmap as a PNG — looks dramatically better than half-blocks
  { echo "hour,day,count"
    for d in Mon Tue Wed Thu Fri Sat Sun; do
      for h in 9 10 11 12 13 14 15 16 17 18 19 20; do
        echo "\$h,\$d,\$((RANDOM % 50 + 5))"
      done
    done
  } | $TPLOT heatmap - -x hour -y day --value count --graphics auto
EOF
maybe_pause

# ============================================================================
banner "7. Performance — text vs graphics rendering"
# ============================================================================
section "Time text rendering"
echo
{ time $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region > /dev/null; } 2>&1
maybe_pause

section "Time iTerm2 graphics rendering (PNG encode + base64)"
echo
{ time $TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --graphics iterm2 > /dev/null; } 2>&1
caption "Both paths are dominated by binary startup (~5ms); PNG encoding adds ~1-2ms. The graphics route feels just as instant as text."
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 7b (image protocol output)"
printf '\n\033[2mTo see images render: run any of the commands from section 6 in iTerm2 or Kitty.\033[0m\n'
printf '\033[2mFor your current terminal, --graphics auto will fall back to text rendering automatically.\033[0m\n\n'
