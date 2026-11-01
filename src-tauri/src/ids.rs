//! Ids and names made from a name, kept apart from the ones already in use.

/// The first of `base`, `base-2`, `base-3`... that `taken` doesn't claim.
pub fn free_id(base: &str, taken: impl Fn(&str) -> bool) -> String {
    first_free(base, "-", taken)
}

/// The first of `base`, `base 2`, `base 3`... that `taken` doesn't claim: for names people read.
pub fn free_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    first_free(base, " ", taken)
}

fn first_free(base: &str, separator: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..)
        .map(|n| match n {
            1 => base.to_string(),
            n => format!("{base}{separator}{n}"),
        })
        .find(|id| !taken(id))
        .expect("an id is free")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_free_id_is_the_base_then_numbered_from_two() {
        assert_eq!(free_id("clip-5", |_| false), "clip-5");
        let used = ["clip-5", "clip-5-2"];
        assert_eq!(free_id("clip-5", |id| used.contains(&id)), "clip-5-3");
        assert_eq!(
            free_name("My tutorial", |n| n == "My tutorial"),
            "My tutorial 2"
        );
    }
}
