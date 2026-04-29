use crate::dataframe::{Column, DataFrame, DataFrameError, Series};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use tplot_protocol::ChartSpec;

#[derive(Debug, thiserror::Error)]
pub enum JsonError {
    #[error("json parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("missing `data` object — expected per-column arrays")]
    MissingData,
    #[error(transparent)]
    DataFrame(#[from] DataFrameError),
}

#[derive(Debug)]
pub struct ParsedJson {
    pub dataframe: DataFrame,
    pub spec: Option<ChartSpec>,
}

#[derive(Debug, Deserialize)]
struct RawJson {
    #[serde(default)]
    data: Option<BTreeMap<String, Vec<Value>>>,
    #[serde(flatten)]
    rest: serde_json::Map<String, Value>,
}

/// Normalize an Axis value written in the externally-tagged form
/// `{"Column": "name"}` or `{"Inline": [...]}` into the untagged form expected
/// by `tplot_protocol::Axis`, which is just `"name"` or `[...]`.
fn normalize_axis(v: &mut Value) {
    if let Value::Object(map) = v
        && map.len() == 1
    {
        if let Some(inner) = map.remove("Column") {
            *v = inner;
            return;
        }
        if let Some(inner) = map.remove("Inline") {
            *v = inner;
        }
    }
}

pub fn parse_json_str(s: &str) -> Result<ParsedJson, JsonError> {
    let raw: RawJson = serde_json::from_str(s)?;
    let data = raw.data.ok_or(JsonError::MissingData)?;

    let columns: Vec<Column> = data
        .into_iter()
        .map(|(name, values)| {
            // If every value is a JSON number, store as Numbers.
            let all_numeric = values.iter().all(|v| v.is_number());
            let series = if all_numeric {
                Series::Numbers(values.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
            } else {
                Series::Strings(
                    values
                        .into_iter()
                        .map(|v| match v {
                            Value::String(s) => s,
                            other => other.to_string(),
                        })
                        .collect(),
                )
            };
            Column::new(name, series)
        })
        .collect();

    let dataframe = DataFrame::from_columns(columns)?;

    // Best-effort spec extraction: if the JSON has a `kind` field, try to
    // interpret it as a ChartSpec (the binary will use this for `--json` mode).
    let spec = if raw.rest.contains_key("kind") {
        let mut rest = raw.rest;
        // Normalize Axis fields written as `{"Column": "name"}` (externally
        // tagged) to the untagged form `"name"` that `Axis` expects.
        if let Some(x) = rest.get_mut("x") {
            normalize_axis(x);
        }
        if let Some(y) = rest.get_mut("y") {
            normalize_axis(y);
        }
        serde_json::from_value::<ChartSpec>(Value::Object(rest)).ok()
    } else {
        None
    };

    Ok(ParsedJson { dataframe, spec })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::Series;

    #[test]
    fn parses_inline_columns() {
        let j = r#"{
            "data": {
                "x": ["a","b","c"],
                "y": [1.0, 2.0, 3.0]
            }
        }"#;
        let parsed = parse_json_str(j).unwrap();
        let df = parsed.dataframe;
        assert_eq!(df.nrows(), 3);
        assert!(matches!(df.column("y").unwrap().series(), Series::Numbers(_)));
    }

    #[test]
    fn extracts_chart_kind() {
        let j = r#"{
            "kind": "bar",
            "orientation": "horizontal",
            "x": {"Column": "x"},
            "y": {"Column": "y"},
            "data": {"x": ["a"], "y": [1]}
        }"#;
        let parsed = parse_json_str(j).unwrap();
        assert!(parsed.spec.is_some());
    }
}
