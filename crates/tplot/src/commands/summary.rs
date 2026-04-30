use anyhow::{Result, anyhow};
use tplot_core::dataframe::{DataFrame, Series};
use tplot_core::stats::quantile;
use tplot_protocol::{Capabilities, Palette};
use tplot_render::ansi::{fg, reset};

#[derive(Debug, Clone, Default)]
pub struct SummaryOptions {
    pub x: Option<String>,
    pub y: String,
    pub top: Option<usize>,
    pub palette_name: String,
    pub focus: Option<String>,
    pub annotate: Option<String>,
    pub neutral: bool,
    pub no_takeaway: bool,
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

    let values: Vec<f64> = match df
        .column(&opts.y)
        .map_err(|e| anyhow!(e.to_string()))?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(anyhow!("y column `{}` must be numeric", opts.y)),
    };

    let line = if let Some(x_col) = &opts.x {
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
        let mut agg: Vec<(String, f64)> = Vec::new();
        for (l, v) in labels.iter().zip(values.iter()) {
            if let Some(slot) = agg.iter_mut().find(|(name, _)| name == l) {
                slot.1 += v;
            } else {
                agg.push((l.clone(), *v));
            }
        }
        render_categorical_summary(
            &agg,
            opts.top,
            palette,
            opts.focus.as_deref(),
            opts.annotate.as_deref(),
            opts.neutral,
            opts.no_takeaway,
        )
    } else {
        render_sequence_summary(
            &values,
            palette,
            opts.annotate.as_deref(),
            opts.neutral,
            opts.no_takeaway,
        )
    };

    Ok(format!("{line}\n"))
}

fn parse_focus_label(raw: Option<&str>) -> Option<&str> {
    raw.map(|s| s.split_once('=').map(|(_k, v)| v).unwrap_or(s))
}

pub fn render_sequence_summary(
    values: &[f64],
    palette: Palette,
    annotate: Option<&str>,
    neutral: bool,
    no_takeaway: bool,
) -> String {
    if values.is_empty() {
        return "[0] (empty input)".into();
    }
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let n = values.len();
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let median = quantile(values, 0.5);
    let glyphs = sparkline(values, SPARK_WIDTH, min, max);
    let max_idx = values
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    let color = if neutral {
        palette.context_color_for(caps.theme)
    } else {
        palette.focal_color()
    };
    let body = format!(
        "[{n}] {open}{glyphs}{close}",
        open = fg(color, caps.color_depth),
        close = reset(),
    );

    let suffix = takeaway_suffix(
        annotate,
        neutral,
        no_takeaway,
        || {
            format!(
                "median={median:.0} max={max:.0} (#{idx})",
                idx = max_idx + 1
            )
        },
        "\u{2192}",
    );
    format!("{body}{suffix}")
}

pub fn render_categorical_summary(
    pairs: &[(String, f64)],
    top: Option<usize>,
    palette: Palette,
    focus_label: Option<&str>,
    annotate: Option<&str>,
    neutral: bool,
    no_takeaway: bool,
) -> String {
    if pairs.is_empty() {
        return "[0 cats] (empty input)".into();
    }
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    let n_total = pairs.len();

    let mut sorted: Vec<(String, f64)> = pairs.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let n_show = top.unwrap_or(sorted.len()).min(sorted.len());
    let shown = &sorted[..n_show];

    let just_values: Vec<f64> = pairs.iter().map(|(_, v)| *v).collect();
    let median = quantile(&just_values, 0.5);

    let forced_focal_idx = parse_focus_label(focus_label).and_then(|target| {
        shown
            .iter()
            .position(|(label, _)| label.eq_ignore_ascii_case(target))
    });

    let (focal_idx, focal_name, focal_value, is_focal) = if neutral {
        (None, String::new(), 0.0, false)
    } else if let Some(idx) = forced_focal_idx {
        let (name, value) = (&shown[idx].0, shown[idx].1);
        (Some(idx), name.clone(), value, true)
    } else {
        let (name, value) = (&shown[0].0, shown[0].1);
        let trust = if median.abs() < 1e-9 {
            if value > 0.0 { f64::INFINITY } else { 0.0 }
        } else {
            value / median
        };
        let auto_is_focal = trust >= 1.5;
        (
            if auto_is_focal { Some(0) } else { None },
            name.clone(),
            value,
            auto_is_focal,
        )
    };

    let focal_color = palette.focal_color();
    let context_color = palette.context_color_for(caps.theme);

    let mut parts = Vec::with_capacity(n_show);
    for (i, (label, value)) in shown.iter().enumerate() {
        let color = if Some(i) == focal_idx {
            focal_color
        } else {
            context_color
        };
        parts.push(format!(
            "{}{}({:.0}){}",
            fg(color, caps.color_depth),
            label,
            value,
            reset()
        ));
    }
    let body = format!("[{n_total} cats] {}", parts.join(" "));

    let suffix = takeaway_suffix(
        annotate,
        neutral,
        no_takeaway,
        || {
            if !is_focal {
                "values within \u{00b1}50% of median".into()
            } else {
                let ratio = if median.abs() < 1e-9 {
                    f64::INFINITY
                } else {
                    focal_value / median
                };
                format!("{focal_name} {ratio:.1}\u{00d7} median")
            }
        },
        "\u{2014}",
    );
    format!("{body}{suffix}")
}

fn takeaway_suffix(
    annotate: Option<&str>,
    neutral: bool,
    no_takeaway: bool,
    auto: impl FnOnce() -> String,
    sep: &str,
) -> String {
    if no_takeaway {
        return String::new();
    }
    if let Some(text) = annotate {
        return format!(" {sep} {text}");
    }
    if neutral {
        return String::new();
    }
    format!(" {sep} {body}", body = auto())
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

    fn seq(values: &[f64]) -> String {
        render_sequence_summary(values, Palette::Signature, None, false, false)
    }

    fn cat(pairs: &[(String, f64)]) -> String {
        render_categorical_summary(pairs, None, Palette::Signature, None, None, false, false)
    }

    #[test]
    fn sequence_summary_includes_count_and_glyphs() {
        let nums = vec![1.0, 3.0, 2.0, 5.0, 4.0, 7.0, 9.0, 8.0, 10.0, 6.0];
        let s = seq(&nums);
        assert!(s.contains("[10]"), "expected count: {s}");
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
        assert!(s.contains("max="));
        assert!(s.contains("median="));
        assert!(!s.contains('\n'), "summary must be single-line: {s:?}");
    }

    #[test]
    fn categorical_summary_lists_top_with_focal() {
        let pairs = vec![
            ("rust".into(), 1832.0),
            ("markdown".into(), 892.0),
            ("shell".into(), 541.0),
            ("yaml".into(), 128.0),
            ("toml".into(), 89.0),
        ];
        let s = cat(&pairs);
        assert!(s.contains("[5 cats]"));
        assert!(s.contains("rust"));
        assert!(s.contains("markdown"));
        assert!(s.contains("\x1b[38;2;238;123;61m"));
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
        let s = render_categorical_summary(
            &pairs,
            Some(3),
            Palette::Signature,
            None,
            None,
            false,
            false,
        );
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert!(s.contains("c"));
        assert!(!s.contains("(50)"), "f's value should be truncated: {s}");
    }

    #[test]
    fn sparkline_glyphs_use_full_range() {
        let nums: Vec<f64> = (1..=8).map(|x| x as f64).collect();
        let s = seq(&nums);
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
        assert!(distinct.len() >= 3, "expected >=3 distinct: {distinct:?}");
    }

    #[test]
    fn empty_input_returns_a_one_line_message() {
        let s = render_sequence_summary(&[], Palette::Signature, None, false, false);
        assert!(!s.contains('\n'));
        assert!(s.to_lowercase().contains("empty") || s.contains("[0]"));
    }

    #[test]
    fn neutral_mode_renders_data_without_focal_or_takeaway() {
        let pairs = vec![
            ("alpha".into(), 142.0),
            ("beta".into(), 98.0),
            ("gamma".into(), 71.0),
            ("delta".into(), 45.0),
        ];
        let s =
            render_categorical_summary(&pairs, None, Palette::Signature, None, None, true, false);
        assert!(s.contains("alpha"), "data must still render: {s}");
        assert!(s.contains("beta"));
        assert!(
            !s.contains("\x1b[38;2;238;123;61m"),
            "neutral must drop focal color: {s:?}"
        );
        assert!(
            !s.contains("median") && !s.contains("\u{2014}"),
            "neutral must drop takeaway: {s:?}"
        );
        assert!(!s.contains('\n'));
    }

    #[test]
    fn neutral_sequence_drops_focal_and_takeaway() {
        let s = render_sequence_summary(
            &[1.0, 2.0, 3.0, 4.0, 5.0],
            Palette::Signature,
            None,
            true,
            false,
        );
        assert!(s.contains("[5]"));
        assert!(
            !s.contains("\x1b[38;2;238;123;61m"),
            "neutral must drop orange: {s:?}"
        );
        assert!(!s.contains("median="));
        assert!(!s.contains("max="));
    }

    #[test]
    fn focus_overrides_auto_focal_in_categorical() {
        let pairs = vec![
            ("alpha".into(), 142.0),
            ("beta".into(), 98.0),
            ("gamma".into(), 71.0),
            ("delta".into(), 45.0),
        ];
        let s = render_categorical_summary(
            &pairs,
            None,
            Palette::Signature,
            Some("delta"),
            None,
            false,
            false,
        );
        let focal = "\x1b[38;2;238;123;61m";
        let after = s.split(focal).nth(1).expect("focal escape present");
        assert!(
            after.starts_with("delta"),
            "delta should be in focal color: {s:?}"
        );
        assert!(s.contains("delta"));
        assert!(s.contains("\u{2014} delta"));
    }

    #[test]
    fn focus_accepts_col_equals_value_form() {
        let pairs = vec![("alpha".into(), 142.0), ("beta".into(), 98.0)];
        let a = render_categorical_summary(
            &pairs,
            None,
            Palette::Signature,
            Some("team=beta"),
            None,
            false,
            false,
        );
        let b = render_categorical_summary(
            &pairs,
            None,
            Palette::Signature,
            Some("beta"),
            None,
            false,
            false,
        );
        assert_eq!(a, b, "col=val and bare-label should yield identical output");
    }

    #[test]
    fn annotate_replaces_auto_takeaway() {
        let pairs = vec![("alpha".into(), 142.0), ("beta".into(), 30.0)];
        let s = render_categorical_summary(
            &pairs,
            None,
            Palette::Signature,
            None,
            Some("custom story"),
            false,
            false,
        );
        assert!(s.contains("\u{2014} custom story"));
        assert!(!s.contains("median"), "annotate replaces auto: {s:?}");
    }

    #[test]
    fn no_takeaway_strips_trailing_copy_but_keeps_focal() {
        let pairs = vec![("alpha".into(), 142.0), ("beta".into(), 30.0)];
        let s =
            render_categorical_summary(&pairs, None, Palette::Signature, None, None, false, true);
        assert!(
            s.contains("\x1b[38;2;238;123;61m"),
            "no-takeaway must keep focal: {s:?}"
        );
        assert!(
            !s.contains("\u{2014}"),
            "no-takeaway must drop suffix: {s:?}"
        );
        assert!(!s.contains("median"));
    }
}
