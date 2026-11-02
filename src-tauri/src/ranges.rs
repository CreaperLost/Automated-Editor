//! Stretches of time as `(start, end)` pairs, end exclusive.

/// `ranges` sorted, the empty ones dropped, those overlapping or touching joined.
pub fn merge(ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    merge_near(ranges, 0)
}

/// [`merge`] that also joins ranges closer than `gap`.
pub fn merge_near(mut ranges: Vec<(u64, u64)>, gap: u64) -> Vec<(u64, u64)> {
    ranges.retain(|(a, b)| b > a);
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges {
        match out.last_mut() {
            Some(last) if a <= last.1.saturating_add(gap) => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// What is left of `ranges`, in their order, outside `remove` (in any order).
pub fn subtract(ranges: &[(u64, u64)], remove: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let remove = merge(remove.to_vec());
    let mut out = Vec::new();
    for &(a, b) in ranges {
        let mut at = a;
        for &(x, y) in remove.iter().filter(|(x, y)| *x < b && *y > a) {
            if x > at {
                out.push((at, x));
            }
            at = at.max(y);
        }
        if at < b {
            out.push((at, b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_sorts_joins_and_drops_the_empty() {
        assert_eq!(
            merge(vec![(50, 60), (0, 10), (5, 20), (20, 30), (40, 40)]),
            vec![(0, 30), (50, 60)]
        );
        assert_eq!(merge_near(vec![(0, 10), (15, 20)], 5), vec![(0, 20)]);
        assert_eq!(
            merge_near(vec![(0, 10), (16, 20)], 5),
            vec![(0, 10), (16, 20)]
        );
        assert_eq!(
            merge_near(vec![(0, u64::MAX), (5, 6)], 5),
            vec![(0, u64::MAX)]
        );
    }

    #[test]
    fn subtracting_keeps_what_is_outside_in_order() {
        assert_eq!(
            subtract(&[(0, 10), (20, 30)], &[(25, 40), (2, 3), (3, 4)]),
            vec![(0, 2), (4, 10), (20, 25)]
        );
        assert_eq!(subtract(&[(5, 6)], &[(0, 10)]), vec![]);
        assert_eq!(subtract(&[(5, 6)], &[]), vec![(5, 6)]);
    }
}
