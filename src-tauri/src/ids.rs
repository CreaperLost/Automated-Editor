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

/// A safe id for a file name or a reference: 1 to 128 ASCII letters, digits, `-` or `_`.
pub fn is_plain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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

    #[test]
    fn a_plain_id_has_no_path_in_it() {
        assert!(is_plain_id("clip-5_a"));
        assert!(!is_plain_id(""));
        assert!(!is_plain_id("../clip"));
        assert!(!is_plain_id(&"a".repeat(129)));
    }
}
