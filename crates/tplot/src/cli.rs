use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "tplot",
    version,
    about = "Storytelling-first chart engine for the terminal",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Render a bar chart from a CSV/JSON file.
    Bar(BarArgs),
    /// Render a histogram of a single numeric column.
    Hist(HistArgs),
    /// Render a line chart (time-series friendly).
    Line(LineArgs),
    /// Render a scatter plot.
    Scatter(ScatterArgs),
    /// Render a one-line sparkline from a column or whitespace-separated numbers.
    Spark(SparkArgs),
    /// Render a 2D heatmap from long-form (x, y, value) data.
    Heatmap(HeatmapArgs),
    /// Render a vertical box plot — one column per group of the `x` column,
    /// 5-number summary of the `y` column.
    Box(BoxArgs),
    /// Render a stacked-area chart. Each group fills from the cumulative
    /// baseline up to its cumulative top, in first-seen order.
    Area(AreaArgs),
    /// Probe the terminal and print a capability report.
    Doctor,
    /// Read a JSON ChartSpec from stdin and render it.
    Json,
}

#[derive(Args, Debug)]
pub struct BarArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Column for x-axis labels.
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column for y-axis values.
    #[arg(short = 'y')]
    pub y: String,
    /// Optional grouping column.
    #[arg(long)]
    pub group: Option<String>,
    /// Vertical bars instead of horizontal.
    #[arg(long)]
    pub vertical: bool,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct HistArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Numeric column to bin.
    #[arg(short = 'x')]
    pub x: String,
    /// Bin count. Default: Sturges' rule (ceil(log2(N)+1)).
    #[arg(long)]
    pub bins: Option<usize>,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct LineArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Numeric column for x-axis (e.g., time).
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column for y-axis (e.g., metric value).
    #[arg(short = 'y')]
    pub y: String,
    /// Optional grouping column splitting into multiple series.
    #[arg(long)]
    pub group: Option<String>,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct ScatterArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Numeric column for x-axis.
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column for y-axis.
    #[arg(short = 'y')]
    pub y: String,
    /// Optional grouping column splitting into multiple series.
    #[arg(long)]
    pub group: Option<String>,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct SparkArgs {
    /// Path to input. Use `-` for stdin. CSV with -y, or whitespace-separated numbers.
    pub input: String,
    /// Column name to extract (only meaningful for CSV input).
    #[arg(short = 'y')]
    pub y: Option<String>,
    /// Color palette: signature | editorial | colorblind-safe.
    #[arg(long, default_value = "signature")]
    pub palette: String,
    /// Skip color (output plain glyphs only).
    #[arg(long)]
    pub no_color: bool,
}

#[derive(Args, Debug)]
pub struct HeatmapArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// X-axis (categorical) column.
    #[arg(short = 'x')]
    pub x: String,
    /// Y-axis (categorical) column.
    #[arg(short = 'y')]
    pub y: String,
    /// Numeric value column for color intensity.
    #[arg(long)]
    pub value: String,
    /// Heat ramp: inferno (default) | viridis | coolwarm.
    #[arg(long, default_value = "inferno")]
    pub ramp: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct BoxArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Categorical column for grouping (one box per unique value).
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric column for the value distribution.
    #[arg(short = 'y')]
    pub y: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug)]
pub struct AreaArgs {
    /// Path to CSV or JSON input. Use `-` for stdin.
    pub input: String,
    /// Numeric x-axis column (typically time).
    #[arg(short = 'x')]
    pub x: String,
    /// Numeric y-axis column (the values to stack).
    #[arg(short = 'y')]
    pub y: String,
    /// Required: column whose unique values form the stacked series.
    #[arg(long)]
    pub group: String,
    #[command(flatten)]
    pub common: CommonStoryArgs,
}

#[derive(Args, Debug, Clone)]
pub struct CommonStoryArgs {
    /// Override auto-detected focal series.
    #[arg(long)]
    pub focus: Option<String>,
    /// Replace the auto-generated takeaway with this text.
    #[arg(long)]
    pub annotate: Option<String>,
    /// Skip the storytelling pass entirely.
    #[arg(long)]
    pub neutral: bool,
    /// Suppress the takeaway line (keep story styling).
    #[arg(long = "no-takeaway")]
    pub no_takeaway: bool,
    /// Force terminal width (default: detected).
    #[arg(long)]
    pub width: Option<usize>,
    /// Color palette: signature | editorial | colorblind-safe.
    #[arg(long, default_value = "signature")]
    pub palette: String,
    /// Render the chart via a terminal graphics protocol when supported.
    /// One of: `auto` (pick the best detected), `kitty`, `iterm2`, `none`.
    #[arg(long, default_value = "none")]
    pub graphics: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_bar_with_xy() {
        let args = Cli::parse_from([
            "tplot",
            "bar",
            "sales.csv",
            "-x",
            "quarter",
            "-y",
            "revenue",
            "--group",
            "region",
        ]);
        match args.command {
            Command::Bar(b) => {
                assert_eq!(b.input, "sales.csv");
                assert_eq!(b.x, "quarter");
                assert_eq!(b.y, "revenue");
                assert_eq!(b.group.as_deref(), Some("region"));
            }
            _ => panic!("expected Bar"),
        }
    }

    #[test]
    fn parses_hist_subcommand() {
        let args = Cli::parse_from(["tplot", "hist", "latencies.csv", "-x", "ms", "--bins", "20"]);
        match args.command {
            Command::Hist(h) => {
                assert_eq!(h.input, "latencies.csv");
                assert_eq!(h.x, "ms");
                assert_eq!(h.bins, Some(20));
            }
            _ => panic!("expected Hist"),
        }
    }

    #[test]
    fn parses_line_subcommand() {
        let args = Cli::parse_from([
            "tplot",
            "line",
            "metrics.csv",
            "-x",
            "time",
            "-y",
            "value",
            "--group",
            "series",
        ]);
        match args.command {
            Command::Line(l) => {
                assert_eq!(l.input, "metrics.csv");
                assert_eq!(l.x, "time");
                assert_eq!(l.y, "value");
                assert_eq!(l.group.as_deref(), Some("series"));
            }
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn parses_scatter_subcommand() {
        let args = Cli::parse_from([
            "tplot",
            "scatter",
            "users.csv",
            "-x",
            "signup_age",
            "-y",
            "session_count",
        ]);
        match args.command {
            Command::Scatter(s) => {
                assert_eq!(s.input, "users.csv");
                assert_eq!(s.x, "signup_age");
                assert_eq!(s.y, "session_count");
            }
            _ => panic!("expected Scatter"),
        }
    }

    #[test]
    fn parses_spark_subcommand() {
        let args = Cli::parse_from(["tplot", "spark", "metrics.csv", "-y", "latency"]);
        match args.command {
            Command::Spark(s) => {
                assert_eq!(s.input, "metrics.csv");
                assert_eq!(s.y.as_deref(), Some("latency"));
                assert_eq!(s.palette, "signature");
            }
            _ => panic!("expected Spark"),
        }
    }

    #[test]
    fn parses_spark_from_stdin() {
        let args = Cli::parse_from(["tplot", "spark", "-"]);
        match args.command {
            Command::Spark(s) => {
                assert_eq!(s.input, "-");
                assert!(s.y.is_none());
            }
            _ => panic!("expected Spark"),
        }
    }

    #[test]
    fn parses_heatmap_subcommand() {
        let args = Cli::parse_from([
            "tplot",
            "heatmap",
            "metrics.csv",
            "-x",
            "hour",
            "-y",
            "day",
            "--value",
            "count",
            "--ramp",
            "viridis",
        ]);
        match args.command {
            Command::Heatmap(h) => {
                assert_eq!(h.input, "metrics.csv");
                assert_eq!(h.x, "hour");
                assert_eq!(h.y, "day");
                assert_eq!(h.value, "count");
                assert_eq!(h.ramp, "viridis");
            }
            _ => panic!("expected Heatmap"),
        }
    }

    #[test]
    fn parses_box_subcommand() {
        let args = Cli::parse_from([
            "tplot",
            "box",
            "metrics.csv",
            "-x",
            "endpoint",
            "-y",
            "latency_ms",
        ]);
        match args.command {
            Command::Box(b) => {
                assert_eq!(b.input, "metrics.csv");
                assert_eq!(b.x, "endpoint");
                assert_eq!(b.y, "latency_ms");
            }
            _ => panic!("expected Box"),
        }
    }

    #[test]
    fn parses_area_subcommand() {
        let args = Cli::parse_from([
            "tplot",
            "area",
            "metrics.csv",
            "-x",
            "month",
            "-y",
            "revenue",
            "--group",
            "region",
        ]);
        match args.command {
            Command::Area(a) => {
                assert_eq!(a.input, "metrics.csv");
                assert_eq!(a.x, "month");
                assert_eq!(a.y, "revenue");
                assert_eq!(a.group, "region");
            }
            _ => panic!("expected Area"),
        }
    }

    #[test]
    fn parses_doctor_subcommand() {
        let args = Cli::parse_from(["tplot", "doctor"]);
        assert!(matches!(args.command, Command::Doctor));
    }

    #[test]
    fn parses_graphics_flag() {
        let args = Cli::parse_from([
            "tplot",
            "bar",
            "sales.csv",
            "-x",
            "quarter",
            "-y",
            "revenue",
            "--graphics",
            "kitty",
        ]);
        match args.command {
            Command::Bar(b) => assert_eq!(b.common.graphics, "kitty"),
            _ => panic!(),
        }
    }

    #[test]
    fn graphics_flag_defaults_to_none() {
        let args = Cli::parse_from([
            "tplot",
            "bar",
            "sales.csv",
            "-x",
            "quarter",
            "-y",
            "revenue",
        ]);
        match args.command {
            Command::Bar(b) => assert_eq!(b.common.graphics, "none"),
            _ => panic!(),
        }
    }

    #[test]
    fn neutral_flag_propagates() {
        let args = Cli::parse_from([
            "tplot",
            "bar",
            "sales.csv",
            "-x",
            "q",
            "-y",
            "r",
            "--neutral",
        ]);
        match args.command {
            Command::Bar(b) => assert!(b.common.neutral),
            _ => panic!(),
        }
    }
}
