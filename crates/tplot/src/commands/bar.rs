use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::{Column, DataFrame, Series};
use tplot_core::layout::{layout_horizontal_bar, layout_vertical_bar};
use tplot_core::rasterize::{rasterize_bar, rasterize_vertical};
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::{render_halfblocks, render_vertical_blocks};
use tplot_story::{SeriesPoint, run_bar_story_pass};

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub x: String,
    pub y: String,
    pub group: Option<String>,
    pub vertical: bool,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
    pub width: Option<usize>,
    /// Used directly in tests; in production callers pass the detected height.
    pub height: usize,
    pub palette_name: String,
}

pub fn render_bar(df: &DataFrame, opts: &RenderOptions) -> Result<String> {
    if opts.vertical {
        return render_vertical_bar(df, opts);
    }

    // ----- aggregate -------------------------------------------------------
    let group_col = opts.group.as_deref().unwrap_or(&opts.x);
    let labels: Vec<String> = match df
        .column(group_col)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df
        .column(&opts.y)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("y column `{}` must be numeric", opts.y)),
    };

    let mut series_points: Vec<SeriesPoint> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(p) = series_points.iter_mut().find(|p| p.key == *l) {
            p.value += v;
        } else {
            series_points.push(SeriesPoint {
                key: l.clone(),
                value: *v,
            });
        }
    }

    // ----- story-pass ------------------------------------------------------
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: match &opts.focus {
            Some(name) => FocusMode::Series(name.clone()),
            None => FocusMode::Auto,
        },
        annotation: opts.annotate.clone(),
    };
    let story = run_bar_story_pass(&series_points, &story_cfg, palette);

    // ----- layout ----------------------------------------------------------
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h = opts.height;

    let agg_df = DataFrame::from_columns(vec![
        Column::new(
            "__label__",
            Series::Strings(series_points.iter().map(|p| p.key.clone()).collect()),
        ),
        Column::new(
            "__value__",
            Series::Numbers(series_points.iter().map(|p| p.value).collect()),
        ),
    ])
    .map_err(|e| anyhow!(e.to_string()))?;
    let layout = layout_horizontal_bar(&agg_df, "__label__", "__value__", None, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- rasterize -------------------------------------------------------
    // Buffer holds ONLY the plot area. Margins are added by the composer.
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_bar(&layout, &story.palette_map, &mut buf);

    // ----- render to halfblocks (one cell row per source cell row) ---------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_halfblocks(&buf, caps);
    let bar_rows: Vec<&str> = body.lines().collect();

    // ----- compose ---------------------------------------------------------
    // For each bar's *top* cell row, build:
    //   "{label:>label_margin}  {colored_block}  {value:>value_margin-2}"
    // For other rows of the same bar, only the colored block is shown.
    let label_margin = layout.label_margin;
    let mut out = String::new();

    for (i, line) in bar_rows.iter().enumerate() {
        // Find the bar whose top cell row matches this row.
        let top_bar = layout.bars.iter().find(|b| (b.pixel_y / 2) == i);
        let any_bar = layout.bars.iter().find(|b| {
            let top = b.pixel_y / 2;
            let bottom = (b.pixel_y + b.pixel_height.saturating_sub(1)) / 2;
            i >= top && i <= bottom
        });

        if let Some(bar) = top_bar {
            // First (top) row of this bar: full ornament.
            out.push_str(&format!(
                "{:>w$}  ",
                bar.label,
                w = label_margin.saturating_sub(2)
            ));
            out.push_str(line);
            out.push_str(&format!("  {:.0}", bar.value));
        } else if any_bar.is_some() {
            // Continuation row of a multi-row bar (only happens when a bar
            // spans 2+ cell rows; in v1 each bar is exactly 1 cell tall, so
            // this branch is rarely hit, but kept for robustness).
            out.push_str(&" ".repeat(label_margin));
            out.push_str(line);
        } else {
            // Pure spacer row between bars.
            out.push_str(&" ".repeat(label_margin));
            out.push_str(line);
        }
        out.push('\n');
    }

    // ----- takeaway --------------------------------------------------------
    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(label_margin));
        out.push_str(&t);
        out.push('\n');
    }

    Ok(out)
}

fn render_vertical_bar(df: &DataFrame, opts: &RenderOptions) -> Result<String> {
    // ----- aggregate -------------------------------------------------------
    let group_col = opts.group.as_deref().unwrap_or(&opts.x);
    let labels: Vec<String> = match df
        .column(group_col)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Strings(v) => v.clone(),
        Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
    };
    let values: Vec<f64> = match df
        .column(&opts.y)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("y column `{}` must be numeric", opts.y)),
    };

    let mut series_points: Vec<SeriesPoint> = Vec::new();
    for (l, v) in labels.iter().zip(values.iter()) {
        if let Some(p) = series_points.iter_mut().find(|p| p.key == *l) {
            p.value += v;
        } else {
            series_points.push(SeriesPoint {
                key: l.clone(),
                value: *v,
            });
        }
    }

    // ----- story-pass ------------------------------------------------------
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let story_cfg = StoryConfig {
        enabled: !opts.neutral,
        takeaway: !opts.no_takeaway,
        focus: match &opts.focus {
            Some(name) => FocusMode::Series(name.clone()),
            None => FocusMode::Auto,
        },
        annotation: opts.annotate.clone(),
    };
    let story = run_bar_story_pass(&series_points, &story_cfg, palette);

    // ----- layout ----------------------------------------------------------
    let (canvas_w, _) = detected_terminal_size(opts.width);
    let canvas_h = opts.height;
    let agg_df = DataFrame::from_columns(vec![
        Column::new(
            "__label__",
            Series::Strings(series_points.iter().map(|p| p.key.clone()).collect()),
        ),
        Column::new(
            "__value__",
            Series::Numbers(series_points.iter().map(|p| p.value).collect()),
        ),
    ])
    .map_err(|e| anyhow!(e.to_string()))?;
    let layout = layout_vertical_bar(&agg_df, "__label__", "__value__", None, canvas_w, canvas_h)
        .map_err(|e| anyhow!(e.to_string()))?;

    // ----- rasterize -------------------------------------------------------
    let mut buf = PixelBuffer::new(layout.plot_box.pixel_width, layout.plot_box.pixel_height);
    rasterize_vertical(&layout, &story.palette_map, &mut buf);

    // ----- render to vertical-blocks ---------------------------------------
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let body = render_vertical_blocks(&buf, caps);
    let body_lines: Vec<&str> = body.lines().collect();

    // ----- compose ---------------------------------------------------------
    // Each chart row gets a y-axis label (left margin), then the rendered row.
    // Below the chart: x-axis labels (one per bar), centered under each bar.
    let mut out = String::new();
    let max_value = layout.bars.iter().map(|b| b.value).fold(f64::MIN, f64::max);
    let n_rows = body_lines.len();

    for (i, line) in body_lines.iter().enumerate() {
        // Y-axis label: print value at top, half-value mid, 0 at bottom.
        let y_label = if i == 0 {
            format!("{:>5.0}", max_value)
        } else if i == n_rows / 2 {
            format!("{:>5.0}", max_value / 2.0)
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

    // X-axis labels row: one label per bar column. Truncate labels that
    // are wider than the per-group budget so neighboring labels stay aligned.
    let group_w = layout.bar_cell_width + layout.gap_cell_width;
    let mut x_axis = String::new();
    x_axis.push_str(&" ".repeat(layout.left_margin));
    let leading = layout.bars.first().map(|b| b.pixel_x).unwrap_or(0);
    x_axis.push_str(&" ".repeat(leading));
    for (idx, bar) in layout.bars.iter().enumerate() {
        let max_label_w = if idx + 1 < layout.bars.len() {
            group_w
        } else {
            layout.bar_cell_width
        };
        let trimmed: String = bar.label.chars().take(max_label_w).collect();
        let pad_left = layout.bar_cell_width.saturating_sub(trimmed.chars().count()) / 2;
        let pad_right = layout
            .bar_cell_width
            .saturating_sub(trimmed.chars().count() + pad_left);
        x_axis.push_str(&" ".repeat(pad_left));
        x_axis.push_str(&trimmed);
        x_axis.push_str(&" ".repeat(pad_right));
        if idx + 1 < layout.bars.len() {
            x_axis.push_str(&" ".repeat(layout.gap_cell_width));
        }
    }
    out.push_str(&x_axis);
    out.push('\n');

    if let Some(t) = story.takeaway {
        out.push('\n');
        out.push_str(&" ".repeat(layout.left_margin + 1));
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
    fn renders_sales_csv_with_focal_emea() {
        let csv = include_str!("../../../../tests/fixtures/sales.csv");
        let df = parse_csv_str(csv).unwrap();
        let opts = RenderOptions {
            x: "quarter".into(),
            y: "revenue".into(),
            group: Some("region".into()),
            vertical: false,
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(80),
            height: 14,
            palette_name: "signature".into(),
        };
        let out = render_bar(&df, &opts).unwrap();
        // Should contain at least one truecolor escape and the takeaway.
        assert!(
            out.contains("\x1b[38;2;238;123;61m"),
            "missing focal color escape: {out:?}"
        );
        assert!(out.contains("EMEA"));
    }

    #[test]
    fn renders_vertical_bars_with_focal() {
        // Apr clearly dominates the median (17.9 → ratio > 1.5 trust threshold).
        let csv = "month,active\nJan,12.4\nFeb,13.1\nMar,18.2\nApr,40.0\nMay,17.9\nJun,9.0\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = RenderOptions {
            x: "month".into(),
            y: "active".into(),
            group: None,
            vertical: true,
            focus: None,
            annotate: None,
            neutral: false,
            no_takeaway: false,
            width: Some(60),
            height: 16,
            palette_name: "signature".into(),
        };
        let out = render_bar(&df, &opts).unwrap();
        // Apr is the max -> focal color (burnt orange) should appear.
        assert!(
            out.contains("\x1b[38;2;238;123;61m"),
            "missing focal color escape"
        );
        assert!(out.contains("Apr"));
        // Vertical bars use the lower-block glyphs.
        assert!(
            out.contains('\u{2588}') || out.contains('\u{2587}') || out.contains('\u{2585}'),
            "no lower-block glyphs in vertical bar output"
        );
    }

    #[test]
    fn neutral_mode_omits_takeaway_when_no_focal() {
        let csv = "x,y\na,50\nb,51\nc,49\nd,50.5\n";
        let df = parse_csv_str(csv).unwrap();
        let opts = RenderOptions {
            x: "x".into(),
            y: "y".into(),
            group: None,
            vertical: false,
            focus: None,
            annotate: None,
            neutral: true,
            no_takeaway: false,
            width: Some(60),
            height: 8,
            palette_name: "signature".into(),
        };
        let out = render_bar(&df, &opts).unwrap();
        assert!(!out.contains("standout"));
    }
}
