use crate::pipeline::detected_terminal_size;
use anyhow::{Result, anyhow};
use tplot_core::dataframe::Series;
use tplot_core::input::parse_csv_str;
use tplot_protocol::{Capabilities, Palette};
use tplot_render::ansi::{fg, reset};

const GLYPHS: [char; 9] = [
    ' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
    '\u{2588}',
];

#[derive(Debug, Clone)]
pub struct SparkOptions {
    pub input: String, // path or "-"
    pub y: Option<String>,
    pub palette_name: String,
    pub no_color: bool,
}

/// Parse the input bytes into a vector of f64. Smart-detects format:
/// - Has a comma followed by a non-numeric token → CSV, requires `y` column.
/// - Otherwise → whitespace + comma tokenized into f64.
pub fn parse_input_numbers(s: &str, y_col: Option<&str>) -> Result<Vec<f64>> {
    if s.trim().is_empty() {
        return Err(anyhow!("empty input"));
    }

    if looks_like_csv(s) {
        let col = y_col.ok_or_else(|| {
            anyhow!("input looks like CSV — pass `-y <column>` to pick the numeric column")
        })?;
        let df = parse_csv_str(s).map_err(|e| anyhow!(e.to_string()))?;
        let series = df.column(col).map_err(|e| anyhow!(e.to_string()))?.series();
        match series {
            Series::Numbers(v) => Ok(v.clone()),
            Series::Strings(_) => Err(anyhow!("column `{col}` must be numeric")),
        }
    } else {
        let mut nums = Vec::new();
        for tok in s.split(|c: char| c.is_whitespace() || c == ',') {
            let tok = tok.trim();
            if tok.is_empty() {
                continue;
            }
            if let Ok(v) = tok.parse::<f64>() {
                nums.push(v);
            }
        }
        if nums.is_empty() {
            return Err(anyhow!("no parseable numeric values in input"));
        }
        Ok(nums)
    }
}

/// Looks like a CSV if the first non-empty line contains a comma AND at
/// least one non-numeric token between commas (i.e., a header row).
fn looks_like_csv(s: &str) -> bool {
    let first = match s.lines().find(|l| !l.trim().is_empty()) {
        Some(l) => l,
        None => return false,
    };
    if !first.contains(',') {
        return false;
    }
    first.split(',').any(|t| {
        let t = t.trim();
        !t.is_empty() && t.parse::<f64>().is_err()
    })
}

/// Render a slice of f64 as a glyph-only string (no escape codes).
pub fn render_glyphs(nums: &[f64]) -> String {
    if nums.is_empty() {
        return String::new();
    }
    let min = nums.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = nums.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let span = (max - min).max(1e-9);
    nums.iter()
        .map(|v| {
            let idx = if (max - min).abs() < 1e-9 {
                4
            } else {
                (((v - min) / span) * 8.0).round().clamp(0.0, 8.0) as usize
            };
            GLYPHS[idx]
        })
        .collect()
}

pub fn render_sparkline(input: &str, opts: &SparkOptions) -> Result<String> {
    let nums = parse_input_numbers(input, opts.y.as_deref())?;
    let glyphs = render_glyphs(&nums);

    if opts.no_color {
        return Ok(format!("{glyphs}\n"));
    }

    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    Ok(format!(
        "{}{glyphs}{}\n",
        fg(palette.focal_color(), caps.color_depth),
        reset()
    ))
}

/// Convenience helper for callers that have already loaded the input bytes.
#[allow(dead_code)]
pub fn render_sparkline_from_numbers(nums: &[f64], opts: &SparkOptions) -> Result<String> {
    let _ = detected_terminal_size(None); // capability-detect side effect parity
    let glyphs = render_glyphs(nums);
    if opts.no_color {
        return Ok(format!("{glyphs}\n"));
    }
    let palette = Palette::from_name(&opts.palette_name).map_err(|e| anyhow!(e.to_string()))?;
    let caps = Capabilities::from_vars(|name| std::env::var(name).ok());
    Ok(format!(
        "{}{glyphs}{}\n",
        fg(palette.focal_color(), caps.color_depth),
        reset()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whitespace_separated_numbers() {
        let nums = parse_input_numbers("1 3 2 5 4 7", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0, 4.0, 7.0]);
    }

    #[test]
    fn parses_newline_separated_numbers() {
        let nums = parse_input_numbers("1\n3\n2\n5\n", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0]);
    }

    #[test]
    fn parses_comma_separated_numbers() {
        let nums = parse_input_numbers("1, 3, 2, 5", None).unwrap();
        assert_eq!(nums, vec![1.0, 3.0, 2.0, 5.0]);
    }

    #[test]
    fn parses_csv_column_when_y_given() {
        let csv = "t,latency\n1,42\n2,58\n3,71\n";
        let nums = parse_input_numbers(csv, Some("latency")).unwrap();
        assert_eq!(nums, vec![42.0, 58.0, 71.0]);
    }

    #[test]
    fn rejects_csv_without_y_column() {
        let csv = "t,latency\n1,42\n2,58\n";
        assert!(parse_input_numbers(csv, None).is_err());
    }

    #[test]
    fn glyph_for_full_range() {
        // 9 evenly-spaced values produce all 9 glyphs in order.
        let nums: Vec<f64> = (0..9).map(|i| i as f64).collect();
        let s = render_glyphs(&nums);
        assert!(s.contains('\u{2581}')); // ▁
        assert!(s.contains('\u{2584}')); // ▄
        assert!(s.contains('\u{2588}')); // █
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(parse_input_numbers("", None).is_err());
    }

    #[test]
    fn single_value_renders_a_mid_glyph() {
        let s = render_glyphs(&[42.0]);
        // Single point → no range; render mid-glyph (4/8 = ▄).
        assert!(s.contains('\u{2584}'));
    }
}
