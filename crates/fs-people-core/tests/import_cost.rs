//! What a large import costs the others: no store for what fails, a turn per card.

mod common;
#[path = "common/gated.rs"]
mod gated;

use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::ME;
use fs_objects::{Store, TursoStore};
use fs_people_core::{Card, JsContactFormat, People, PeopleRepo};
use gated::Gated;

struct Fixture {
    r: Arc<PeopleRepo>,
}

async fn fixture() -> (Fixture, Arc<Gated>) {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let gated = Arc::new(Gated::new(store));
    let r = Arc::new(PeopleRepo::with_backend(gated.clone(), gated.clone(), ME));
    (Fixture { r }, gated)
}

#[tokio::test(flavor = "current_thread")]
async fn a_file_of_failures_never_asks_the_store_and_bounds_its_report() {
    let (f, gated) = fixture().await;
    let before = gated.calls.load(Ordering::SeqCst);
    let text = format!("[{}]", vec!["{}"; 300_000].join(","));
    let report = People::import(&*f.r, &JsContactFormat, &text)
        .await
        .unwrap();
    assert_eq!(gated.calls.load(Ordering::SeqCst), before);
    assert_eq!(report.created, [] as [fs_objects::ObjectId; 0]);
    let last = report.failed.last().unwrap();
    let more: usize = last.strip_suffix(" more").map_or(0, |n| n.parse().unwrap());
    assert_eq!(more + report.failed.len() - usize::from(more > 0), 300_000);
    let bytes: usize = report.failed.iter().map(String::len).sum();
    assert!(bytes <= 5 * 1024 * 1024 + 64, "{bytes}");
}

#[tokio::test(flavor = "current_thread")]
async fn others_write_between_the_cards_of_an_import() {
    let (f, gated) = fixture().await;
    let cards: Vec<String> = (0..50)
        .map(|i| {
            format!(r#"{{"@type":"Card","version":"1.0","uid":"i{i}","name":{{"full":"I"}}}}"#)
        })
        .collect();
    let text = format!("[{}]", cards.join(","));
    gated.armed.store(1, Ordering::SeqCst);
    let r = f.r.clone();
    let import = tokio::spawn(async move { People::import(&*r, &JsContactFormat, &text).await });
    gated.arrived.notified().await;
    let r = f.r.clone();
    let other = tokio::spawn(async move {
        People::create(&*r, Card::individual("Other"))
            .await
            .unwrap();
        People::list(&*r, &fs_people_core::ListFilter::default())
            .await
            .unwrap()
            .len()
    });
    tokio::task::yield_now().await;
    gated.gate.add_permits(1000);
    let when_done = other.await.unwrap();
    assert!(
        when_done < 50 + 1,
        "the other write waited for the whole file: {when_done}"
    );
    assert_eq!(import.await.unwrap().unwrap().created.len(), 50);
}
