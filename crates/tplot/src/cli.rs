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
