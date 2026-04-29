use crate::dataframe::{Column, DataFrame, DataFrameError, Series};
use std::io::Read;

#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("csv parse error: {0}")]
    Parse(#[from] csv::Error),
    #[error(transparent)]
    DataFrame(#[from] DataFrameError),
    #[error("empty input — no header row")]
    Empty,
}

pub fn parse_csv_str(s: &str) -> Result<DataFrame, CsvError> {
    parse_csv_reader(s.as_bytes())
}

pub fn parse_csv_reader<R: Read>(r: R) -> Result<DataFrame, CsvError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(r);

    let headers: Vec<String> = rdr.headers()?.iter().map(String::from).collect();
    if headers.is_empty() {
        return Err(CsvError::Empty);
    }

    let mut raw: Vec<Vec<String>> = vec![Vec::new(); headers.len()];
    for record in rdr.records() {
        let record = record?;
        for (i, field) in record.iter().enumerate() {
            if i < raw.len() {
                raw[i].push(field.to_string());
            }
        }
    }

    // Type inference: if every row in a column parses as f64, it's numeric.
    let columns: Vec<Column> = headers
        .into_iter()
        .zip(raw)
        .map(|(name, vals)| {
            let parsed: Option<Vec<f64>> = vals
                .iter()
                .map(|v| v.trim().parse::<f64>().ok())
                .collect();
            let series = match parsed {
                Some(numbers) if !numbers.is_empty() => Series::Numbers(numbers),
                _ => Series::Strings(vals),
            };
            Column::new(name, series)
        })
        .collect();

    Ok(DataFrame::from_columns(columns)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::Series;

    #[test]
    fn parses_simple_csv() {
        let csv = "a,b\n1,foo\n2,bar\n3,baz\n";
        let df = parse_csv_str(csv).unwrap();
        assert_eq!(df.nrows(), 3);
        assert!(matches!(df.column("a").unwrap().series(), Series::Numbers(_)));
        assert!(matches!(df.column("b").unwrap().series(), Series::Strings(_)));
    }

    #[test]
    fn promotes_numeric_columns_when_all_rows_parse() {
        let csv = "x,y\nQ1,1\nQ2,2\nQ3,3\n";
        let df = parse_csv_str(csv).unwrap();
        assert!(matches!(df.column("x").unwrap().series(), Series::Strings(_)));
        assert!(matches!(df.column("y").unwrap().series(), Series::Numbers(_)));
    }

    #[test]
    fn parses_sales_fixture() {
        let csv = include_str!("../../../../tests/fixtures/sales.csv");
        let df = parse_csv_str(csv).unwrap();
        assert_eq!(df.nrows(), 20);
        let revenue = df.column("revenue").unwrap();
        assert!(matches!(revenue.series(), Series::Numbers(_)));
    }
}
