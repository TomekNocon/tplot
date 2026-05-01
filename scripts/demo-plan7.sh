#!/usr/bin/env bash
# Walk-through of Plan 7 — capability probing + tplot doctor.
# Run from project root:    bash scripts/demo-plan7.sh
# With pauses:              bash scripts/demo-plan7.sh --pause

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

# Run doctor with isolated environment overrides. `env -i` strips inherited
# vars; we re-add only what we want, then PATH so the binary runs.
run_doctor() {
  env -i PATH="$PATH" HOME="$HOME" "$@" "$TPLOT" doctor
}

# ============================================================================
banner "1. The basics — doctor on your live terminal"
# ============================================================================
section "Default report (uses your real environment + live OSC probes)"
caption "This is what you'd run on any new machine to see how tplot views the terminal."
$TPLOT doctor
maybe_pause

# ============================================================================
banner "2. Forcing different environments — see how the report adapts"
# ============================================================================
section "Forced TERM=xterm-kitty (env-detected Kitty, no live probe)"
caption "Without an actual Kitty terminal under us, the OSC probe returns false; env still classifies as Kitty."
TERM=xterm-kitty COLORTERM=truecolor TERM_PROGRAM= COLORFGBG= run_doctor TERM=xterm-kitty COLORTERM=truecolor 2>&1 | head -30
maybe_pause

section "Forced TERM_PROGRAM=iTerm.app"
caption "iTerm2 inline-image protocol detected from \$TERM_PROGRAM."
TERM=xterm-256color COLORTERM=truecolor TERM_PROGRAM=iTerm.app run_doctor TERM=xterm-256color COLORTERM=truecolor TERM_PROGRAM=iTerm.app 2>&1 | head -30
maybe_pause

section "Forced TERM_PROGRAM=Apple_Terminal (light theme by default)"
caption "Apple Terminal classifies as Light theme — context gray adapts to dark gray (#4a4a4a)."
run_doctor TERM=xterm-256color COLORTERM=truecolor TERM_PROGRAM=Apple_Terminal 2>&1 | head -30
maybe_pause

section "Forced COLORFGBG=0;15 (light background)"
caption "Theme detection classifies this as a light terminal."
run_doctor TERM=xterm-256color COLORTERM=truecolor COLORFGBG="0;15" 2>&1 | head -30
maybe_pause

section "Forced COLORFGBG=15;0 (dark background, default for most dev terminals)"
run_doctor TERM=xterm-256color COLORTERM=truecolor COLORFGBG="15;0" 2>&1 | head -30
maybe_pause

# ============================================================================
banner "3. Color-depth degradation — what happens on lesser terminals"
# ============================================================================
section "TERM=xterm-256color (no truecolor)"
caption "Falls back to ANSI 256-color, the engine adapts the palette accordingly."
run_doctor TERM=xterm-256color 2>&1 | head -30
maybe_pause

section "TERM=dumb (no color at all)"
caption "Engine should classify as monochrome and warn about palette degradation."
run_doctor TERM=dumb 2>&1 | head -30
maybe_pause

section "TERM=xterm (no 256, no truecolor)"
caption "Plain ANSI 16-color path."
run_doctor TERM=xterm 2>&1 | head -30
maybe_pause

# ============================================================================
banner "4. Non-TTY mode — probes skipped, env-only report"
# ============================================================================
section "Pipe through cat → no TTY → probes skipped, no raw-mode flicker"
caption "Notice that \"Graphics protocol\" reflects ONLY env detection (not OSC)."
$TPLOT doctor | cat
maybe_pause

section "Capture to a file — same env-only behavior"
$TPLOT doctor > /tmp/tplot-doctor.txt && cat /tmp/tplot-doctor.txt && rm -f /tmp/tplot-doctor.txt
maybe_pause

# ============================================================================
banner "5. Side-by-side — doctor verdict vs actual rendering"
# ============================================================================
section "1. Run doctor"
$TPLOT doctor | head -20
maybe_pause

section "2. Run a chart with the same caps"
caption "The colors / glyphs the doctor described above are exactly what you should see in this chart."
$TPLOT bar tests/fixtures/sales.csv -x quarter -y revenue --group region --width 60
maybe_pause

# ============================================================================
banner "6. Edge cases"
# ============================================================================
section "Empty environment — engine uses Capabilities::conservative()"
caption "No \$TERM, no \$COLORTERM, nothing — engine assumes ANSI 256, half-blocks, dark theme."
env -i PATH="$PATH" HOME="$HOME" "$TPLOT" doctor 2>&1 | head -30
maybe_pause

section "Doctor inside SSH (simulated via TERM=screen)"
run_doctor TERM=screen-256color COLORTERM=truecolor SSH_CLIENT="192.0.2.1 22 22" 2>&1 | head -30
maybe_pause

# ----------------------------------------------------------------- closing ---
banner "Done — Plan 7 (capability probing + tplot doctor)"
printf '\n\033[2mNote: When you run `tplot doctor` for real, the output reflects YOUR terminal.\033[0m\n'
printf '\033[2mTry it in iTerm2, Kitty, Apple Terminal, an SSH session — you should see different results.\033[0m\n\n'
