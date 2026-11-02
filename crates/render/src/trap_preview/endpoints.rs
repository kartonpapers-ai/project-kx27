//! Bounded intersections of original local curves, including clips and the search
//! rectangle. These cuts stop a sampling cell from straddling a visibility junction.
use super::*;

pub(super) const TOLERANCE: f64 = 0.0001; // document points (~0.000035 mm)
const MAX_VISITS: usize = 120_000;

fn spend(visits: &mut usize) -> Result<(), String> {
    *visits += 1;
    if *visits > MAX_VISITS {
        return Err("Los cruces superan el presupuesto local. Elige una zona menor.".into());
    }
    Ok(())
}
fn coincident(a: PathSeg, b: PathSeg) -> bool {
    [false, true]
        .iter()
        .any(|reverse| [0.0, 0.25, 0.5, 0.75, 1.0].iter().all(|&t| a.eval(t).distance(b.eval(if *reverse { 1.0 - t } else { t })) < TOLERANCE * 0.01))
}
fn crossings(a: PathSeg, b: PathSeg, cuts: &mut Vec<f64>, visits: &mut usize) -> Result<(), String> {
    spend(visits)?;
    if !overlaps(a.bounding_box().inflate(TOLERANCE, TOLERANCE), b.bounding_box()) || coincident(a, b) {
        return Ok(());
    }
    if let (PathSeg::Line(a), PathSeg::Line(b)) = (a, b) {
        let u = a.p1 - a.p0;
        let v = b.p1 - b.p0;
        let delta = b.p0 - a.p0;
        let det = u.cross(v);
        if det.abs() > 1e-14 * u.hypot().max(1.0) * v.hypot().max(1.0) {
            let t = delta.cross(v) / det;
            let s = delta.cross(u) / det;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&s) {
                cuts.push(t);
            }
        } else if u.hypot2() > 1e-20 && delta.cross(u).abs() <= TOLERANCE * u.hypot() {
            // Shared straight borders: retain the ends of their common interval.
            for p in [b.p0, b.p1] {
                let t = (p - a.p0).dot(u) / u.hypot2();
                if (0.0..=1.0).contains(&t) {
                    cuts.push(t);
                }
            }
        }
        return Ok(());
    }
    let mut work = vec![(a, b, 0.0, 1.0, 0u8)];
    while let Some((a, b, lo, hi, depth)) = work.pop() {
        spend(visits)?;
        let ab = a.bounding_box();
        let bb = b.bounding_box();
        if !overlaps(ab.inflate(TOLERANCE * 0.01, TOLERANCE * 0.01), bb) {
            continue;
        }
        let size_a = ab.width().hypot(ab.height());
        let size_b = bb.width().hypot(bb.height());
        if size_a.max(size_b) <= TOLERANCE {
            if a.eval(0.5).distance(b.eval(0.5)) <= TOLERANCE * 2.0 {
                cuts.push((lo + hi) * 0.5);
            }
            continue;
        }
        if depth >= 80 {
            return Err("No se pudo resolver un cruce local con la tolerancia requerida.".into());
        }
        if size_a >= size_b {
            let mid = (lo + hi) * 0.5;
            work.push((a.subsegment(0.5..1.0), b, mid, hi, depth + 1));
            work.push((a.subsegment(0.0..0.5), b, lo, mid, depth + 1));
        } else {
            work.push((a, b.subsegment(0.5..1.0), lo, hi, depth + 1));
            work.push((a, b.subsegment(0.0..0.5), lo, hi, depth + 1));
        }
        if cuts.len() > 8192 {
            return Err("Demasiados cruces en la zona. Elige una región menor.".into());
        }
    }
    Ok(())
}
pub(super) fn parameters(original: PathSeg, boundaries: &[PathSeg], region: Rect, visits: &mut usize) -> Result<Vec<f64>, String> {
    let mut cuts = vec![0.0, 1.0];
    for &other in boundaries {
        crossings(original, other, &mut cuts, visits)?;
    }
    let p = [Point::new(region.x0, region.y0), Point::new(region.x1, region.y0), Point::new(region.x1, region.y1), Point::new(region.x0, region.y1)];
    for i in 0..4 {
        crossings(original, PathSeg::Line(kurbo::Line::new(p[i], p[(i + 1) % 4])), &mut cuts, visits)?;
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
    Ok(cuts)
}
