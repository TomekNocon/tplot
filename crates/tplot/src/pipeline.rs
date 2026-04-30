//! Orchestration helpers shared between subcommands.
//!
//! The pipeline glues input parsing, the story-pass, layout, rasterization,
//! and rendering. Concrete chart subcommands (e.g., `commands::bar`) own the
//! decision of *which* layout/raster functions to call; the pipeline provides
//! the I/O scaffolding.
use anyhow::{Result, anyhow};
use std::io::{self, Read};
use std::time::Duration;
use tplot_core::{
    dataframe::DataFrame,
    input::{parse_csv_str, parse_json_str},
};
use tplot_protocol::{Capabilities, GraphicsProtocol};
use tplot_render::probe::{da1, kitty};

const PROBE_TIMEOUT: Duration = Duration::from_millis(80);

pub fn read_dataframe(path: &str) -> Result<DataFrame> {
    let raw = if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        std::fs::read_to_string(path)?
    };
    if raw.trim_start().starts_with('{') {
        Ok(parse_json_str(&raw)?.dataframe)
    } else {
        Ok(parse_csv_str(&raw)?)
    }
}

pub fn detected_terminal_size(width_override: Option<usize>) -> (usize, usize) {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let w = width_override.unwrap_or(w as usize).max(40);
    (w, (h as usize).max(8))
}

/// Like [`detected_terminal_size`] but rejects widths below 40 cells with a
/// clear, actionable error instead of silently clamping. Width-sensitive
/// pipelines (every chart except sparkline) use this to bail early before
/// layouts overflow.
pub fn require_minimum_width(width_override: Option<usize>) -> Result<(usize, usize)> {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    let w_actual = width_override.unwrap_or(w as usize);
    if w_actual < 40 {
        return Err(anyhow!(
            "terminal too narrow: needs at least 40 cells, got {w_actual}. \
             Resize the terminal or pass --width 40 to override."
        ));
    }
    Ok((w_actual, (h as usize).max(8)))
}

/// Probe the live terminal and return a `Capabilities` that combines env
/// detection with OSC probe results. Always restores the terminal mode.
pub fn probe_capabilities() -> Capabilities {
    let env = Capabilities::from_vars(|name| std::env::var(name).ok());

    // Skip probing if not on a TTY (e.g., piped output, CI).
    if !is_tty() {
        return env;
    }

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

/// Translate the `--graphics` CLI flag into a [`GraphicsProtocol`].
///
/// - `"auto"`         → use `caps.graphics_protocol` (env + probe-detected).
/// - `"kitty"`        → force Kitty (trust the user even if probing said no).
/// - `"iterm2"`       → force iTerm2.
/// - `"none"` / empty → `GraphicsProtocol::None` (text rendering).
/// - anything else    → `GraphicsProtocol::None` (don't crash).
pub fn resolve_graphics(flag: &str, caps: Capabilities) -> GraphicsProtocol {
    match flag {
        "auto" => caps.graphics_protocol,
        "kitty" => GraphicsProtocol::Kitty,
        "iterm2" => GraphicsProtocol::ITerm2,
        "none" | "" => GraphicsProtocol::None,
        _ => GraphicsProtocol::None,
    }
}

fn run_probes() -> Option<(bool, bool)> {
    let mut stdout = std::io::stdout();
    let mut stdin = std::io::stdin();
    // Each probe is independent; both run with the same timeout budget.
    let kitty_ok = kitty::probe_kitty(&mut stdout, &mut stdin, PROBE_TIMEOUT).unwrap_or(false);
    let sixel_ok = da1::probe_sixel(&mut stdout, &mut stdin, PROBE_TIMEOUT).unwrap_or(false);
    Some((kitty_ok, sixel_ok))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

    fn caps_with(g: GraphicsProtocol) -> Capabilities {
        Capabilities {
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::Octants,
            graphics_protocol: g,
            theme: Theme::Dark,
        }
    }

    #[test]
    fn rejects_below_minimum_width() {
        let err = require_minimum_width(Some(20)).unwrap_err();
        assert!(err.to_string().contains("at least 40"));
    }

    #[test]
    fn accepts_exactly_minimum_width() {
        assert!(require_minimum_width(Some(40)).is_ok());
    }

    #[test]
    fn resolve_none_returns_none() {
        assert_eq!(
            resolve_graphics("none", caps_with(GraphicsProtocol::Kitty)),
            GraphicsProtocol::None
        );
    }

    #[test]
    fn resolve_kitty_forces_kitty() {
        assert_eq!(
            resolve_graphics("kitty", caps_with(GraphicsProtocol::None)),
            GraphicsProtocol::Kitty
        );
    }

    #[test]
    fn resolve_iterm2_forces_iterm2() {
        assert_eq!(
            resolve_graphics("iterm2", caps_with(GraphicsProtocol::None)),
            GraphicsProtocol::ITerm2
        );
    }

    #[test]
    fn resolve_auto_uses_caps() {
        assert_eq!(
            resolve_graphics("auto", caps_with(GraphicsProtocol::Kitty)),
            GraphicsProtocol::Kitty
        );
        assert_eq!(
            resolve_graphics("auto", caps_with(GraphicsProtocol::ITerm2)),
            GraphicsProtocol::ITerm2
        );
        assert_eq!(
            resolve_graphics("auto", caps_with(GraphicsProtocol::None)),
            GraphicsProtocol::None
        );
    }
}
