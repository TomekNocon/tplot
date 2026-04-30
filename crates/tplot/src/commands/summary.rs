use anyhow::{Result, anyhow};
use tplot_core::dataframe::{DataFrame, Series};
use tplot_core::stats::quantile;
use tplot_protocol::{Capabilities, Palette};
use tplot_render::ansi::{fg, reset};

#[derive(Debug, Clone, Default)]
pub struct SummaryOptions {
    pub x: Option<String>, // categorical column (categorical mode if present)
    pub y: String,         // numeric column (always required)
    pub top: Option<usize>,
    pub palette_name: String,
    pub annotate: Option<String>,
    pub neutral: bool,
}

const SPARK_WIDTH: usize = 12;
const GLYPHS: [char; 9] = [
    ' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
    '\u{2588}',
];

pub fn render_summary(df: &DataFrame, opts: &SummaryOptions) -> Result<String> {
    let palette = if opts.palette_name.is_empty() {
        Palette::Signature
    } else {
        Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?
    };

    if opts.neutral {
        // Neutral mode just prints the take-away annotation if present, or "ok".
        return Ok(opts.annotate.clone().unwrap_or_else(|| "(neutral)".into()) + "\n");
    }

    // Read the numeric y column.
    let values: Vec<f64> = match df
        .column(&opts.y)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("y column `{}` must be numeric", opts.y)),
    };

    let line = if let Some(x_col) = &opts.x {
        // Categorical mode: build (label, value) pairs.
        let labels: Vec<String> = match df
            .column(x_col)
            .map_err(|e| anyhow!(e.to_string()))?
            .series()
        {
            Series::Strings(v) => v.clone(),
            Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
        };
        if labels.len() != values.len() {
            return Err(anyhow!("x and y columns differ in length"));
        }
        // Aggregate duplicates by sum.
        let mut agg: Vec<(String, f64)> = Vec::new();
        for (l, v) in labels.iter().zip(values.iter()) {
            if let Some(slot) = agg.iter_mut().find(|(name, _)| name == l) {
                slot.1 += v;
            } else {
                agg.push((l.clone(), *v));
            }
        }
        render_categorical_summary(&agg, opts.top, palette)
    } else {
        render_sequence_summary(&values, palette)
    };

    Ok(format!("{line}\n"))
}

pub fn render_sequence_summary(values: &[f64], palette: Palette) -> String {
    if values.is_empty() {
        return "[0] (empty input)".into();
    }
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let n = values.len();
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let median = quantile(values, 0.5);

    // Build a SPARK_WIDTH-glyph mini-sparkline by resampling the values.
    let glyphs = sparkline(values, SPARK_WIDTH, min, max);

    // Find the position of the max for naming.
    let max_idx = values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let focal = palette.focal_color();
    format!(
        "[{n}] {open}{glyphs}{close} \u{2192} median={median:.0} max={max:.0} (#{idx})",
        open = fg(focal, caps.color_depth),
        close = reset(),
        idx = max_idx + 1,
    )
}

pub fn render_categorical_summary(
    pairs: &[(String, f64)],
    top: Option<usize>,
    palette: Palette,
) -> String {
    if pairs.is_empty() {
        return "[0 cats] (empty input)".into();
    }
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let n_total = pairs.len();

    // Sort descending, optionally truncate.
    let mut sorted: Vec<(String, f64)> = pairs.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let n_show = top.unwrap_or(sorted.len()).min(sorted.len());
    let shown = &sorted[..n_show];

    // Median for the trust-score check.
    let mut just_values: Vec<f64> = pairs.iter().map(|(_, v)| *v).collect();
    just_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = just_values[just_values.len() / 2];

    // Focal = max value if it dominates (max ≥ 1.5× median).
    let (focal_name, focal_value) = (&shown[0].0, shown[0].1);
    let trust = if median.abs() < 1e-9 {
        if focal_value > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        focal_value / median
    };
    let is_focal = trust >= 1.5;

    let focal_color = palette.focal_color();
    let context_color = palette.context_color_for(caps.theme);

    let mut parts = Vec::with_capacity(n_show);
    for (i, (label, value)) in shown.iter().enumerate() {
        let cell = if i == 0 && is_focal {
            format!(
                "{}{}({:.0}){}",
                fg(focal_color, caps.color_depth),
                label,
                value,
                reset()
            )
        } else {
            format!(
                "{}{}({:.0}){}",
                fg(context_color, caps.color_depth),
                label,
                value,
                reset()
            )
        };
        parts.push(cell);
    }
    let body = parts.join(" ");
    let suffix = if is_focal {
        format!(" \u{2014} {focal_name} {trust:.1}\u{00d7} median")
    } else {
        " \u{2014} values within \u{00b1}50% of median".into()
    };
    format!("[{n_total} cats] {body}{suffix}")
}

fn sparkline(values: &[f64], width: usize, min: f64, max: f64) -> String {
    if values.is_empty() || width == 0 {
        return String::new();
    }
    let span = (max - min).max(1e-9);
    let mut out = String::with_capacity(width);
    for i in 0..width {
        let t = if width <= 1 {
            0.0
        } else {
            i as f64 / (width - 1) as f64
        };
        let src_idx = (t * (values.len() - 1) as f64).round() as usize;
        let v = values[src_idx.min(values.len() - 1)];
        let frac = (v - min) / span;
        let glyph_idx = (frac * 8.0).round().clamp(0.0, 8.0) as usize;
        out.push(GLYPHS[glyph_idx]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tplot_protocol::Palette;

    #[test]
    fn sequence_summary_includes_count_and_glyphs() {
        let nums = vec![1.0, 3.0, 2.0, 5.0, 4.0, 7.0, 9.0, 8.0, 10.0, 6.0];
        let s = render_sequence_summary(&nums, Palette::Signature);
        assert!(s.contains("[10]"), "expected count: {s}");
        // At least one block-octant glyph present.
        assert!(s.chars().any(|c| matches!(
            c,
            '\u{2581}'
                | '\u{2582}'
                | '\u{2583}'
                | '\u{2584}'
                | '\u{2585}'
                | '\u{2586}'
                | '\u{2587}'
                | '\u{2588}'
        )));
        // Headline stats present.
        assert!(s.contains("max="));
        assert!(s.contains("median="));
        // Single-line guarantee.
        assert!(!s.contains('\n'), "summary must be single-line: {s:?}");
    }

    #[test]
    fn categorical_summary_lists_top_with_focal() {
        let pairs = vec![
            ("rust".to_string(), 1832.0),
            ("markdown".to_string(), 892.0),
            ("shell".to_string(), 541.0),
            ("yaml".to_string(), 128.0),
            ("toml".to_string(), 89.0),
        ];
        let s = render_categorical_summary(&pairs, None, Palette::Signature);
        assert!(s.contains("[5 cats]"));
        assert!(s.contains("rust"));
        assert!(s.contains("markdown"));
        // Focal color escape (burnt orange) appears.
        assert!(s.contains("\x1b[38;2;238;123;61m"));
        // Single line.
        assert!(!s.contains('\n'));
    }

    #[test]
    fn categorical_top_truncates_to_n() {
        let pairs = vec![
            ("a".into(), 100.0),
            ("b".into(), 90.0),
            ("c".into(), 80.0),
            ("d".into(), 70.0),
            ("e".into(), 60.0),
            ("f".into(), 50.0),
        ];
        let s = render_categorical_summary(&pairs, Some(3), Palette::Signature);
        // Only the top 3 should appear.
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.contains("c"));
        assert!(!s.contains("(50)"), "f's value should be truncated: {s}");
    }

    #[test]
    fn sparkline_glyphs_use_full_range() {
        // A monotonic sequence should hit several different glyphs.
        let nums: Vec<f64> = (1..=8).map(|x| x as f64).collect();
        let s = render_sequence_summary(&nums, Palette::Signature);
        // Should contain at least 3 distinct block glyphs.
        let distinct = s
            .chars()
            .filter(|c| {
                matches!(
                    c,
                    '\u{2581}'
                        | '\u{2582}'
                        | '\u{2583}'
                        | '\u{2584}'
                        | '\u{2585}'
                        | '\u{2586}'
                        | '\u{2587}'
                        | '\u{2588}'
                )
            })
            .collect::<std::collections::HashSet<_>>();
        assert!(
            distinct.len() >= 3,
            "expected ≥3 distinct glyphs: {distinct:?}"
        );
    }

    #[test]
    fn empty_input_returns_a_one_line_message() {
        let s = render_sequence_summary(&[], Palette::Signature);
        assert!(!s.contains('\n'));
        assert!(s.to_lowercase().contains("empty") || s.contains("[0]"));
    }
}
