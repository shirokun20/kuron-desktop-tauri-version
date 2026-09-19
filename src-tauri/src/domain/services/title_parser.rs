//! TitleParser — port `services/title_parser.dart` (regex -> `regex` crate, Fase 2).

/// Fase 0: trim + collapse whitespace saja.
pub fn clean_title(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace() {
        assert_eq!(clean_title("  Mock   Gallery  1 "), "Mock Gallery 1");
    }
}
