//! Orchestration helpers shared between subcommands.
//!
//! The pipeline glues input parsing, the story-pass, layout, rasterization,
//! and rendering. Concrete chart subcommands (e.g., `commands::bar`) own the
//! decision of *which* layout/raster functions to call; the pipeline provides
//! the I/O scaffolding.
use anyhow::{Result, anyhow};
use std::io::{self, Read};
use tplot_core::{
    dataframe::DataFrame,
    input::{parse_csv_str, parse_json_str},
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_below_minimum_width() {
        let err = require_minimum_width(Some(20)).unwrap_err();
        assert!(err.to_string().contains("at least 40"));
    }

    #[test]
    fn accepts_exactly_minimum_width() {
        assert!(require_minimum_width(Some(40)).is_ok());
    }
}
