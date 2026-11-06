//! vCard import / export and original keeping (ADR 0030).

mod common;

use common::repo;
use fs_people_core::vcard;

#[tokio::test]
async fn vcard_import_keeps_original_and_exports() {
    let r = repo().await;
    let vcf = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:01a1aaef-ba80-7f7a-8dff-000000000001\r\nFN:Ada Lovelace\r\nN:Lovelace;Ada;;;\r\nEMAIL;TYPE=work:Ada@Example.org\r\nTEL;VALUE=uri:tel:+49-30-1234567\r\nORG:Analytical Engines\r\nURL:https://example.org/ada\r\nNOTE:First programmer\r\nKIND:individual\r\nADR;TYPE=home:;;1 Main St;Berlin;;10115;Germany\r\nX-CUSTOM:keep me\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Bob Old\r\nEMAIL:bob@old.example\r\nEND:VCARD\r\n";
    let rep = r.import_vcard(vcf).await.unwrap();
    assert_eq!(rep.created.len(), 2, "{rep:?}");
    let ada = r.get(rep.created[0]).await.unwrap().unwrap();
    // original kept byte for byte, per card
    let orig = ada.original.as_ref().unwrap();
    assert_eq!(orig.media_type, "text/vcard");
    assert!(
        std::str::from_utf8(&orig.bytes)
            .unwrap()
            .starts_with("BEGIN:VCARD\r\nVERSION:4.0")
    );
    assert!(
        std::str::from_utf8(&orig.bytes)
            .unwrap()
            .ends_with("END:VCARD\r\n")
    );
    assert!(!std::str::from_utf8(&orig.bytes).unwrap().contains("Bob"));
    assert_eq!(ada.envelope.name, "Ada Lovelace");
    assert_eq!(
        ada.envelope.id.to_string(),
        "01a1aaef-ba80-7f7a-8dff-000000000001"
    );
    assert_eq!(ada.index["email"], ["ada@example.org"]);
    assert_eq!(ada.index["phone"], ["+49301234567"]);
    assert_eq!(ada.index["org"], ["analytical engines"]);
    // v3 card without UID got one
    let bob = r.get_card(rep.created[1]).await.unwrap().unwrap();
    assert!(bob.uid.starts_with("urn:uuid:"));
    // re-import skips
    let again = r.import_vcard(vcf).await.unwrap();
    assert_eq!((again.created.len(), again.skipped.len()), (0, 2));
    // export
    let out = r.export_vcard(ada.envelope.id).await.unwrap();
    for needle in [
        "BEGIN:VCARD",
        "VERSION:4.0",
        "FN:Ada Lovelace",
        "ada@example.org".to_lowercase().as_str(),
        "UID:urn:uuid:01a1aaef",
        "END:VCARD",
    ] {
        assert!(
            out.to_lowercase().contains(&needle.to_lowercase()),
            "{needle} in\n{out}"
        );
    }
    // exported vCard re-imports to an equivalent core card
    let back = vcard::card_from_vcard(&out).unwrap();
    let a = r.get_card(ada.envelope.id).await.unwrap().unwrap();
    assert_eq!(back.uid, a.uid);
    assert_eq!(back.display_name(), a.display_name());
    assert_eq!(
        back.emails
            .values()
            .map(|e| e.address.clone())
            .collect::<Vec<_>>(),
        a.emails
            .values()
            .map(|e| e.address.clone())
            .collect::<Vec<_>>()
    );
    // the original cannot be replaced by an update (store rule), but is kept
    let o = r.update(ada.envelope.id, a).await.unwrap();
    assert_eq!(o.original.as_ref(), Some(orig));
}

#[test]
fn vcard_garbage_is_reported_not_panicking() {
    assert!(vcard::import_vcards("hello world").is_empty());
    let r = vcard::import_vcards("BEGIN:VCARD\nVERSION:4.0\nEND:VCARD\n");
    // no FN / name: still a card (uid generated) or a clean error, never a panic
    assert_eq!(r.len(), 1);
}
