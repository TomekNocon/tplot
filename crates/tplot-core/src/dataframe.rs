#[derive(Debug, Clone, PartialEq)]
pub enum Series {
    Numbers(Vec<f64>),
    Strings(Vec<String>),
}

impl Series {
    pub fn len(&self) -> usize {
        match self {
            Series::Numbers(v) => v.len(),
            Series::Strings(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: String,
    series: Series,
}

impl Column {
    pub fn new(name: impl Into<String>, series: Series) -> Self {
        Self {
            name: name.into(),
            series,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn series(&self) -> &Series {
        &self.series
    }
    pub fn len(&self) -> usize {
        self.series.len()
    }
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataFrame {
    columns: Vec<Column>,
}

#[derive(Debug, thiserror::Error)]
pub enum DataFrameError {
    #[error("column lengths differ: {0}")]
    MismatchedLengths(String),
    #[error("unknown column `{name}`{}",
        if let Some(s) = suggestion { format!(" — did you mean `{s}`?") } else { String::new() })]
    UnknownColumn {
        name: String,
        suggestion: Option<String>,
    },
}

impl DataFrame {
    pub fn from_columns(columns: Vec<Column>) -> Result<Self, DataFrameError> {
        if let Some(first) = columns.first() {
            let n = first.len();
            for c in &columns[1..] {
                if c.len() != n {
                    return Err(DataFrameError::MismatchedLengths(format!(
                        "`{}` has {} rows, `{}` has {}",
                        first.name(),
                        n,
                        c.name(),
                        c.len()
                    )));
                }
            }
        }
        Ok(Self { columns })
    }

    pub fn column(&self, name: &str) -> Result<&Column, DataFrameError> {
        if let Some(c) = self.columns.iter().find(|c| c.name() == name) {
            Ok(c)
        } else {
            // Cheap Levenshtein-ish suggestion: pick the first column whose
            // first 3 chars overlap with the requested name.
            let suggestion = self
                .columns
                .iter()
                .map(|c| c.name())
                .min_by_key(|n| levenshtein(n, name))
                .map(String::from);
            Err(DataFrameError::UnknownColumn {
                name: name.to_string(),
                suggestion,
            })
        }
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
    pub fn nrows(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Names of all columns whose `Series` is `Numbers`.
    pub fn numeric_columns(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|c| matches!(c.series(), Series::Numbers(_)))
            .map(|c| c.name())
            .collect()
    }
}

/// Format a list of column names as a comma-separated, backtick-quoted string.
/// Returns "(none)" for an empty list.
pub fn comma_list(names: Vec<&str>) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names
            .iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new(
                "quarter",
                Series::Strings(
                    vec!["Q1", "Q2", "Q3", "Q4"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
            Column::new("revenue", Series::Numbers(vec![42.0, 58.0, 71.0, 52.0])),
        ])
        .unwrap()
    }

    #[test]
    fn column_lookup_by_name() {
        let df = sample();
        let col = df.column("revenue").unwrap();
        assert_eq!(col.name(), "revenue");
        assert!(matches!(col.series(), Series::Numbers(_)));
    }

    #[test]
    fn column_missing_reports_options() {
        let df = sample();
        let err = df.column("revneue").unwrap_err();
        // Did-you-mean lists at least the closest column.
        assert!(err.to_string().contains("revenue"));
    }

    #[test]
    fn numeric_columns_lists_only_numeric() {
        let df = sample();
        let numeric = df.numeric_columns();
        assert_eq!(numeric, vec!["revenue"]);
    }

    #[test]
    fn comma_list_formats_with_backticks() {
        assert_eq!(comma_list(vec!["a", "b"]), "`a`, `b`");
        assert_eq!(comma_list(vec![]), "(none)");
    }

    #[test]
    fn rejects_mismatched_column_lengths() {
        let bad = DataFrame::from_columns(vec![
            Column::new("a", Series::Numbers(vec![1.0, 2.0])),
            Column::new("b", Series::Numbers(vec![3.0])),
        ]);
        assert!(bad.is_err());
    }
}
