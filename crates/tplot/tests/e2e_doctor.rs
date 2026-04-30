use tplot::commands::doctor::format_report_with_env;
use tplot_protocol::{Capabilities, ColorDepth, GlyphSet, GraphicsProtocol, Theme};

/// Deterministic environment getter — pretends we're on iTerm with truecolor.
fn fake_env(name: &str) -> Option<String> {
    match name {
        "TERM" => Some("xterm-256color".into()),
        "TERM_PROGRAM" => Some("iTerm.app".into()),
        "COLORTERM" => Some("truecolor".into()),
        _ => None,
    }
}

/// Empty environment — every variable is unset.
fn empty_env(_name: &str) -> Option<String> {
    None
}

#[test]
fn doctor_report_truecolor_kitty_dark_snapshot() {
    let caps = Capabilities {
        color_depth: ColorDepth::Truecolor,
        glyph_set: GlyphSet::Octants,
        graphics_protocol: GraphicsProtocol::Kitty,
        theme: Theme::Dark,
    };
    insta::assert_snapshot!(
        "doctor_truecolor_kitty_dark",
        format_report_with_env(&caps, fake_env)
    );
}

#[test]
fn doctor_report_ansi16_light_no_graphics_snapshot() {
    let caps = Capabilities {
        color_depth: ColorDepth::Ansi16,
        glyph_set: GlyphSet::HalfBlocks,
        graphics_protocol: GraphicsProtocol::None,
        theme: Theme::Light,
    };
    insta::assert_snapshot!(
        "doctor_ansi16_light_nographics",
        format_report_with_env(&caps, empty_env)
    );
}
