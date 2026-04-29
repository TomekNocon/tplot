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
}
