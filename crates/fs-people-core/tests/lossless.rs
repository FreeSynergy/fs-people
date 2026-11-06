//! Numbers stay what they were, or they do not come in: plain `serde_json` keeps these, and refuses nothing
//! that a plain reader of the same data would stumble over. Also what a vCard export writes.

mod common;

use common::{person, repo};
use fs_people_core::VCardFormat;
use fs_people_core::{Exporter, Importer, JsContactFormat, RoleRegistry};
use serde_json::{Map, Value, json};

const BIG: &str = "123456789012345678901234567890";

#[tokio::test]
async fn unknown_properties_and_details_keep_what_comes_in() {
    let r = repo().await;
    let text = r#"{"@type":"Card","version":"1.0","uid":"u","name":{"full":"A"},"example.org:n":{"i":18446744073709551615,"f":1.5,"small":7}}"#;
    let report = r.import_jscontact(text).await.unwrap();
    assert_eq!(report.failed, Vec::<String>::new());
    let id = report.created[0];
    let again = r.get(id).await.unwrap().unwrap().content;
    assert_eq!(
        again["example.org:n"].to_string(),
        r#"{"f":1.5,"i":18446744073709551615,"small":7}"#
    );
    let card = r.get_card(id).await.unwrap().unwrap();
    assert_eq!(card.vendor("example.org:n").unwrap()["f"], json!(1.5));
    let details: Map<String, Value> = json!({"f": 1.5}).as_object().unwrap().clone();
    let config =
        json!([{"id": "o", "names": {"en": "O"}, "owner": "fs-people", "scope": "public"}]);
    r.swap_role_config(&config.to_string()).unwrap();
    let held = r.assign_role(id, "o", None, details.clone()).await.unwrap();
    assert_eq!(held.details, details);
    assert_eq!(r.roles_of(id).await.unwrap()[0].details, details);
}

#[tokio::test]
async fn an_import_names_the_cards_with_a_number_it_cannot_keep_and_stores_the_rest() {
    let r = repo().await;
    let card = |uid: &str, n: &str| {
        format!(
            r#"{{"@type":"Card","version":"1.0","uid":"{uid}","name":{{"full":"{uid}"}},"example.org:n":{n}}}"#
        )
    };
    let text = format!(
        "[{},{},{},{}]",
        card("big", BIG),
        card("fine", "1.5"),
        card("zero", "-0"),
        card("long", "1.2345678901234567890")
    );
    let report = r.import_jscontact(&text).await.unwrap();
    assert_eq!(report.created.len(), 1, "{report:?}");
    assert_eq!(
        r.get(report.created[0])
            .await
            .unwrap()
            .unwrap()
            .envelope
            .name,
        "fine"
    );
    let failed = report.failed.join(" | ");
    assert_eq!(report.failed.len(), 3, "{failed}");
    for token in [BIG, "-0", "1.2345678901234567890"] {
        assert!(failed.contains(token), "{failed}");
    }
    // A number no plain parser can read at all: the whole text is not JSON to it.
    assert!(
        r.import_jscontact(&format!("[{}]", card("inf", "1e400")))
            .await
            .is_err()
    );
    assert!(fs_people_core::validate::check_exact_numbers(BIG).is_err());

    let items = JsContactFormat.import(&card("big", BIG)).unwrap();
    assert!(items[0].is_err());
    assert!(
        RoleRegistry::check(&format!(
            r#"[{{"id":"x","names":{{"en":"X"}},"owner":"o","example.org:n":{BIG}}}]"#
        ))
        .is_err()
    );
}

#[tokio::test]
async fn a_card_made_in_code_cannot_carry_a_negative_zero() {
    let r = repo().await;
    let mut card = person("Ada", "a@x.org");
    card.extra.insert("example.org:n".into(), json!(-0.0));
    assert!(r.create(card).await.is_err());
}

#[test]
fn a_fractional_vendor_property_comes_out_as_a_number() {
    let mut card = person("Ada", "a@x.org");
    card.extra.insert("example.com:ratio".into(), json!(1.5));
    let text = VCardFormat.export(&card).unwrap();
    assert!(
        text.contains(r#"JSPROP;JSPTR="example.com:ratio":1.5"#),
        "{text}"
    );
    assert!(!text.contains("serde_json"), "{text}");
}

#[tokio::test]
async fn a_float_in_a_vcard_survives_import_and_export() {
    let r = repo().await;
    let vcf =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:u1\r\nFN:Ada\r\nX-FOO;VALUE=FLOAT:1.5\r\nEND:VCARD\r\n";
    let report = r.import_vcard(vcf).await.unwrap();
    assert_eq!(report.created.len(), 1, "{report:?}");
    let text = r.export_vcard(report.created[0]).await.unwrap();
    assert!(text.contains("X-FOO") && text.contains("1.5"), "{text}");
    assert!(!text.contains("serde_json"), "{text}");
}
