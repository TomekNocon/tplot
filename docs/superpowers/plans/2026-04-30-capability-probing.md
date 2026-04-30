# TerminalPlot — Plan 7: Capability Probing + tplot doctor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Replace env-only capability detection with real OSC-based probing of the live terminal. Add a `tplot doctor` subcommand that runs the probes and prints a friendly report ("Kitty graphics: yes • Sixel: no • truecolor: yes • theme: dark"). The probing piece sets up Plan 7b (image protocol output) — `--graphics` mode needs to know whether the terminal can actually receive image escapes before sending them.

**Architecture:** New `probe` module in `tplot-render`. Probe functions take generic `Read`/`Write` so they're unit-testable with in-memory buffers; the binary wraps them with crossterm raw-mode handling. `Capabilities::probe_terminal()` is the high-level entry point — env detection first (cheap, always works), OSC probe second (50ms timeout per query, can fail).

**Tech Stack:** Same as prior plans + `crossterm`'s raw-mode helpers (already a workspace dep). No new dependencies.

**Inherited context (Plans 1–6):**
- 9 chart types, polished error paths, theme detection from env, 214 tests passing.
- `Capabilities { color_depth, glyph_set, graphics_protocol, theme }` is constructed via `Capabilities::from_vars` (env-only).
- All commits attributed to `tomek.lm10@gmail.com`.

---

## File Structure

```
crates/
├── tplot-protocol/src/
│   └── capabilities.rs             EXTEND: ProbeSource + Capabilities builder
├── tplot-render/src/
│   ├── lib.rs                      EXTEND: pub mod probe
│   └── probe/
│       ├── mod.rs                  ★ NEW: public API + read-with-timeout helper
│       ├── da1.rs                  ★ NEW: Device Attributes 1 probe (detects Sixel)
│       └── kitty.rs                ★ NEW: Kitty graphics protocol probe
├── tplot/src/
│   ├── cli.rs                      EXTEND: Doctor subcommand (no args)
│   ├── commands/
│   │   ├── mod.rs                  EXTEND
│   │   └── doctor.rs               ★ NEW: orchestrates probes + formats the report
│   ├── pipeline.rs                 EXTEND: helper to run probes inside raw mode
│   └── main.rs                     EXTEND: Doctor match arm
└── tplot/tests/
    └── e2e_doctor.rs               ★ NEW: snapshot test for doctor output (env-only mode)
```

---

## Task 1: probe module — read-with-timeout helper

**Files:**
- Create: `crates/tplot-render/src/probe/mod.rs`
- Modify: `crates/tplot-render/src/lib.rs`

The probing core is "send some bytes, then read until we see a terminator OR a timeout fires." Keep it generic over `Read + Write` so probing can be tested without a real terminal.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/probe/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn read_until_finds_terminator() {
        let mut reader = Cursor::new(b"hello\x07world".to_vec());
        let buf = read_until(&mut reader, b'\x07', Duration::from_secs(1)).unwrap();
        assert_eq!(buf, b"hello".to_vec());
    }

    #[test]
    fn read_until_returns_what_it_has_on_eof() {
        let mut reader = Cursor::new(b"partial".to_vec());
        let buf = read_until(&mut reader, b'\x07', Duration::from_millis(50));
        // Should return what was read so far (non-Ok timeout) — implementation-defined
        // but should not panic.
        assert!(buf.is_err() || buf.unwrap() == b"partial".to_vec());
    }

    #[test]
    fn write_query_then_read() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"\x1b[?62;4c".to_vec());
        let response = query(&mut writer, &mut reader, b"\x1b[c", b'c', Duration::from_secs(1)).unwrap();
        assert_eq!(writer, b"\x1b[c");
        // Returned bytes include the terminator? — strip it; we just want the content.
        assert!(response.contains(&b'?'), "response should include the device attributes prefix");
    }
}
```

- [ ] **Step 2: Add module ref**

```rust
// crates/tplot-render/src/lib.rs (append)
pub mod probe;
```

- [ ] **Step 3: Run (expected fail)**

```bash
cargo test -p tplot-render --lib probe
```

- [ ] **Step 4: Implement**

```rust
// crates/tplot-render/src/probe/mod.rs (above tests)
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub mod da1;
pub mod kitty;

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("timed out waiting for terminal response")]
    Timeout,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Read from `reader` until a byte equal to `terminator` is encountered,
/// returning all bytes BEFORE the terminator (the terminator itself is
/// consumed but not included). Times out after `timeout` elapses.
///
/// Reads one byte at a time. Designed for OSC/CSI responses, which are
/// short (≤ 64 bytes) and have a clear terminator.
pub fn read_until<R: Read>(
    reader:     &mut R,
    terminator: u8,
    timeout:    Duration,
) -> Result<Vec<u8>, ProbeError> {
    let deadline = Instant::now() + timeout;
    let mut out = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    while Instant::now() < deadline {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                if byte[0] == terminator { return Ok(out); }
                out.push(byte[0]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(ProbeError::Io(e)),
        }
    }
    if out.is_empty() { Err(ProbeError::Timeout) } else { Ok(out) }
}

/// Send `query` bytes to `writer`, flush, then read from `reader` until
/// `terminator` is seen (or we time out).
pub fn query<W: Write, R: Read>(
    writer:     &mut W,
    reader:     &mut R,
    query:      &[u8],
    terminator: u8,
    timeout:    Duration,
) -> Result<Vec<u8>, ProbeError> {
    writer.write_all(query)?;
    writer.flush()?;
    read_until(reader, terminator, timeout)
}
```

Add `thiserror` to `tplot-render`'s `Cargo.toml` if not already present:
```toml
thiserror = { workspace = true }
```

- [ ] **Step 5: Run (expected pass)**

- [ ] **Step 6: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add probe module with read-with-timeout and query helpers"
```

---

## Task 2: DA1 probe — detects Sixel support

**Files:**
- Create: `crates/tplot-render/src/probe/da1.rs`

Send `ESC [ c` (Device Attributes Primary). Modern terminals respond with `ESC [ ? <ps>...c` where `<ps>` is a list of supported features. Sixel is `4`. Other features include 22 (color), 28 (rectangular editing), etc.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/probe/da1.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn parses_sixel_capable_response() {
        // Real iTerm2 / xterm-with-sixel response: "\x1b[?62;4;22c"
        let response = b"\x1b[?62;4;22";
        assert!(parse_response_supports_sixel(response));
    }

    #[test]
    fn parses_sixel_absent_response() {
        // A response without "4" (no sixel)
        let response = b"\x1b[?62;22";
        assert!(!parse_response_supports_sixel(response));
    }

    #[test]
    fn parses_response_with_4_only_in_a_substring() {
        // "?64;1" should NOT match because "4" appears only as part of "64".
        let response = b"\x1b[?64;1";
        assert!(!parse_response_supports_sixel(response));
    }

    #[test]
    fn full_query_round_trip() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"\x1b[?62;4;22c".to_vec());
        let supports = probe_sixel(&mut writer, &mut reader, Duration::from_millis(50)).unwrap();
        assert_eq!(writer, b"\x1b[c");
        assert!(supports);
    }
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/probe/da1.rs (above tests)
use super::{query, ProbeError};
use std::io::{Read, Write};
use std::time::Duration;

const DA1_QUERY:      &[u8] = b"\x1b[c";
const DA1_TERMINATOR: u8    = b'c';

/// Probe the terminal for Sixel graphics support via DA1.
pub fn probe_sixel<W: Write, R: Read>(
    writer:  &mut W,
    reader:  &mut R,
    timeout: Duration,
) -> Result<bool, ProbeError> {
    let response = query(writer, reader, DA1_QUERY, DA1_TERMINATOR, timeout)?;
    Ok(parse_response_supports_sixel(&response))
}

/// Parse a DA1 response and return true if "4" appears as a standalone
/// numeric feature code. Response format: "\x1b[?<ps>;<ps>;...c", e.g.
/// "\x1b[?62;4;22c". The leading "\x1b[?" prefix and trailing "c" are
/// optional in our caller's view (read_until strips the terminator).
pub fn parse_response_supports_sixel(response: &[u8]) -> bool {
    // Find the last "?" — features come after it, separated by ";".
    let s = match std::str::from_utf8(response) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let after_question = match s.rfind('?') {
        Some(i) => &s[i + 1..],
        None    => s,
    };
    after_question
        .split(';')
        .any(|f| f.trim() == "4")
}
```

- [ ] **Step 4: Run (expected pass)**

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add DA1 probe for Sixel support detection"
```

---

## Task 3: Kitty graphics probe

**Files:**
- Create: `crates/tplot-render/src/probe/kitty.rs`

Kitty's graphics protocol uses a "query" form: send a tiny placeholder image with `a=q` action; if the terminal supports the protocol, it responds with a status string starting with `OK`. Format:

```
\x1b_Gi=31,a=q,t=d,f=24,s=1,v=1;AAAA\x1b\\
```

Response (Kitty): `\x1b_Gi=31;OK\x1b\\`. The terminator is the `\x1b\` (ESC backslash, "string terminator"). For our purposes we read until `\x1b` and check for `OK` in the body.

Most non-Kitty terminals don't respond at all → probe times out → returns `false`.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-render/src/probe/kitty.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::time::Duration;

    #[test]
    fn parses_ok_response() {
        // "\x1b_Gi=31;OK\x1b" (the trailing \x1b would have been the terminator).
        assert!(parse_response_supports_kitty(b"_Gi=31;OK"));
    }

    #[test]
    fn parses_error_response() {
        assert!(!parse_response_supports_kitty(b"_Gi=31;ENOTSUPPORTED:not implemented"));
    }

    #[test]
    fn parses_unrelated_response() {
        // Some terminals may echo back the query bytes — that's not a Kitty OK.
        assert!(!parse_response_supports_kitty(b"\x1b[c"));
    }

    #[test]
    fn full_query_round_trip() {
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(b"_Gi=31;OK\x1b".to_vec());
        let supports = probe_kitty(&mut writer, &mut reader, Duration::from_millis(50)).unwrap();
        assert!(writer.starts_with(b"\x1b_Gi=31"));
        assert!(supports);
    }

    #[test]
    fn timeout_returns_false_not_error() {
        // No reader response — should NOT bubble the timeout as a Result::Err;
        // we want a clean "false" so probing doesn't fail-stop the doctor.
        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(Vec::<u8>::new());
        let supports = probe_kitty(&mut writer, &mut reader, Duration::from_millis(20)).unwrap();
        assert!(!supports);
    }
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-render/src/probe/kitty.rs (above tests)
use super::{query, ProbeError};
use std::io::{Read, Write};
use std::time::Duration;

/// Tiny query: send a 1x1 RGBA-style "query" image.
/// (`a=q` = action: query — terminal answers without rendering anything.)
const KITTY_QUERY: &[u8] =
    b"\x1b_Gi=31,a=q,t=d,f=24,s=1,v=1;AAAA\x1b\\";

const ESC: u8 = b'\x1b';

/// Probe the terminal for Kitty graphics protocol support.
/// Timeouts are normal (most terminals don't speak Kitty); they're returned
/// as `Ok(false)` so the doctor pipeline can keep going.
pub fn probe_kitty<W: Write, R: Read>(
    writer:  &mut W,
    reader:  &mut R,
    timeout: Duration,
) -> Result<bool, ProbeError> {
    match query(writer, reader, KITTY_QUERY, ESC, timeout) {
        Ok(response) => Ok(parse_response_supports_kitty(&response)),
        Err(ProbeError::Timeout) => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn parse_response_supports_kitty(response: &[u8]) -> bool {
    let s = match std::str::from_utf8(response) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains(";OK")
}
```

- [ ] **Step 4: Run (expected pass)**

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-render/
git commit -m "Add Kitty graphics protocol probe"
```

---

## Task 4: Capabilities builder — combine env + probes

**Files:**
- Modify: `crates/tplot-protocol/src/capabilities.rs`

Add a `ProbeSource { Env, Probe, Default }` annotation and a `Capabilities::merge_with_probe(probed_kitty, probed_sixel)` constructor that takes probe results. The doctor command builds Capabilities from env first, then overlays probe results.

- [ ] **Step 1: Write failing tests**

```rust
// crates/tplot-protocol/src/capabilities.rs (within existing tests module)
#[test]
fn probe_promotes_graphics_protocol_when_env_unknown() {
    let env_caps = Capabilities::from_vars(|name| match name {
        "TERM" => Some("xterm-256color".into()),
        _      => None,
    });
    assert_eq!(env_caps.graphics_protocol, GraphicsProtocol::None);

    let with_probe = env_caps.with_probe_results(true, false);
    assert_eq!(with_probe.graphics_protocol, GraphicsProtocol::Kitty);
}

#[test]
fn probe_falls_back_to_sixel_when_only_sixel_works() {
    let env_caps = Capabilities::conservative();
    let with_probe = env_caps.with_probe_results(false, true);
    assert_eq!(with_probe.graphics_protocol, GraphicsProtocol::Sixel);
}

#[test]
fn probe_does_not_demote_existing_iterm2_detection() {
    let env_caps = Capabilities::from_vars(|name| match name {
        "TERM_PROGRAM" => Some("iTerm.app".into()),
        _              => None,
    });
    assert_eq!(env_caps.graphics_protocol, GraphicsProtocol::ITerm2);

    // Sixel probe says false; iTerm2 was already detected. Keep iTerm2.
    let with_probe = env_caps.with_probe_results(false, false);
    assert_eq!(with_probe.graphics_protocol, GraphicsProtocol::ITerm2);
}
```

- [ ] **Step 2: Run (expected fail)**

- [ ] **Step 3: Implement**

```rust
// crates/tplot-protocol/src/capabilities.rs (append on impl Capabilities)
impl Capabilities {
    /// Merge probe results into env-detected capabilities.
    /// Probes promote the graphics_protocol upward; they don't demote.
    /// Priority order (highest first): Kitty, iTerm2, Sixel, None.
    pub fn with_probe_results(mut self, kitty: bool, sixel: bool) -> Self {
        let env_priority = priority(self.graphics_protocol);
        let probe_protocol = if kitty {
            GraphicsProtocol::Kitty
        } else if sixel {
            GraphicsProtocol::Sixel
        } else {
            GraphicsProtocol::None
        };
        if priority(probe_protocol) > env_priority {
            self.graphics_protocol = probe_protocol;
        }
        self
    }
}

fn priority(p: GraphicsProtocol) -> u8 {
    match p {
        GraphicsProtocol::Kitty   => 3,
        GraphicsProtocol::ITerm2  => 2,
        GraphicsProtocol::Sixel   => 1,
        GraphicsProtocol::None    => 0,
    }
}
```

- [ ] **Step 4: Run (expected pass)**

- [ ] **Step 5: Commit**

```bash
git add crates/tplot-protocol/
git commit -m "Add Capabilities::with_probe_results to merge env + OSC probe"
```

---

## Task 5: Pipeline helper — run probes inside crossterm raw mode

**Files:**
- Modify: `crates/tplot/src/pipeline.rs`

The probes need the terminal in raw mode (so reads don't block on Enter, and bytes aren't echoed). crossterm's `terminal::enable_raw_mode()` / `disable_raw_mode()` are the toggles. The helper:
1. Enables raw mode.
2. Runs the probes against `std::io::stdout()` and `std::io::stdin()`.
3. Disables raw mode regardless of success/failure (defer pattern).

This task is harder to unit-test (relies on a real TTY). Skip TDD here and do a smoke test instead.

- [ ] **Step 1: Implement**

```rust
// crates/tplot/src/pipeline.rs (append)
use std::io::{Read, Write};
use std::time::Duration;
use tplot_protocol::Capabilities;
use tplot_render::probe::{da1, kitty};

const PROBE_TIMEOUT: Duration = Duration::from_millis(80);

/// Probe the live terminal and return a `Capabilities` that combines env
/// detection with OSC probe results. Always restores the terminal mode.
pub fn probe_capabilities() -> Capabilities {
    let env = Capabilities::from_vars(|name| std::env::var(name).ok());

    // Skip probing if not on a TTY (e.g., piped output, CI).
    if !is_tty() { return env; }

    // Enable raw mode; whether or not it succeeded, we'll always try to
    // disable it before returning.
    let raw_was_enabled = crossterm::terminal::enable_raw_mode().is_ok();

    let result = run_probes();

    if raw_was_enabled {
        let _ = crossterm::terminal::disable_raw_mode();
    }

    let (kitty_ok, sixel_ok) = result.unwrap_or((false, false));
    env.with_probe_results(kitty_ok, sixel_ok)
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal() && std::io::stdin().is_terminal()
}

fn run_probes() -> Option<(bool, bool)> {
    let mut stdout = std::io::stdout();
    let mut stdin  = std::io::stdin();
    // Each probe is independent; both run with the same timeout budget.
    let kitty_ok = kitty::probe_kitty(&mut stdout, &mut stdin, PROBE_TIMEOUT).unwrap_or(false);
    let sixel_ok = da1::probe_sixel(&mut stdout, &mut stdin, PROBE_TIMEOUT).unwrap_or(false);
    Some((kitty_ok, sixel_ok))
}
```

Add the `IsTerminal` trait import — it's stable since Rust 1.70.

- [ ] **Step 2: Smoke test**

Just confirm it builds and doesn't panic — actual probe results depend on the terminal:

```bash
cargo build -p tplot
cargo run -p tplot -- bar tests/fixtures/sales.csv -x quarter -y revenue --group region
```
(Should still render normally; the new code is only used by the doctor.)

- [ ] **Step 3: Commit**

```bash
git add crates/tplot/
git commit -m "Add pipeline::probe_capabilities — run OSC probes in raw mode"
```

---

## Task 6: doctor subcommand

**Files:**
- Create: `crates/tplot/src/commands/doctor.rs`
- Modify: `crates/tplot/src/commands/mod.rs`
- Modify: `crates/tplot/src/cli.rs`
- Modify: `crates/tplot/src/main.rs`

A small, formatted report that prints what's detected.

- [ ] **Step 1: Add CLI subcommand**

```rust
// crates/tplot/src/cli.rs — modify Command enum
#[derive(Subcommand, Debug)]
pub enum Command {
    Bar(BarArgs),
    Hist(HistArgs),
    Line(LineArgs),
    Scatter(ScatterArgs),
    Spark(SparkArgs),
    Heatmap(HeatmapArgs),
    Box(BoxArgs),
    Area(AreaArgs),
    /// Probe the terminal and print a capability report.
    Doctor,
    Json,
}
```

Add a parse-test:
```rust
// crates/tplot/src/cli.rs (within existing tests module)
#[test]
fn parses_doctor_subcommand() {
    let args = Cli::parse_from(["tplot", "doctor"]);
    assert!(matches!(args.command, Command::Doctor));
}
```

- [ ] **Step 2: Write the doctor module**

```rust
// crates/tplot/src/commands/doctor.rs
use anyhow::Result;
use std::fmt::Write as _;
use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

pub fn run() -> Result<String> {
    let caps = crate::pipeline::probe_capabilities();
    Ok(format_report(&caps))
}

pub fn format_report(caps: &Capabilities) -> String {
    let mut out = String::with_capacity(512);
    let _ = writeln!(out, "TerminalPlot — terminal diagnostic");
    let _ = writeln!(out, "==================================");

    let term         = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let colorterm    = std::env::var("COLORTERM").unwrap_or_default();
    let colorfgbg    = std::env::var("COLORFGBG").unwrap_or_default();

    let _ = writeln!(out, "");
    let _ = writeln!(out, "Environment");
    let _ = writeln!(out, "  TERM         = {}", if term.is_empty()         { "(unset)" } else { &term });
    let _ = writeln!(out, "  TERM_PROGRAM = {}", if term_program.is_empty() { "(unset)" } else { &term_program });
    let _ = writeln!(out, "  COLORTERM    = {}", if colorterm.is_empty()    { "(unset)" } else { &colorterm });
    let _ = writeln!(out, "  COLORFGBG    = {}", if colorfgbg.is_empty()    { "(unset)" } else { &colorfgbg });

    let _ = writeln!(out, "");
    let _ = writeln!(out, "Detected capabilities");
    let _ = writeln!(out, "  Color depth       : {}", color_depth_label(caps.color_depth));
    let _ = writeln!(out, "  Glyph set         : {}", glyph_set_label(caps.glyph_set));
    let _ = writeln!(out, "  Theme             : {}", theme_label(caps.theme));
    let _ = writeln!(out, "  Graphics protocol : {}", graphics_label(caps.graphics_protocol));

    let _ = writeln!(out, "");
    let _ = writeln!(out, "Recommendations");
    if matches!(caps.graphics_protocol, GraphicsProtocol::None) {
        let _ = writeln!(out, "  • Text rendering only — no graphics-protocol output is supported.");
    } else {
        let _ = writeln!(out, "  • Run with --graphics for high-fidelity image-protocol rendering");
        let _ = writeln!(out, "    (planned in Plan 7b — pipes a PNG to {} escapes).", graphics_label(caps.graphics_protocol));
    }
    if matches!(caps.color_depth, ColorDepth::Mono | ColorDepth::Ansi16) {
        let _ = writeln!(out, "  • Color depth is limited; the storytelling palette will degrade gracefully.");
    }

    out
}

fn color_depth_label(d: ColorDepth) -> &'static str {
    match d {
        ColorDepth::Mono      => "monochrome",
        ColorDepth::Ansi16    => "ANSI 16-color",
        ColorDepth::Ansi256   => "ANSI 256-color",
        ColorDepth::Truecolor => "truecolor (24-bit RGB)",
    }
}

fn glyph_set_label(g: GlyphSet) -> &'static str {
    match g {
        GlyphSet::Ascii      => "ASCII only (limited)",
        GlyphSet::HalfBlocks => "half-blocks (▀▄█)",
        GlyphSet::Braille    => "Braille (2x4 dots/cell) + half-blocks",
        GlyphSet::Octants    => "Octants + Braille + half-blocks (full set)",
    }
}

fn theme_label(t: Theme) -> &'static str {
    match t {
        Theme::Dark  => "dark",
        Theme::Light => "light",
    }
}

fn graphics_label(p: GraphicsProtocol) -> &'static str {
    match p {
        GraphicsProtocol::None    => "none",
        GraphicsProtocol::Kitty   => "Kitty",
        GraphicsProtocol::ITerm2  => "iTerm2 inline-image",
        GraphicsProtocol::Sixel   => "Sixel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_contains_all_sections() {
        let caps = Capabilities {
            color_depth:       ColorDepth::Truecolor,
            glyph_set:         GlyphSet::Octants,
            graphics_protocol: GraphicsProtocol::Kitty,
            theme:             Theme::Dark,
        };
        let r = format_report(&caps);
        assert!(r.contains("Environment"));
        assert!(r.contains("Detected capabilities"));
        assert!(r.contains("Recommendations"));
        assert!(r.contains("truecolor"));
        assert!(r.contains("Octants"));
        assert!(r.contains("dark"));
        assert!(r.contains("Kitty"));
    }

    #[test]
    fn report_recommends_text_only_when_no_graphics() {
        let caps = Capabilities {
            color_depth:       ColorDepth::Truecolor,
            glyph_set:         GlyphSet::Octants,
            graphics_protocol: GraphicsProtocol::None,
            theme:             Theme::Dark,
        };
        let r = format_report(&caps);
        assert!(r.to_lowercase().contains("text rendering only"));
    }
}
```

- [ ] **Step 3: Wire into commands/mod.rs and main.rs**

```rust
// crates/tplot/src/commands/mod.rs (append)
pub mod doctor;
```

```rust
// crates/tplot/src/main.rs — add Doctor match arm
Command::Doctor => {
    let report = commands::doctor::run()?;
    print!("{report}");
    Ok(())
}
```

- [ ] **Step 4: Run tests + smoke test**

```bash
cargo test -p tplot --lib commands::doctor
cargo build --workspace
cargo run -p tplot -- doctor
```

The smoke test should print something like:
```
TerminalPlot — terminal diagnostic
==================================

Environment
  TERM         = xterm-256color
  TERM_PROGRAM = iTerm.app
  COLORTERM    = truecolor
  COLORFGBG    = (unset)

Detected capabilities
  Color depth       : truecolor (24-bit RGB)
  Glyph set         : Octants + Braille + half-blocks (full set)
  Theme             : dark
  Graphics protocol : iTerm2 inline-image

Recommendations
  • Run with --graphics for high-fidelity image-protocol rendering
    (planned in Plan 7b — pipes a PNG to iTerm2 inline-image escapes).
```

- [ ] **Step 5: Commit**

```bash
git add crates/tplot/
git commit -m "Add tplot doctor subcommand with capability report"
```

---

## Task 7: Snapshot test for doctor (env-only mode) + README + lint

**Files:**
- Create: `crates/tplot/tests/e2e_doctor.rs`
- Modify: `README.md`

The probing is non-deterministic (depends on the actual terminal under test), so we snapshot only the env-driven path: invoke `format_report` directly with a known `Capabilities`, not via `tplot doctor` running the probes.

- [ ] **Step 1: Write the snapshot test**

This test calls into the library, not the binary, because the binary's behavior depends on the test runner's terminal:

```rust
// crates/tplot/tests/e2e_doctor.rs
use tplot::commands::doctor::format_report;
use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

#[test]
fn doctor_report_truecolor_kitty_dark_snapshot() {
    let caps = Capabilities {
        color_depth:       ColorDepth::Truecolor,
        glyph_set:         GlyphSet::Octants,
        graphics_protocol: GraphicsProtocol::Kitty,
        theme:             Theme::Dark,
    };
    insta::assert_snapshot!("doctor_truecolor_kitty_dark", format_report(&caps));
}

#[test]
fn doctor_report_ansi16_light_no_graphics_snapshot() {
    let caps = Capabilities {
        color_depth:       ColorDepth::Ansi16,
        glyph_set:         GlyphSet::HalfBlocks,
        graphics_protocol: GraphicsProtocol::None,
        theme:             Theme::Light,
    };
    insta::assert_snapshot!("doctor_ansi16_light_nographics", format_report(&caps));
}
```

For the binary crate's tests to call into `commands::doctor::format_report`, the `tplot` crate needs a `lib.rs` exposing the relevant module, OR the test file can be in the binary crate's `src/` instead.

Simplest: make `commands::doctor::format_report` `pub` and have an integration test reach into the binary's source via a `[lib]` target or `path = "src/commands/doctor.rs"` declaration. If that's awkward, move the test to a unit test inside `commands/doctor.rs` (it's already there from Task 6) and skip the integration snapshot — the unit tests cover the same behavior.

(If the integration path is too fiddly, just rely on the Task-6 unit tests and skip Task 7's snapshot test.)

- [ ] **Step 2: Run, inspect, accept (or skip if integration path is awkward)**

```bash
cargo test -p tplot --test e2e_doctor 2>&1
# If it fails to find format_report, skip and commit only the README + lint.
cargo insta accept
```

- [ ] **Step 3: Update README**

Add to the "What's in this version" section (now Plans 1+2+3+4+4.5+5+5.5+6+7):

```markdown
- `tplot doctor` — prints a capability report (color depth, glyph set, theme, graphics-protocol detection). Run it once to see how `tplot` views your terminal.
```

Add a quickstart line:
```bash
$ tplot doctor
TerminalPlot — terminal diagnostic
==================================
...
```

- [ ] **Step 4: Lint pass**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- [ ] **Step 5: Commit**

```bash
git add README.md crates/tplot/
# If fmt/clippy made changes:
git add -A
git commit -m "Add doctor snapshot test + README quickstart, lint pass"
```

---

## Done — Plan 7 deliverable

```bash
$ tplot doctor
TerminalPlot — terminal diagnostic
==================================

Environment
  TERM         = xterm-kitty
  TERM_PROGRAM = (unset)
  COLORTERM    = truecolor
  COLORFGBG    = 15;0

Detected capabilities
  Color depth       : truecolor (24-bit RGB)
  Glyph set         : Octants + Braille + half-blocks (full set)
  Theme             : dark
  Graphics protocol : Kitty

Recommendations
  • Run with --graphics for high-fidelity image-protocol rendering
    (planned in Plan 7b — pipes a PNG to Kitty escapes).
```

Plus probe-source data is now available to the binary, so once Plan 7b ships, `--graphics` can default to the best-supported protocol without env-only guessing.

**Still missing for v1:** image protocol output (Plan 7b), distribution (Plan 8).

## Self-review notes

- probe module + helpers (Task 1) ✓
- DA1 / Sixel probe (Task 2) ✓
- Kitty probe (Task 3) ✓
- Capabilities builder for probe results (Task 4) ✓
- Pipeline raw-mode helper (Task 5) ✓
- doctor subcommand (Task 6) ✓
- Snapshot tests + README + lint (Task 7) ✓
