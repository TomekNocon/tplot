//! Squarify treemap rectangle packing (Bruls et al. 2000).
//!
//! Greedy algorithm that produces near-square rectangles whose areas are
//! proportional to the input values. Output rectangles are aligned with
//! input order — caller can pre-sort (typical: descending) to bias the
//! largest rectangles toward squareness.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Squarify a list of values into a target rectangle (Bruls 2000).
/// Values are NOT sorted by the function — caller can sort if desired
/// (typical: descending). Output rectangles are aligned with input order.
pub fn squarify(values: &[f64], target: Rect) -> Vec<Rect> {
    if values.is_empty() {
        return Vec::new();
    }
    let total_value: f64 = values.iter().sum::<f64>().max(1e-9);
    let total_area = target.w * target.h;

    // Scale values so their sum equals the target's area (so worst() math
    // works in area units).
    let scaled: Vec<f64> = values
        .iter()
        .map(|v| v / total_value * total_area)
        .collect();

    let mut out = Vec::with_capacity(values.len());
    squarify_inner(&scaled, &mut Vec::new(), target, &mut out);
    out
}

fn squarify_inner(remaining: &[f64], row: &mut Vec<f64>, rect: Rect, out: &mut Vec<Rect>) {
    if remaining.is_empty() {
        if !row.is_empty() {
            layout_row(row, rect, out);
        }
        return;
    }
    let next = remaining[0];
    let w = rect.w.min(rect.h);

    if row.is_empty() || worst_ratio(row, next, w) <= worst_ratio(row, 0.0, w) {
        row.push(next);
        squarify_inner(&remaining[1..], row, rect, out);
    } else {
        let new_rect = layout_row(row, rect, out);
        row.clear();
        squarify_inner(remaining, row, new_rect, out);
    }
}

/// Compute the worst aspect ratio (max of width/height and height/width across
/// all rectangles) IF we add `extra` to the row. Pass `extra = 0.0` to compute
/// the worst ratio of the row as-is.
fn worst_ratio(row: &[f64], extra: f64, w: f64) -> f64 {
    let mut sum = row.iter().sum::<f64>();
    sum += extra;
    if sum <= 0.0 || w <= 0.0 {
        return f64::INFINITY;
    }

    let mut max_v = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    for &v in row {
        if v > max_v {
            max_v = v;
        }
        if v < min_v {
            min_v = v;
        }
    }
    if extra > 0.0 {
        if extra > max_v {
            max_v = extra;
        }
        if extra < min_v {
            min_v = extra;
        }
    }
    if min_v <= 0.0 {
        return f64::INFINITY;
    }

    let s2 = sum * sum;
    let w2 = w * w;
    let r1 = (max_v * w2) / s2;
    let r2 = s2 / (min_v * w2);
    r1.max(r2)
}

/// Lay out a row of values along the SHORTER side of `rect`. Returns the
/// remaining rectangle (the part not consumed by this row).
fn layout_row(row: &[f64], rect: Rect, out: &mut Vec<Rect>) -> Rect {
    let sum: f64 = row.iter().sum();
    if rect.w <= rect.h {
        // Row goes along the top (along width); height of row = sum / width.
        let row_h = sum / rect.w.max(1e-9);
        let mut x = rect.x;
        for &v in row {
            let cell_w = v / row_h.max(1e-9);
            out.push(Rect {
                x,
                y: rect.y,
                w: cell_w,
                h: row_h,
            });
            x += cell_w;
        }
        Rect {
            x: rect.x,
            y: rect.y + row_h,
            w: rect.w,
            h: (rect.h - row_h).max(0.0),
        }
    } else {
        // Row goes down the left (along height); width of row = sum / height.
        let row_w = sum / rect.h.max(1e-9);
        let mut y = rect.y;
        for &v in row {
            let cell_h = v / row_w.max(1e-9);
            out.push(Rect {
                x: rect.x,
                y,
                w: row_w,
                h: cell_h,
            });
            y += cell_h;
        }
        Rect {
            x: rect.x + row_w,
            y: rect.y,
            w: (rect.w - row_w).max(0.0),
            h: rect.h,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "expected {b}, got {a}");
    }

    #[test]
    fn single_item_fills_target_rect() {
        let rects = squarify(
            &[100.0],
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
        );
        assert_eq!(rects.len(), 1);
        approx(rects[0].w, 10.0);
        approx(rects[0].h, 10.0);
    }

    #[test]
    fn two_equal_items_split_target_evenly() {
        // Target is 10x10 (square). Two equal-value items should get equal area
        // and produce a 1:2 stack.
        let rects = squarify(
            &[50.0, 50.0],
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
        );
        assert_eq!(rects.len(), 2);
        // Each rect has area 50 ([10x10 / 2]).
        let area0 = rects[0].w * rects[0].h;
        let area1 = rects[1].w * rects[1].h;
        approx(area0, 50.0);
        approx(area1, 50.0);
    }

    #[test]
    fn rectangles_cover_target_area() {
        let target = Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 400.0,
        };
        let values = vec![6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let total: f64 = values.iter().sum();
        let rects = squarify(&values, target);
        let computed_area: f64 = rects.iter().map(|r| r.w * r.h).sum();
        approx(computed_area, target.w * target.h);
        // And areas should be proportional to values.
        for (v, r) in values.iter().zip(rects.iter()) {
            let expected_area = v / total * target.w * target.h;
            approx(r.w * r.h, expected_area);
        }
    }

    #[test]
    fn rectangles_dont_overlap_or_leak() {
        let target = Rect {
            x: 5.0,
            y: 10.0,
            w: 100.0,
            h: 80.0,
        };
        let rects = squarify(&[10.0, 8.0, 7.0, 6.0, 5.0], target);
        // Every rect must lie within the target.
        for r in &rects {
            assert!(r.x >= target.x - 1e-6);
            assert!(r.y >= target.y - 1e-6);
            assert!(r.x + r.w <= target.x + target.w + 1e-6);
            assert!(r.y + r.h <= target.y + target.h + 1e-6);
        }
        // Pairwise non-overlap (open-interior check).
        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let a = &rects[i];
                let b = &rects[j];
                let overlap_x = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                let overlap_y = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
                let overlap = overlap_x.max(0.0) * overlap_y.max(0.0);
                assert!(overlap < 1e-3, "rects {i}/{j} overlap by {overlap}");
            }
        }
    }
}
