//! Writes through the traits at the same time, over a backend that yields at every step: none is lost.

mod common;

use common::person;
use common::yielding::yielding_repo;
use fs_people_core::{Card, People, RoleBook};
use serde_json::Map;

#[tokio::test(flavor = "current_thread")]
async fn members_added_at_once_all_stay() {
    let r = yielding_repo().await;
    let group = People::create_group(&*r, "All").await.unwrap().envelope.id;
    let mut members = vec![];
    for i in 0..10 {
        let card = person(&format!("M{i}"), "m@x.org");
        members.push(People::create(&*r, card).await.unwrap().envelope.id);
    }
    let mut calls = tokio::task::JoinSet::new();
    for member in members {
        let r = r.clone();
        calls.spawn(async move { People::add_member(&*r, group, member).await });
    }
    while let Some(done) = calls.join_next().await {
        done.unwrap().unwrap();
    }
    assert_eq!(People::members(&*r, group).await.unwrap().len(), 10);
}

#[tokio::test(flavor = "current_thread")]
async fn a_uid_is_taken_once_and_a_role_held_once() {
    let r = yielding_repo().await;
    let mut calls = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let r = r.clone();
        let mut card = Card::individual("Ada");
        card.uid = "one".into();
        calls.spawn(async move { People::create(&*r, card).await });
    }
    let mut ids = vec![];
    while let Some(done) = calls.join_next().await {
        if let Ok(stored) = done.unwrap() {
            ids.push(stored.envelope.id);
        }
    }
    assert_eq!(ids.len(), 1);

    let mut calls = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let r = r.clone();
        let id = ids[0];
        calls.spawn(async move {
            RoleBook::assign_role(&*r, id, "member", Some("https://x/g"), Map::new()).await
        });
    }
    while let Some(done) = calls.join_next().await {
        done.unwrap().unwrap();
    }
    assert_eq!(RoleBook::roles_of(&*r, ids[0]).await.unwrap().len(), 1);
}
