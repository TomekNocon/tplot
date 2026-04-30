use anyhow::Result;
use clap::Parser;
use tplot::cli::{Cli, Command};
use tplot::commands;
use tplot::commands::{
    AreaOptions, BoxOptions, HeatmapOptions, HistogramOptions, LineOptions, RenderOptions,
    ScatterOptions, render_bar, render_boxplot, render_heatmap, render_histogram, render_line,
    render_scatter, render_stacked_area,
};
use tplot::pipeline;

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
                    graphics: b.common.graphics,
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
                    graphics: h.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Line(l) => {
            let df = pipeline::read_dataframe(&l.input)?;
            let (_, height) = pipeline::detected_terminal_size(l.common.width);
            let out = render_line(
                &df,
                &LineOptions {
                    x: l.x,
                    y: l.y,
                    group: l.group,
                    focus: l.common.focus,
                    annotate: l.common.annotate,
                    neutral: l.common.neutral,
                    no_takeaway: l.common.no_takeaway,
                    width: l.common.width,
                    height,
                    palette_name: l.common.palette,
                    graphics: l.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Scatter(s) => {
            let df = pipeline::read_dataframe(&s.input)?;
            let (_, height) = pipeline::detected_terminal_size(s.common.width);
            let out = render_scatter(
                &df,
                &ScatterOptions {
                    x: s.x,
                    y: s.y,
                    group: s.group,
                    focus: s.common.focus,
                    annotate: s.common.annotate,
                    neutral: s.common.neutral,
                    no_takeaway: s.common.no_takeaway,
                    width: s.common.width,
                    height,
                    palette_name: s.common.palette,
                    graphics: s.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Spark(s) => {
            let raw = if s.input == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                std::fs::read_to_string(&s.input)?
            };
            let out = commands::render_sparkline(
                &raw,
                &commands::SparkOptions {
                    input: s.input,
                    y: s.y,
                    palette_name: s.palette,
                    no_color: s.no_color,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Heatmap(h) => {
            let df = pipeline::read_dataframe(&h.input)?;
            let (_, height) = pipeline::detected_terminal_size(h.common.width);
            let out = render_heatmap(
                &df,
                &HeatmapOptions {
                    x: h.x,
                    y: h.y,
                    value: h.value,
                    ramp_name: h.ramp,
                    annotate: h.common.annotate,
                    no_takeaway: h.common.no_takeaway,
                    width: h.common.width,
                    height,
                    graphics: h.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Box(b) => {
            let df = pipeline::read_dataframe(&b.input)?;
            let (_, height) = pipeline::detected_terminal_size(b.common.width);
            let out = render_boxplot(
                &df,
                &BoxOptions {
                    x: b.x,
                    y: b.y,
                    focus: b.common.focus,
                    annotate: b.common.annotate,
                    neutral: b.common.neutral,
                    no_takeaway: b.common.no_takeaway,
                    width: b.common.width,
                    height,
                    palette_name: b.common.palette,
                    graphics: b.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Area(a) => {
            let df = pipeline::read_dataframe(&a.input)?;
            let (_, height) = pipeline::detected_terminal_size(a.common.width);
            let out = render_stacked_area(
                &df,
                &AreaOptions {
                    x: a.x,
                    y: a.y,
                    group: a.group,
                    focus: a.common.focus,
                    annotate: a.common.annotate,
                    neutral: a.common.neutral,
                    no_takeaway: a.common.no_takeaway,
                    width: a.common.width,
                    height,
                    palette_name: a.common.palette,
                    graphics: a.common.graphics,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Candle(c) => {
            let df = pipeline::read_dataframe(&c.input)?;
            let (_, height) = pipeline::detected_terminal_size(c.common.width);
            let out = commands::render_candlestick(
                &df,
                &commands::CandleOptions {
                    x: c.x,
                    open: c.open,
                    high: c.high,
                    low: c.low,
                    close: c.close,
                    graphics: c.common.graphics,
                    width: c.common.width,
                    height,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Tree(t) => {
            let df = pipeline::read_dataframe(&t.input)?;
            let (_, height) = pipeline::detected_terminal_size(t.common.width);
            let out = commands::render_treemap(
                &df,
                &commands::TreeOptions {
                    x: t.x,
                    y: t.y,
                    focus: t.common.focus,
                    annotate: t.common.annotate,
                    neutral: t.common.neutral,
                    no_takeaway: t.common.no_takeaway,
                    graphics: t.common.graphics,
                    width: t.common.width,
                    height,
                    palette_name: t.common.palette,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Violin(v) => {
            let df = pipeline::read_dataframe(&v.input)?;
            let (_, height) = pipeline::detected_terminal_size(v.common.width);
            let out = commands::render_violin(
                &df,
                &commands::ViolinOptions {
                    x: v.x,
                    y: v.y,
                    focus: v.common.focus,
                    annotate: v.common.annotate,
                    neutral: v.common.neutral,
                    no_takeaway: v.common.no_takeaway,
                    graphics: v.common.graphics,
                    width: v.common.width,
                    height,
                    palette_name: v.common.palette,
                },
            )?;
            print!("{out}");
            Ok(())
        }
        Command::Ridge(_r) => {
            // Wired in plan 11.5 task 6.
            anyhow::bail!("ridge command not yet wired");
        }
        Command::Doctor => {
            let report = commands::doctor::run()?;
            print!("{report}");
            Ok(())
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
