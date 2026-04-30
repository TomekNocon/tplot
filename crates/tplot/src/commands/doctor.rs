use anyhow::Result;
use std::fmt::Write as _;
use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

pub fn run() -> Result<String> {
    let caps = crate::pipeline::probe_capabilities();
    Ok(format_report_with_env(&caps, |name| {
        std::env::var(name).ok()
    }))
}

/// Format a capability report using the live process environment for the
/// "Environment" section. Snapshot-friendly callers should use
/// [`format_report_with_env`] with a controlled getter instead.
pub fn format_report(caps: &Capabilities) -> String {
    format_report_with_env(caps, |name| std::env::var(name).ok())
}

/// Same as [`format_report`] but takes a getter so tests can supply a
/// deterministic environment without touching `std::env`.
pub fn format_report_with_env<F>(caps: &Capabilities, get_env: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::with_capacity(512);
    let _ = writeln!(out, "TerminalPlot — terminal diagnostic");
    let _ = writeln!(out, "==================================");

    let term = get_env("TERM").unwrap_or_default();
    let term_program = get_env("TERM_PROGRAM").unwrap_or_default();
    let colorterm = get_env("COLORTERM").unwrap_or_default();
    let colorfgbg = get_env("COLORFGBG").unwrap_or_default();

    let _ = writeln!(out);
    let _ = writeln!(out, "Environment");
    let _ = writeln!(
        out,
        "  TERM         = {}",
        if term.is_empty() { "(unset)" } else { &term }
    );
    let _ = writeln!(
        out,
        "  TERM_PROGRAM = {}",
        if term_program.is_empty() {
            "(unset)"
        } else {
            &term_program
        }
    );
    let _ = writeln!(
        out,
        "  COLORTERM    = {}",
        if colorterm.is_empty() {
            "(unset)"
        } else {
            &colorterm
        }
    );
    let _ = writeln!(
        out,
        "  COLORFGBG    = {}",
        if colorfgbg.is_empty() {
            "(unset)"
        } else {
            &colorfgbg
        }
    );

    let _ = writeln!(out);
    let _ = writeln!(out, "Detected capabilities");
    let _ = writeln!(
        out,
        "  Color depth       : {}",
        color_depth_label(caps.color_depth)
    );
    let _ = writeln!(
        out,
        "  Glyph set         : {}",
        glyph_set_label(caps.glyph_set)
    );
    let _ = writeln!(out, "  Theme             : {}", theme_label(caps.theme));
    let _ = writeln!(
        out,
        "  Graphics protocol : {}",
        graphics_label(caps.graphics_protocol)
    );

    let _ = writeln!(out);
    let _ = writeln!(out, "Recommendations");
    if matches!(caps.graphics_protocol, GraphicsProtocol::None) {
        let _ = writeln!(
            out,
            "  • Text rendering only — no graphics-protocol output is supported."
        );
    } else {
        let _ = writeln!(
            out,
            "  • Run with --graphics for high-fidelity image-protocol rendering"
        );
        let _ = writeln!(
            out,
            "    (planned in Plan 7b — pipes a PNG to {} escapes).",
            graphics_label(caps.graphics_protocol)
        );
    }
    if matches!(caps.color_depth, ColorDepth::Mono | ColorDepth::Ansi16) {
        let _ = writeln!(
            out,
            "  • Color depth is limited; the storytelling palette will degrade gracefully."
        );
    }

    out
}

fn color_depth_label(d: ColorDepth) -> &'static str {
    match d {
        ColorDepth::Mono => "monochrome",
        ColorDepth::Ansi16 => "ANSI 16-color",
        ColorDepth::Ansi256 => "ANSI 256-color",
        ColorDepth::Truecolor => "truecolor (24-bit RGB)",
    }
}

fn glyph_set_label(g: GlyphSet) -> &'static str {
    match g {
        GlyphSet::Ascii => "ASCII only (limited)",
        GlyphSet::HalfBlocks => "half-blocks (▀▄█)",
        GlyphSet::Braille => "Braille (2x4 dots/cell) + half-blocks",
        GlyphSet::Octants => "Octants + Braille + half-blocks (full set)",
    }
}

fn theme_label(t: Theme) -> &'static str {
    match t {
        Theme::Dark => "dark",
        Theme::Light => "light",
    }
}

fn graphics_label(p: GraphicsProtocol) -> &'static str {
    match p {
        GraphicsProtocol::None => "none",
        GraphicsProtocol::Kitty => "Kitty",
        GraphicsProtocol::ITerm2 => "iTerm2 inline-image",
        GraphicsProtocol::Sixel => "Sixel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_contains_all_sections() {
        let caps = Capabilities {
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::Octants,
            graphics_protocol: GraphicsProtocol::Kitty,
            theme: Theme::Dark,
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
            color_depth: ColorDepth::Truecolor,
            glyph_set: GlyphSet::Octants,
            graphics_protocol: GraphicsProtocol::None,
            theme: Theme::Dark,
        };
        let r = format_report(&caps);
        assert!(r.to_lowercase().contains("text rendering only"));
    }
}
