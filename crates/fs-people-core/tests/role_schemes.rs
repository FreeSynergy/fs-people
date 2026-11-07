//! Role files as SKOS concept schemes: full ids, files side by side, atomic swap.
//! These tests talk to the `RoleDirectory` trait only.

use fs_people_core::{
    AI_ROLE_IRI, Error, RoleDirectory, RoleKind, RoleRegistry, RoleScope, full_id,
};
use serde_json::json;

const MINE: &str = "https://example.org/roles/kal/";
const OURS: &str = "https://freesynergy.net/ns/roles/";

fn scheme(base: &str, roles: serde_json::Value) -> String {
    json!({
        "@context": ["https://freesynergy.net/ns/roles.jsonld", {"@base": base}],
        "id": "", "type": "skos:ConceptScheme", "names": {"en": "Test"}, "roles": roles
    })
    .to_string()
}

fn role(id: &str) -> serde_json::Value {
    json!({"id": id, "type": "skos:Concept", "names": {"en": id}, "owner": "kal", "scope": "personal"})
}

#[test]
fn shipped_roles_are_found_by_full_id_and_by_the_short_one() {
    let dir: Box<dyn RoleDirectory> = Box::new(RoleRegistry::known());
    let ai = dir.find(AI_ROLE_IRI).unwrap();
    assert_eq!(ai.id, AI_ROLE_IRI);
    assert_eq!(dir.find("ai").unwrap(), ai);
    assert_eq!(full_id("ai"), AI_ROLE_IRI);
    assert!(dir.find(&format!("{OURS}nobody")).is_none());
}

#[test]
fn shipped_roles_have_the_decided_types_and_names() {
    let dir = RoleRegistry::known();
    for id in ["customer", "supplier", "lead", "prospect"] {
        let def = dir.find(id).unwrap();
        assert_eq!(def.kind, RoleKind::Concept, "{id}");
        assert!(
            def.names.contains_key("en") && def.names.contains_key("de"),
            "{id}"
        );
        assert!(
            def.card_kind.is_none(),
            "{id} may be held by an org card too"
        );
    }
    assert_eq!(dir.find("member").unwrap().kind, RoleKind::OrgRole);
    assert_eq!(dir.find("lead").unwrap().names["de"], "Lead");
    assert_eq!(dir.find("prospect").unwrap().names["de"], "Interessent");
    assert_eq!(dir.find("customer").unwrap().names["de"], "Kunde");
}

#[test]
fn two_files_sit_side_by_side_and_do_not_clash() {
    let dir = RoleRegistry::known();
    dir.load(&scheme(MINE, json!([role("customer"), role("treasurer")])))
        .unwrap();
    let ours = dir.find(&format!("{OURS}customer")).unwrap();
    let mine = dir.find(&format!("{MINE}customer")).unwrap();
    assert_ne!(ours, mine);
    assert_eq!(ours.kind, RoleKind::Concept);
    assert_eq!(mine.id, format!("{MINE}customer"));
    assert!(dir.find("ai").is_some(), "ours stays");
    assert_eq!(dir.bases(), [MINE, OURS]);
}

#[test]
fn loading_replaces_only_the_scheme_with_the_same_base() {
    let dir = RoleRegistry::known();
    dir.load(&scheme(MINE, json!([role("a")]))).unwrap();
    dir.load(&scheme(MINE, json!([role("b")]))).unwrap();
    assert!(dir.find(&format!("{MINE}a")).is_none());
    assert!(dir.find(&format!("{MINE}b")).is_some());
    assert!(dir.find("member").is_some());
}

#[test]
fn a_bad_file_changes_nothing_and_a_preview_changes_nothing() {
    let dir = RoleRegistry::known();
    let before = dir.all();
    let twice = scheme(MINE, json!([role("a"), role("a")]));
    assert!(matches!(dir.load(&twice), Err(Error::Conflict(_))));
    let no_base = json!({"@context": "x", "id": "", "type": "skos:ConceptScheme", "roles": []});
    assert!(matches!(
        dir.load(&no_base.to_string()),
        Err(Error::Invalid(_))
    ));
    let seen = dir.preview(&scheme(MINE, json!([role("a")]))).unwrap();
    assert!(seen.contains_key(&format!("{MINE}a")) && seen.contains_key(AI_ROLE_IRI));
    assert_eq!(dir.all(), before);
}

#[test]
fn owner_is_an_id_not_a_sentence() {
    let dir = RoleRegistry::empty();
    let mut bad = role("a");
    bad["owner"] = json!("CRM (later)");
    assert!(matches!(
        dir.load(&scheme(MINE, json!([bad]))),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn the_fields_of_the_format_are_kept() {
    let dir = RoleRegistry::empty();
    let mut r = role("treasurer");
    r["type"] = json!("org:Role");
    r["broader"] = json!(format!("{OURS}member"));
    r["closeMatch"] = json!("https://example.org/other/schatzmeister");
    r["standardRefs"] = json!(["https://schema.org/EmployeeRole"]);
    r["description"] = json!({"de": "Verwaltet die Kasse."});
    r["scope"] = json!("group");
    dir.load(&scheme(MINE, json!([r]))).unwrap();
    let def = dir.find(&format!("{MINE}treasurer")).unwrap();
    assert_eq!(def.kind, RoleKind::OrgRole);
    assert_eq!(def.scope, RoleScope::Group);
    assert_eq!(
        def.broader.as_deref(),
        Some(format!("{OURS}member").as_str())
    );
    assert_eq!(
        def.close_match.as_deref(),
        Some("https://example.org/other/schatzmeister")
    );
    assert_eq!(def.standard_refs, ["https://schema.org/EmployeeRole"]);
}

#[test]
fn the_old_array_is_still_read_as_a_file_of_our_base() {
    let dir = RoleRegistry::empty();
    dir.load(&json!([{"id": "x", "names": {"en": "X"}, "owner": "o"}]).to_string())
        .unwrap();
    assert_eq!(dir.find("x").unwrap().id, full_id("x"));
}
