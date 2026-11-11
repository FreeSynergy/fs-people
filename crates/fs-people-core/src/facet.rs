//! The facets of the people program: which view a window of `fs-people --facet NAME` shows.
//!
//! Pattern: Registry (a fixed table) — the program has one facet today, `people`; the server serves
//! everything whatever the facet says (ADR 0063, 0066).

/// The facets of this program.
pub const FACETS: &[&str] = &["people"];

/// Check a facet name (already shaped by `fs_interfaces::rpc::parse_facet`) against the facets of this program.
///
/// # Errors
/// The text for the person: the name is unknown, the known ones are listed.
pub fn check_facet(name: &str) -> Result<(), String> {
    if FACETS.contains(&name) {
        return Ok(());
    }
    Err(format!(
        "unknown facet {name}: this program has {}",
        FACETS.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_names_pass() {
        assert!(check_facet("people").is_ok());
        assert!(check_facet("gallery").unwrap_err().contains("people"));
    }
}
