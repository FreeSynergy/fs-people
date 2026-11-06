//! Filters that combine, and people who appear once.

mod common;

use common::{person, repo};
use fs_people_core::{Card, ListFilter};
use serde_json::Map;

const CLUB: &str = "https://x/groups/club";

#[tokio::test]
async fn people_with_a_role_appear_once_and_filters_combine() {
    let r = repo().await;
    let ada = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    r.create(person("Adam", "b@x.org")).await.unwrap();
    let org = r
        .create(Card::new("org", "Adacorp"))
        .await
        .unwrap()
        .envelope
        .id;
    for holder in [ada, org] {
        r.assign_role(holder, "member", Some(CLUB), Map::new())
            .await
            .unwrap();
    }
    r.assign_role(ada, "member", Some("https://x/groups/b"), Map::new())
        .await
        .unwrap();
    assert_eq!(r.people_with_role("member", None).await.unwrap().len(), 2);

    let names = |filter: ListFilter| {
        let r = r.clone();
        async move {
            let found = r.list(&filter).await.unwrap();
            found
                .into_iter()
                .map(|o| o.envelope.name)
                .collect::<Vec<_>>()
        }
    };
    let term = Some("ada".to_string());
    let all = names(ListFilter {
        term: term.clone(),
        ..Default::default()
    })
    .await;
    assert_eq!(all, ["Ada", "Adacorp", "Adam"]);
    let role = Some("member".to_string());
    let held = names(ListFilter {
        term: term.clone(),
        role: role.clone(),
        ..Default::default()
    })
    .await;
    assert_eq!(held, ["Ada", "Adacorp"]);
    let filter = ListFilter {
        term,
        role: role.clone(),
        kind: Some("org".into()),
        ..Default::default()
    };
    assert_eq!(names(filter).await, ["Adacorp"]);
    let filter = ListFilter {
        role,
        limit: Some(1),
        offset: 1,
        ..Default::default()
    };
    assert_eq!(names(filter).await, ["Adacorp"]);
}
