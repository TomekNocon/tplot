//! Orchestration helpers shared between subcommands.
//!
//! The pipeline glues input parsing, the story-pass, layout, rasterization,
//! and rendering. Concrete chart subcommands (e.g., `commands::bar`) own the
//! decision of *which* layout/raster functions to call; the pipeline provides
//! the I/O scaffolding.
use anyhow::Result;
use std::io::{self, Read};
use tplot_core::{dataframe::DataFrame, input::{parse_csv_str, parse_json_str}};

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
