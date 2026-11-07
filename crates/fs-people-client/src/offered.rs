//! What this client offers of what the server reports.
//!
//! Pattern: Adapter (a filter on the report) — provider rule, ADR 0045: `capabilities()` never
//! promises more than the methods of `RpcPeople` keep. The server's report is cut to what the client
//! implements; the rest is reported as not possible, with the reason.

use fs_interfaces::Capabilities;

/// What the server can do and this client does not call: reported as not possible. A capability the
/// client keeps in part (`person.read` without `get_by_uid`, `person.find` as `search` only,
/// `person.import` as vCard, `role.read` without `roles`) stays reported as possible; `DESIGN.md`
/// lists the methods that answer `Unsupported` all the same.
pub(crate) const NOT_OFFERED: [&str; 12] = [
    "person.subscribe",
    "relation.read",
    "relation.write",
    "cardgroup.read",
    "cardgroup.write",
    "same.propose",
    "same.link",
    "same.read",
    "role.define",
    "role.configure",
    "role.assign",
    "ai.mark",
];

/// The report of the server, cut to what this client implements.
pub(crate) fn offered(reported: &Capabilities) -> Capabilities {
    let mut caps = Capabilities::new(reported.provider());
    for entry in reported.entries() {
        if entry.supported && NOT_OFFERED.contains(&entry.capability.as_str()) {
            caps.unsupported(
                entry.capability.as_str(),
                "the server offers it, the client of the people program does not yet",
            );
        } else if entry.supported {
            caps.supported(entry.capability.as_str());
        } else {
            caps.unsupported(entry.capability.as_str(), entry.reason.as_str());
        }
    }
    caps
}
