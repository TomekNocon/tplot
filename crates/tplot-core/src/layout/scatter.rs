use crate::dataframe::{DataFrame, Series};
pub use crate::layout::bar::PlotBox;

#[derive(Debug, Clone)]
pub struct ScatterSeries {
    pub key: String,
    pub points: Vec<(usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct ScatterLayout {
    pub plot_box: PlotBox,
    pub series: Vec<ScatterSeries>,
    pub canvas_cells_w: usize,
    pub canvas_cells_h: usize,
    pub left_margin: usize,
    pub bottom_reserve: usize,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScatterLayoutError {
    #[error("x column `{0}` must be numeric")]
    NonNumericX(String),
    #[error("y column `{0}` must be numeric")]
    NonNumericY(String),
    #[error("no data rows")]
    Empty,
}

const SUB_X: usize = 2;
const SUB_Y: usize = 4;
const Y_AXIS_LABEL_WIDTH: usize = 6;

pub fn layout_scatter(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
    group_col: Option<&str>,
    canvas_cells_w: usize,
    canvas_cells_h: usize,
) -> Result<ScatterLayout, ScatterLayoutError> {
    let xs: Vec<f64> = match df
        .column(x_col)
        .map_err(|_| ScatterLayoutError::Empty)?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(ScatterLayoutError::NonNumericX(x_col.to_string())),
    };
    let ys: Vec<f64> = match df
        .column(y_col)
        .map_err(|_| ScatterLayoutError::Empty)?
        .series()
    {
        Series::Numbers(v) => v.clone(),
        Series::Strings(_) => return Err(ScatterLayoutError::NonNumericY(y_col.to_string())),
    };
    if xs.is_empty() {
        return Err(ScatterLayoutError::Empty);
    }

    let groups: Vec<String> = if let Some(g) = group_col {
        match df.column(g).map_err(|_| ScatterLayoutError::Empty)?.series() {
            Series::Strings(v) => v.clone(),
            Series::Numbers(v) => v.iter().map(|n| format!("{n}")).collect(),
        }
    } else {
        vec!["__main__".to_string(); xs.len()]
    };

    let x_min = xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_max = xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_min = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let x_span = (x_max - x_min).max(1e-9);
    let y_span = (y_max - y_min).max(1e-9);

    let left_margin = Y_AXIS_LABEL_WIDTH;
    let bottom_reserve = 3;
    let plot_cells_w = canvas_cells_w.saturating_sub(left_margin).max(8);
    let plot_cells_h = canvas_cells_h.saturating_sub(bottom_reserve).max(4);
    let plot_pixels_w = plot_cells_w * SUB_X;
    let plot_pixels_h = plot_cells_h * SUB_Y;

    let plot_box = PlotBox {
        pixel_width: plot_pixels_w,
        pixel_height: plot_pixels_h,
    };

    let mut series: Vec<ScatterSeries> = Vec::new();
    for ((x, y), g) in xs.iter().zip(ys.iter()).zip(groups.iter()) {
        let px = (((x - x_min) / x_span) * (plot_pixels_w - 1) as f64).round() as usize;
        let py = ((y_max - y) / y_span * (plot_pixels_h - 1) as f64).round() as usize;
        let py = py.min(plot_pixels_h - 1);
        if let Some(s) = series.iter_mut().find(|s| s.key == *g) {
            s.points.push((px, py));
        } else {
            series.push(ScatterSeries {
                key: g.clone(),
                points: vec![(px, py)],
            });
        }
    }

    Ok(ScatterLayout {
        plot_box,
        series,
        canvas_cells_w,
        canvas_cells_h,
        left_margin,
        bottom_reserve,
        x_min,
        x_max,
        y_min,
        y_max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{Column, DataFrame, Series};

    fn pts_df() -> DataFrame {
        DataFrame::from_columns(vec![
            Column::new("x", Series::Numbers(vec![1.0, 2.0, 3.0, 4.0, 5.0])),
            Column::new("y", Series::Numbers(vec![10.0, 25.0, 18.0, 42.0, 30.0])),
            Column::new(
                "g",
                Series::Strings(
                    vec!["A", "B", "A", "B", "A"]
                        .into_iter()
                        .map(String::from)
                        .collect(),
                ),
            ),
        ])
        .unwrap()
    }

    #[test]
    fn produces_one_point_per_row_when_ungrouped() {
        let layout = layout_scatter(&pts_df(), "x", "y", None, 80, 16).unwrap();
        assert_eq!(layout.series.len(), 1);
        assert_eq!(layout.series[0].points.len(), 5);
    }

    #[test]
    fn groups_split_into_separate_series() {
        let layout = layout_scatter(&pts_df(), "x", "y", Some("g"), 80, 16).unwrap();
        assert_eq!(layout.series.len(), 2);
    }
}
