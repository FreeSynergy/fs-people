//! The commands of the `Person` description that need more than the description: patch, check, suggest.
//! They run against the trait `People`, on a real store.

mod common;

use common::{person, repo};
use fs_people_core::{Card, Error, PatchObject, People};
use serde_json::json;

fn patch(v: serde_json::Value) -> PatchObject {
    PatchObject::from_value(v).unwrap()
}

#[tokio::test]
async fn a_patch_sets_removes_and_leaves_the_rest() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.add_phone("+49 30 123456");
    let id = r.create(c).await.unwrap().envelope.id;
    let before = r.get_card(id).await.unwrap().unwrap();
    let phone_id = before.phones.keys().next().unwrap().clone();
    let email_id = before.emails.keys().next().unwrap().clone();
    let p = patch(json!({
        format!("emails/{email_id}/address"): "ada@example.com",
        format!("phones/{phone_id}"): null,
        "nicknames": {"n1": {"name": "Countess"}},
        "name/full": "Ada L."
    }));
    let stored = People::patch(&r, id, &p).await.unwrap();
    let after: Card = serde_json::from_value(stored.content).unwrap();
    assert_eq!(after.emails[&email_id].address, "ada@example.com");
    assert!(after.phones.is_empty());
    assert_eq!(after.nicknames["n1"].name, "Countess");
    assert_eq!(after.name.unwrap().full.as_deref(), Some("Ada L."));
    assert_eq!(after.uid, before.uid);
}

#[tokio::test]
async fn a_path_through_what_is_not_there_is_refused_not_made_up() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(
        People::patch(&r, id, &patch(json!({"nicknames/n1": {"name": "x"}})))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_patch_that_cannot_be_applied_changes_nothing() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let before = r.get(id).await.unwrap().unwrap().content;
    for bad in [
        json!({"name/full": "X", "nope/deeper": 1}),
        json!({"name/components/0/value": "X"}),
        json!({"name": {"full": "A"}, "name/full": "B"}),
        json!({"uid": "urn:uuid:other"}),
        json!({"emails/e1/address": ""}),
        json!({"": 1}),
        json!({"name/~2": 1}),
    ] {
        let err = People::patch(&r, id, &patch(bad.clone()))
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Invalid(_) | Error::Conflict(_)),
            "{bad}: {err}"
        );
    }
    assert_eq!(r.get(id).await.unwrap().unwrap().content, before);
}

#[tokio::test]
async fn a_key_with_a_slash_is_written_with_tilde_one() {
    let r = repo().await;
    let id = r
        .create(person("Ada", "ada@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let stored = People::patch(&r, id, &patch(json!({"example.org:a~1b": 1})))
        .await
        .unwrap();
    assert_eq!(stored.content["example.org:a/b"], json!(1));
}

#[tokio::test]
async fn a_patch_of_nobody_is_not_found_and_a_text_that_is_no_object_is_refused() {
    let r = repo().await;
    let nobody = fs_objects::ObjectId::new();
    assert!(matches!(
        People::patch(&r, nobody, &patch(json!({"a/b": 1}))).await,
        Err(Error::NotFound(_))
    ));
    assert!(PatchObject::parse("[1]").is_err());
    assert!(PatchObject::parse("not json").is_err());
}

#[tokio::test]
async fn check_reports_the_values_that_break_a_rule_of_their_basic_type() {
    let r = repo().await;
    let mut c = person("Ada", "ada@example.org");
    c.add_phone("+49 30 123456");
    let id = r.create(c).await.unwrap().envelope.id;
    assert!(People::check(&r, id).await.unwrap().is_empty());
    // The program accepts what is typed; the check says what is wrong with it.
    let p = patch(json!({
        "emails/e1/address": "not an address",
        "links": {"w1": {"uri": "javascript:alert(1)"}},
        "preferredLanguages": {"l1": {"language": "not a tag!"}}
    }));
    People::patch(&r, id, &p).await.unwrap();
    let found = People::check(&r, id).await.unwrap();
    let at: Vec<&str> = found.iter().map(|i| i.path.as_str()).collect();
    assert!(at.contains(&"/emails/e1"), "{found:?}");
    assert!(at.contains(&"/links/w1"), "{found:?}");
    assert!(at.contains(&"/preferredLanguages/l1"), "{found:?}");
    let link = found.iter().find(|i| i.path == "/links/w1").unwrap();
    assert_eq!(link.message, "uri.unsafe-scheme", "a code, not a sentence");
}

#[tokio::test]
async fn check_of_nobody_is_not_found() {
    let r = repo().await;
    assert!(matches!(
        People::check(&r, fs_objects::ObjectId::new()).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn suggest_completes_an_address_with_the_domains_in_the_book() {
    let r = repo().await;
    for (n, e) in [
        ("A", "a@example.org"),
        ("B", "b@example.org"),
        ("C", "c@example.com"),
        ("D", "d@corp.test"),
    ] {
        r.create(person(n, e)).await.unwrap();
    }
    let all = People::suggest(&r, "emails", "ada").await.unwrap();
    assert_eq!(
        all,
        ["ada@example.org", "ada@corp.test", "ada@example.com"],
        "most used first, then by name"
    );
    let some = People::suggest(&r, "emails", "ada@ex").await.unwrap();
    assert_eq!(some, ["ada@example.org", "ada@example.com"]);
    assert!(People::suggest(&r, "emails", "").await.unwrap().is_empty());
}

#[tokio::test]
async fn suggest_never_learns_from_someone_sensitive() {
    let r = repo().await;
    r.create(person("Open", "o@open.test")).await.unwrap();
    let mut secret = person("Secret", "s@secret.test");
    secret.mark_sensitive(["pd:HealthRecord"]);
    r.create(secret).await.unwrap();
    let got = People::suggest(&r, "emails", "x").await.unwrap();
    assert_eq!(got, ["x@open.test"]);
}

#[tokio::test]
async fn suggest_for_a_field_it_does_not_complete_is_refused() {
    let r = repo().await;
    assert!(matches!(
        People::suggest(&r, "uid", "x").await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        People::suggest(&r, "nope", "x").await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn suggest_offers_the_choices_of_a_choice_field() {
    let r = repo().await;
    let got = People::suggest(&r, "grammaticalGender", "fem")
        .await
        .unwrap();
    assert_eq!(got, ["feminine"]);
}
