/// Produce a one-line takeaway for a horizontal bar chart.
///
/// `focal_value` is the focal series' value; `median` is the median across
/// all series; `n_series` is the count for the "out of N" phrasing.
pub fn bar_takeaway(focal: Option<&str>, focal_value: f64, median: f64, n_series: usize) -> String {
    let Some(name) = focal else {
        return "No series stands out clearly — values are within ±50% of the median.".into();
    };
    if median.abs() < 1e-9 {
        return format!(
            "{name} led with {focal_value:.0} — the standout in an otherwise flat field."
        );
    }
    let multiple = focal_value / median;
    if multiple >= 2.5 {
        format!(
            "{name} was the standout at {focal_value:.0} — over {multiple:.1}× the median across {n_series} series."
        )
    } else {
        format!("{name} led with {focal_value:.0}, the highest of {n_series} series.")
    }
}

/// Produce a one-line takeaway for a line chart. `focal` is the focal
/// series' key (or `None` for a roughly flat field). `first` and `last`
/// are the focal series' starting and ending values.
pub fn line_takeaway(focal: Option<&str>, first: f64, last: f64) -> String {
    match focal {
        None => "No series stands out — trends are flat.".to_string(),
        Some(name) => {
            let delta = last - first;
            let direction = if delta >= 0.0 { "grew" } else { "fell" };
            if first.abs() < 1e-9 {
                format!("{name} {direction} from {first:.0} to {last:.0}.")
            } else {
                let pct = (delta.abs() / first.abs() * 100.0).round() as i64;
                let multiple = (last / first).abs();
                if multiple >= 2.0 {
                    format!("{name} {direction} {multiple:.1}× — from {first:.0} to {last:.0}.")
                } else {
                    format!("{name} {direction} {pct}% — from {first:.0} to {last:.0}.")
                }
            }
        }
    }
}

/// Produce a one-line takeaway for a histogram. `modal_bin` is the label of
/// the bin that dominates (or `None` for a roughly uniform distribution).
/// `modal_count` is the count of observations in that bin and `total` is the
/// total observation count.
pub fn histogram_takeaway(modal_bin: Option<&str>, modal_count: u64, total: u64) -> String {
    match modal_bin {
        None => "Distribution is roughly even — no clear cluster.".to_string(),
        Some(label) if total == 0 => format!("Most observations fell in {label}."),
        Some(label) => {
            let pct = (modal_count as f64 / total as f64 * 100.0).round() as i64;
            format!("Most observations clustered in {label} — {pct}% of the total.")
        }
    }
}

/// `cell` is (x_label, y_label); `value` is the hot cell's value;
/// `total` is the sum across all cells (used for percentage phrasing).
pub fn heatmap_takeaway(cell: Option<(&str, &str)>, value: f64, total: f64) -> String {
    match cell {
        None => "No data — heatmap is empty.".to_string(),
        Some((y_label, x_label)) if total <= 0.0 => {
            format!("Hottest cell: {y_label} × {x_label} ({value:.0}).")
        }
        Some((y_label, x_label)) => {
            let pct = (value / total * 100.0).round() as i64;
            format!("Hottest cell: {y_label} × {x_label} ({value:.0}) — {pct}% of total.")
        }
    }
}

/// Produce a one-line takeaway for a box plot. `focal` is the focal
/// series' key (the widest-IQR group), or `None` for a roughly uniform
/// spread. The other arguments are the focal group's quartiles and bounds
/// — the binary supplies them from the layout's 5-number summary.
pub fn boxplot_takeaway(focal: Option<&str>, q1: f64, q3: f64, min_v: f64, max_v: f64) -> String {
    match focal {
        None => "Series have similar spread — no obvious outlier distribution.".to_string(),
        Some(name) => {
            let iqr = q3 - q1;
            let total_range = max_v - min_v;
            format!(
                "{name} has the widest spread — IQR {iqr:.0} (range {min_v:.0}–{max_v:.0}, total {total_range:.0})."
            )
        }
    }
}

/// Produce a one-line takeaway for a stacked area chart. `focal` is the
/// largest-total series' key (or `None` when totals are uniform).
/// `focal_total` is that series's cumulative total, and `grand_total` is the
/// sum across all series (used for percentage phrasing).
pub fn stacked_area_takeaway(focal: Option<&str>, focal_total: f64, grand_total: f64) -> String {
    match focal {
        None => "Series contributed similar amounts — no clear leader.".to_string(),
        Some(name) if grand_total <= 0.0 => format!("{name} carried the bulk of the total."),
        Some(name) => {
            let pct = (focal_total / grand_total * 100.0).round() as i64;
            format!(
                "{name} contributed the most — {pct}% of the cumulative total ({focal_total:.0} of {grand_total:.0})."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_takeaway_names_focal_and_quantifies() {
        let t = bar_takeaway(Some("EMEA"), 72.0, 25.5, 5);
        assert!(t.contains("EMEA"));
        assert!(t.contains("standout") || t.contains("led") || t.contains("highest"));
    }

    #[test]
    fn no_focal_returns_neutral_message() {
        let t = bar_takeaway(None, 0.0, 0.0, 0);
        assert!(t.to_lowercase().contains("no series"));
    }

    #[test]
    fn histogram_takeaway_names_modal_bin_and_pct() {
        let t = histogram_takeaway(Some("40–60"), 8, 19);
        assert!(t.contains("40–60"));
        assert!(t.contains("42%") || t.contains("43%") || t.contains("4"));
    }

    #[test]
    fn histogram_takeaway_neutral_for_uniform() {
        let t = histogram_takeaway(None, 0, 0);
        assert!(t.to_lowercase().contains("evenly") || t.to_lowercase().contains("no clear"));
    }

    #[test]
    fn line_takeaway_describes_trend_direction_and_magnitude() {
        let t = line_takeaway(Some("EMEA"), 50.0, 200.0);
        assert!(t.contains("EMEA"));
        assert!(t.contains("rose") || t.contains("grew") || t.contains("4×") || t.contains("300%"));
    }

    #[test]
    fn line_takeaway_neutral_for_no_focal() {
        let t = line_takeaway(None, 0.0, 0.0);
        assert!(t.to_lowercase().contains("no series") || t.to_lowercase().contains("flat"));
    }

    #[test]
    fn heatmap_takeaway_names_hot_cell_with_value() {
        let t = heatmap_takeaway(Some(("Tue", "10")), 25.0, 60.0);
        assert!(t.contains("Tue"));
        assert!(t.contains("10"));
        assert!(t.contains("25") || t.contains("42%") || t.contains("41%"));
    }

    #[test]
    fn heatmap_takeaway_neutral_for_empty_grid() {
        let t = heatmap_takeaway(None, 0.0, 0.0);
        assert!(t.to_lowercase().contains("no data") || t.to_lowercase().contains("empty"));
    }

    #[test]
    fn boxplot_takeaway_names_focal_with_iqr() {
        let t = boxplot_takeaway(Some("/orders"), 50.0, 200.0, 30.0, 400.0);
        assert!(t.contains("/orders"));
        assert!(t.contains("150") || t.contains("IQR") || t.contains("spread"));
    }

    #[test]
    fn boxplot_takeaway_neutral_for_no_focal() {
        let t = boxplot_takeaway(None, 0.0, 0.0, 0.0, 0.0);
        assert!(t.to_lowercase().contains("similar") || t.to_lowercase().contains("no series"));
    }

    #[test]
    fn stacked_area_takeaway_quantifies_share() {
        let t = stacked_area_takeaway(Some("NA"), 70.0, 94.0);
        assert!(t.contains("NA"));
        assert!(t.contains("74%") || t.contains("75%") || t.contains("70"));
    }

    #[test]
    fn stacked_area_takeaway_neutral_for_no_focal() {
        let t = stacked_area_takeaway(None, 0.0, 0.0);
        assert!(t.to_lowercase().contains("similar") || t.to_lowercase().contains("no series"));
    }
}
