use crate::dataframe::{DataFrame, Series};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Text,
    Number,
    Boolean,
}

#[derive(Debug, Clone)]
pub struct TableColumn {
    pub name: String,
    pub kind: ColumnType,
    /// Numeric column max (used for inline bars). None for non-numeric.
    pub max_numeric: Option<f64>,
    /// Numeric column min. None for non-numeric.
    pub min_numeric: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct TableLayoutConfig {
    /// Column to render an inline value bar for.
    pub bars: Option<String>,
    /// Column to sort by (descending).
    pub sort: Option<String>,
    /// Keep only the first N rows after sorting.
    pub top: Option<usize>,
    /// Override the auto-detected focal row.
    pub focus_text: Option<(String, String)>, // (column, value) pair
}

#[derive(Debug, Clone)]
pub struct TableLayout {
    pub columns: Vec<TableColumn>,
    /// `rows[row_idx][col_idx]` = formatted display string.
    pub rows: Vec<Vec<String>>,
    /// Numeric values for the bars column, parallel to `rows`. None if no
    /// bars column was specified or the column isn't numeric.
    pub bars_values: Option<Vec<f64>>,
    /// Index of the bars column within `columns`, if present.
    pub bars_col_idx: Option<usize>,
    /// Index of the focal row (story-pass output).
    pub focal_row: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum TableLayoutError {
    #[error("no data rows")]
    Empty,
    #[error(transparent)]
    DataFrame(#[from] crate::dataframe::DataFrameError),
    #[error("sort column `{0}` must be numeric")]
    NonNumericSort(String),
}

pub fn layout_table(
    df: &DataFrame,
    cfg: TableLayoutConfig,
) -> Result<TableLayout, TableLayoutError> {
    if df.nrows() == 0 {
        return Err(TableLayoutError::Empty);
    }

    // Detect column types and compute min/max for numeric columns.
    let mut columns = Vec::with_capacity(df.columns().len());
    for col in df.columns() {
        match col.series() {
            Series::Numbers(v) => {
                let min = v.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max = v.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                columns.push(TableColumn {
                    name: col.name().to_string(),
                    kind: ColumnType::Number,
                    max_numeric: Some(max),
                    min_numeric: Some(min),
                });
            }
            Series::Strings(v) => {
                let kind = if v.iter().all(|s| is_boolean_like(s)) {
                    ColumnType::Boolean
                } else {
                    ColumnType::Text
                };
                columns.push(TableColumn {
                    name: col.name().to_string(),
                    kind,
                    max_numeric: None,
                    min_numeric: None,
                });
            }
        }
    }

    // Build all rows as a Vec of (row_idx → Vec<String>).
    let mut row_indices: Vec<usize> = (0..df.nrows()).collect();

    // Sort by column if requested.
    if let Some(sort_name) = &cfg.sort {
        let sort_col_idx = df
            .columns()
            .iter()
            .position(|c| c.name() == sort_name)
            .ok_or_else(|| TableLayoutError::NonNumericSort(sort_name.clone()))?;
        let series = df.columns()[sort_col_idx].series();
        let values: Vec<f64> = match series {
            Series::Numbers(v) => v.clone(),
            Series::Strings(_) => return Err(TableLayoutError::NonNumericSort(sort_name.clone())),
        };
        row_indices.sort_by(|&a, &b| {
            values[b]
                .partial_cmp(&values[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Top-N filter.
    if let Some(n) = cfg.top {
        row_indices.truncate(n);
    }

    // Format every row.
    let rows: Vec<Vec<String>> = row_indices
        .iter()
        .map(|&ri| {
            df.columns()
                .iter()
                .enumerate()
                .map(|(ci, col)| format_cell(col.series(), ri, columns[ci].kind))
                .collect()
        })
        .collect();

    // Determine bars column + values.
    let (bars_col_idx, bars_values) = if let Some(bars_name) = &cfg.bars {
        let idx = df.columns().iter().position(|c| c.name() == bars_name);
        match idx {
            Some(i) => match df.columns()[i].series() {
                Series::Numbers(v) => {
                    let filtered: Vec<f64> = row_indices.iter().map(|&ri| v[ri]).collect();
                    (Some(i), Some(filtered))
                }
                _ => (Some(i), None),
            },
            None => (None, None),
        }
    } else {
        (None, None)
    };

    // Focal row: max in bars column, or via focus_text override.
    let focal_row = compute_focal_row(&columns, &rows, &bars_values, bars_col_idx, &cfg);

    Ok(TableLayout {
        columns,
        rows,
        bars_values,
        bars_col_idx,
        focal_row,
    })
}

fn is_boolean_like(s: &str) -> bool {
    matches!(
        s.trim().to_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "1" | "0" | "y" | "n" | "t" | "f"
    )
}

fn format_cell(series: &Series, idx: usize, kind: ColumnType) -> String {
    match (series, kind) {
        (Series::Numbers(v), _) => format_number(v[idx]),
        (Series::Strings(v), ColumnType::Boolean) => match v[idx].trim().to_lowercase().as_str() {
            "true" | "yes" | "1" | "y" | "t" => "✓".into(),
            _ => "✗".into(),
        },
        (Series::Strings(v), _) => v[idx].clone(),
    }
}

/// Format a number with thousands separator. Whole numbers get no decimal;
/// fractional values get 2 decimals.
fn format_number(v: f64) -> String {
    let abs = v.abs();
    let is_int = (v - v.round()).abs() < 1e-9 && abs < 1e15;
    if is_int {
        let n = v.round() as i64;
        with_thousands(n)
    } else if abs >= 1e9 {
        format!("{:.2e}", v)
    } else {
        format!("{:.2}", v)
    }
}

fn with_thousands(mut n: i64) -> String {
    let neg = n < 0;
    if neg {
        n = -n;
    }
    let s = n.to_string();
    let chars: Vec<char> = s.chars().rev().collect();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    let result: String = out.chars().rev().collect();
    if neg { format!("-{result}") } else { result }
}

fn compute_focal_row(
    columns: &[TableColumn],
    rows: &[Vec<String>],
    bars_values: &Option<Vec<f64>>,
    bars_col_idx: Option<usize>,
    cfg: &TableLayoutConfig,
) -> Option<usize> {
    // 1. Explicit focus_text wins.
    if let Some((col_name, value)) = &cfg.focus_text {
        let ci = columns.iter().position(|c| &c.name == col_name)?;
        return rows.iter().position(|r| r[ci] == *value);
    }
    // 2. Otherwise: max in bars column with the standard trust-score gate.
    if let (Some(values), Some(_)) = (bars_values, bars_col_idx) {
        if values.is_empty() {
            return None;
        }
        let mut sorted: Vec<f64> = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let max_idx = values
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)?;
        let trust = if median.abs() < 1e-9 {
            if values[max_idx] > 0.0 {
                f64::INFINITY
            } else {
                0.0
            }
        } else {
            values[max_idx] / median
        };
        if trust >= 1.5 { Some(max_idx) } else { None }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn sales_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new(
                "region",
                Series::Strings(
                    vec!["NA", "EMEA", "LATAM", "APAC", "AU"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new(
                "revenue",
                Series::Numbers(vec![179.0, 193.0, 78.0, 97.0, 53.0]),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn detects_text_and_numeric_columns() {
        let layout = layout_table(&sales_df(), TableLayoutConfig::default()).unwrap();
        assert_eq!(layout.columns.len(), 2);
        assert_eq!(layout.columns[0].name, "region");
        assert!(matches!(layout.columns[0].kind, ColumnType::Text));
        assert_eq!(layout.columns[1].name, "revenue");
        assert!(matches!(layout.columns[1].kind, ColumnType::Number));
    }

    #[test]
    fn formats_numbers_with_thousands_separator() {
        let df = DataFrame::from_columns(vec![Column::new(
            "n",
            Series::Numbers(vec![1234567.0, 1000.0, 1.5]),
        )])
        .unwrap();
        let layout = layout_table(&df, TableLayoutConfig::default()).unwrap();
        // Row 0 = 1234567.0 → "1,234,567"
        assert_eq!(layout.rows[0][0], "1,234,567");
        assert_eq!(layout.rows[1][0], "1,000");
        // Floats with fraction → "1.50"
        assert_eq!(layout.rows[2][0], "1.50");
    }

    #[test]
    fn sort_descending_by_column() {
        let cfg = TableLayoutConfig {
            sort: Some("revenue".into()),
            ..Default::default()
        };
        let layout = layout_table(&sales_df(), cfg).unwrap();
        // EMEA (193) should be first, AU (53) last.
        assert_eq!(layout.rows[0][0], "EMEA");
        assert_eq!(layout.rows[layout.rows.len() - 1][0], "AU");
    }

    #[test]
    fn top_filter_keeps_first_n_after_sort() {
        let cfg = TableLayoutConfig {
            sort: Some("revenue".into()),
            top: Some(2),
            ..Default::default()
        };
        let layout = layout_table(&sales_df(), cfg).unwrap();
        assert_eq!(layout.rows.len(), 2);
        assert_eq!(layout.rows[0][0], "EMEA");
        assert_eq!(layout.rows[1][0], "NA");
    }

    #[test]
    fn focal_row_is_max_of_bars_column() {
        let cfg = TableLayoutConfig {
            bars: Some("revenue".into()),
            ..Default::default()
        };
        let layout = layout_table(&sales_df(), cfg).unwrap();
        assert!(layout.focal_row.is_some());
        // EMEA has the max revenue → its row index should be focal.
        let emea_row = layout.rows.iter().position(|r| r[0] == "EMEA").unwrap();
        assert_eq!(layout.focal_row, Some(emea_row));
    }

    #[test]
    fn boolean_column_renders_as_check_or_cross() {
        let df = DataFrame::from_columns(vec![Column::new(
            "active",
            Series::Strings(
                vec!["true", "false", "yes", "no"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            ),
        )])
        .unwrap();
        let layout = layout_table(&df, TableLayoutConfig::default()).unwrap();
        assert!(matches!(layout.columns[0].kind, ColumnType::Boolean));
        assert_eq!(layout.rows[0][0], "✓");
        assert_eq!(layout.rows[1][0], "✗");
        assert_eq!(layout.rows[2][0], "✓");
        assert_eq!(layout.rows[3][0], "✗");
    }
}
