//! The rules for the sub-objects added with the rest of RFC 9553 (required members, `@type`, `pref`),
//! on the card of the fixture with one member taken away or changed at a time.

use fs_people_core::Card;
use serde_json::{Value, json};

const FULL: &str = include_str!("fixtures/full_card.json");

fn problems_of(mut edit: impl FnMut(&mut Value)) -> Vec<String> {
    let mut v: Value = serde_json::from_str(FULL).unwrap();
    edit(&mut v);
    let card: Card = serde_json::from_value(v).unwrap();
    fs_people_core::validate::issues(&card)
        .into_iter()
        .map(|i| i.path)
        .collect()
}

#[test]
fn the_required_members_of_the_added_sub_objects_are_checked() {
    for (path, member) in [
        ("calendars/c1", "uri"),
        ("schedulingAddresses/a1", "uri"),
        ("cryptoKeys/k1", "uri"),
        ("directories/d1", "uri"),
        ("media/m1", "uri"),
        ("preferredLanguages/l1", "language"),
        ("anniversaries/b", "date"),
        ("personalInfo/i1", "value"),
        ("speakAs/pronouns/p1", "pronouns"),
    ] {
        let found = problems_of(|v| {
            let mut at = &mut *v;
            for part in path.split('/') {
                at = &mut at[part];
            }
            at.as_object_mut().unwrap().remove(member);
        });
        assert_eq!(found, [format!("/{path}")], "without {member}");
    }
}

#[test]
fn an_online_service_needs_something_to_find_it_by() {
    let found = problems_of(|v| {
        v["onlineServices"]["s1"] = json!({"@type": "OnlineService", "label": "x"});
    });
    assert_eq!(found, ["/onlineServices/s1"]);
}

#[test]
fn a_wrong_type_name_and_a_pref_out_of_range_are_found() {
    let found = problems_of(|v| {
        v["media"]["m1"]["@type"] = json!("Photo");
        v["phones"]["p1"]["pref"] = json!(0);
        v["speakAs"]["pronouns"]["p1"]["pref"] = json!(101);
    });
    assert!(found.contains(&"/media/m1/@type".to_owned()), "{found:?}");
    assert!(found.contains(&"/phones/p1/pref".to_owned()), "{found:?}");
    assert!(
        found.contains(&"/speakAs/pronouns/p1/pref".to_owned()),
        "{found:?}"
    );
}
