//! The canonical title-to-slug rule.
//!
//! Single source of truth for every filename slug duckspec derives from a
//! human title. Callers decide how to treat an empty result (see [`slugify`]).

/// Convert a human title into a kebab-case slug.
///
/// Lowercases, keeps Unicode alphanumerics, maps every run of other characters
/// to a single `-`, and trims leading/trailing `-`. Returns an empty string
/// when the input has no alphanumeric characters; callers decide how to treat
/// that.
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec slug Slug transformation: Words become lowercase, dash-joined tokens
    #[test]
    fn words_become_lowercase_dash_joined_tokens() {
        assert_eq!(slugify("Implement Auth"), "implement-auth");
    }

    // @spec slug Slug transformation: A run of non-alphanumeric characters collapses to one dash
    #[test]
    fn non_alphanumeric_run_collapses_to_one_dash() {
        assert_eq!(slugify("Soundness & fidelity"), "soundness-fidelity");
    }

    // @spec slug Slug transformation: Leading and trailing non-alphanumeric characters are dropped
    #[test]
    fn leading_and_trailing_non_alphanumeric_are_dropped() {
        assert_eq!(slugify("-- Draft! --"), "draft");
    }

    // @spec slug Slug transformation: Unicode alphanumerics are preserved
    #[test]
    fn unicode_alphanumerics_are_preserved() {
        assert_eq!(slugify("Café Résumé"), "café-résumé");
    }

    // @spec slug Slug transformation: A title with no alphanumeric characters yields an empty string
    #[test]
    fn no_alphanumeric_characters_yields_empty_string() {
        assert_eq!(slugify("!!! ---"), "");
    }
}
