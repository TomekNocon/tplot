use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::DataFrame;
use tplot_core::layout::layout_histogram;
use tplot_core::rasterize::rasterize_vertical;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::render_vertical_blocks;
use tplot_story::{SeriesPoint, run_histogram_story_pass};

#[derive(Debug, Clone)]
pub struct HistogramOptions {
    pub x: String,
    pub bins: Option<usize>,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    pub height: usize,
    pub palette_name: String,
}

pub fn render_histogram(df: &DataFrame, opts: &HistogramOptions) -> Result<String> {
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h = opts.height;

    // Compute the histogram layout (this also produces the per-bin labels +
    // counts as a synthetic vertical bar layout).
    let hist = layout_histogram(df, &opts.x, opts.bins, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // Run the story-pass over the bins.
    let bins_as_points: Vec<SeriesPoint> = hist
        .bars
        .bars
        .iter()
        .map(|b| SeriesPoint {
            key: b.label.clone(),
            value: b.value,
        })
        .collect();
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: match &opts.focus {
            Some(name) => FocusMode::Series(name.clone()),
            None => FocusMode::Auto,
        },
        annotation: opts.annotate.clone(),
    };
    let story = run_histogram_story_pass(&bins_as_points, &story_cfg, palette);

    // Rasterize.
    let mut buf = PixelBuffer::new(
        hist.bars.plot_box.pixel_width,
        hist.bars.plot_box.pixel_height,
    );
    rasterize_vertical(&hist.bars, &story.palette_map, &mut buf);

    // Render via vertical-block glyphs.
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_vertical_blocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // Compose: y-axis labels (max count, half, 0), then bar labels under bars.
    let mut out = String::new();
    let max_count = hist
        .bars
        .bars
        .iter()
        .map(|b| b.value)
        .fold(f64::MIN, f64::max);
    let n_rows = body_lines.len();

    for (i, line) in body_lines.iter().enumerate() {
        let y_label = if i == 0 {
            format!("{:>5.0}", max_count)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", max_count / 2.0)
        } else if i + 1 == n_rows {
            format!("{:>5}", "0")
        } else {
            " ".repeat(5)
        };
        out.push_str(&y_label);
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }

    // Bin-range labels under the bars.
    let group_w = hist.bars.bar_cell_width + hist.bars.gap_cell_width;
    out.push_str(&" ".repeat(hist.bars.left_margin));
    let leading = hist.bars.bars.first().map(|b| b.pixel_x).unwrap_or(0);
    out.push_str(&" ".repeat(leading));
    for (idx, bar) in hist.bars.bars.iter().enumerate() {
        let trimmed: String = bar.label.chars().take(group_w.saturating_sub(1)).collect();
        let pad_left = hist
            .bars
            .bar_cell_width
            .saturating_sub(trimmed.chars().count())
            / 2;
        let pad_right = hist
            .bars
            .bar_cell_width
            .saturating_sub(trimmed.chars().count() + pad_left);
        out.push_str(&" ".repeat(pad_left));
        out.push_str(&trimmed);
        out.push_str(&" ".repeat(pad_right));
        if idx + 1 < hist.bars.bars.len() {
            out.push_str(&" ".repeat(hist.bars.gap_cell_width));
        }
    }
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(hist.bars.left_margin + 1));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_core::input::parse_csv_str;

    #[test]
    fn renders_histogram_with_focal_modal_bin() {
        // Synthetic latency data: clear peak in 40-60ms range.
        let csv =
            "ms\n10\n22\n35\n41\n48\n49\n50\n50\n51\n52\n55\n58\n60\n65\n80\n95\n110\n145\n220\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = HistogramOptions {
            x: "ms".into(),
            bins: Some(7),
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_histogram(&df, &opts).unwrap();
        assert!(out.contains("\x1b[38;2;238;123;61m"), "no focal color");
        assert!(out.contains("Most observations clustered in"));
    }
}
