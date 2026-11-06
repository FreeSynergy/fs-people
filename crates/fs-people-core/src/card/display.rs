//! Display name of a card (ADR 0032).
//!
//! Pattern: Chain of responsibility (fallback chain) — `name.full`, then name components, then
//! nickname, organisation, e-mail, `uid`; the first non-empty answer wins, so the result is never empty.

use super::model::Card;

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

impl Card {
    /// Name for display and for the envelope `name` (ADR 0032): `name.full`, else the joined
    /// components, else nickname, organisation, e-mail, finally the `uid`. Never empty.
    pub fn display_name(&self) -> String {
        if let Some(n) = &self.name {
            if let Some(f) = n.full.as_deref().and_then(non_empty) {
                return f;
            }
            let parts: Vec<&str> = n
                .components
                .iter()
                .filter(|c| !c.value.trim().is_empty() && c.kind != "separator")
                .map(|c| c.value.trim())
                .collect();
            if !parts.is_empty() {
                return parts.join(" ");
            }
        }
        self.nicknames
            .values()
            .find_map(|n| non_empty(&n.name))
            .or_else(|| {
                self.organizations
                    .values()
                    .find_map(|o| o.name.as_deref().and_then(non_empty))
            })
            .or_else(|| self.emails.values().find_map(|e| non_empty(&e.address)))
            .or_else(|| non_empty(&self.uid))
            .unwrap_or_else(|| "Unnamed contact".into())
    }
}
