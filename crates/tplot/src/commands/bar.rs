use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::PixelBuffer;
use tplot_core::dataframe::{Column, DataFrame, Series};
use tplot_core::layout::layout_horizontal_bar;
use tplot_core::rasterize::rasterize_bar;
use tplot_protocol::{Capabilities, FocusMode, Palette, StoryConfig};
use tplot_render::render_halfblocks;
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
        return Err(anyhow!("vertical bars land in plan 2"));
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
