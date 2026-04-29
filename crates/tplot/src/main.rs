mod cli;
mod commands;
mod pipeline;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use commands::{HistogramOptions, RenderOptions, render_bar, render_histogram};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::Bar(b) => {
            let df = pipeline::read_dataframe(&b.input)?;
            let (_, h) = pipeline::detected_terminal_size(b.common.width);
            let out = render_bar(
                &df,
                &RenderOptions {
                    x: b.x,
                    y: b.y,
                    group: b.group,
                    vertical: b.vertical,
                    focus: b.common.focus,
                    annotate: b.common.annotate,
                    neutral: b.common.neutral,
                    no_takeaway: b.common.no_takeaway,
                    width: b.common.width,
                    height: h,
                    palette_name: b.common.palette,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Hist(h) => {
            let df = pipeline::read_dataframe(&h.input)?;
            let (_, height) = pipeline::detected_terminal_size(h.common.width);
            let out = render_histogram(
                &df,
                &HistogramOptions {
                    x: h.x,
                    bins: h.bins,
                    focus: h.common.focus,
                    annotate: h.common.annotate,
                    neutral: h.common.neutral,
                    no_takeaway: h.common.no_takeaway,
                    width: h.common.width,
                    height,
                    palette_name: h.common.palette,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Line(_) | Command::Scatter(_) => {
            // Wired in plan 3 task 12.
            anyhow::bail!("line/scatter wiring lands in plan 3 task 12")
        }
        Command::Json => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            let (w, h) = pipeline::detected_terminal_size(None);
            let out = commands::render_from_json(&buf, w, h)?;
            print!("{out}");
            Ok(())
        }
    }
}
