//! `PersonPages` and `PageWalk`: the list read page by page, by cursor, against the real server and
//! the real store.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use std::sync::Arc;

use common::{Served, add};
use fs_people_client::{Cursor, PageWalk, PersonPages};
use fs_people_core::{Card, Error, ListFilter, People};
use fs_store::ObjectId;

async fn fill(people: &dyn People, prefix: &str, count: usize) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    for i in 0..count {
        ids.push(add(people, Card::individual(format!("{prefix} {i:03}"))).await);
    }
    ids
}

fn term(t: &str) -> ListFilter {
    ListFilter {
        term: Some(t.into()),
        ..ListFilter::default()
    }
}

/// Every page of `filter`, with the sizes.
async fn all_pages(pages: &dyn PersonPages, filter: &ListFilter) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut page = pages.first(filter).await.unwrap();
    loop {
        out.push(page.items.iter().map(|o| o.envelope.name.clone()).collect());
        match page.next {
            Some(cursor) => page = pages.next(filter, &cursor).await.unwrap(),
            None => return out,
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn more_than_one_page_in_order_and_the_last_has_no_next() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "P", 230).await;
    let pages = all_pages(&people, &ListFilter::default()).await;
    let sizes: Vec<usize> = pages.iter().map(Vec::len).collect();
    assert_eq!(sizes, [100, 100, 30]);
    let names: Vec<String> = pages.concat();
    let expected: Vec<String> = (0..230).map(|i| format!("P {i:03}")).collect();
    assert_eq!(names, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn exactly_one_full_page_ends_without_an_empty_page_after_it() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "P", 100).await;
    let first = people.first(&ListFilter::default()).await.unwrap();
    assert_eq!(first.items.len(), 100);
    assert!(first.next.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_list_is_one_empty_page_without_next() {
    let served = Served::start().await;
    let people = served.client();
    let first = people.first(&ListFilter::default()).await.unwrap();
    assert!(first.items.is_empty() && first.next.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_search_term_pages_as_well_and_only_what_matches() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "Alpha", 120).await;
    fill(&people, "Beta", 5).await;
    let alpha = all_pages(&people, &term("alpha")).await;
    assert_eq!(alpha.iter().map(Vec::len).collect::<Vec<_>>(), [100, 20]);
    assert!(alpha.concat().iter().all(|n| n.starts_with("Alpha")));
    let beta = all_pages(&people, &term("beta")).await;
    assert_eq!(beta.iter().map(Vec::len).collect::<Vec<_>>(), [5]);
    assert!(all_pages(&people, &term("nothing")).await[0].is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cursor_that_the_server_did_not_hand_out_is_a_clear_error() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "P", 3).await;
    for bytes in [vec![9, 9], vec![1, 2, 3], vec![7; 9], b"nonsense".to_vec()] {
        let got = people
            .next(&ListFilter::default(), &Cursor::from_bytes(bytes))
            .await;
        match got {
            Err(Error::Store(fs_store::Error::Invalid(text))) => {
                assert!(text.contains("cursor"), "{text}");
            }
            other => panic!("expected an invalid cursor, got {other:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_added_before_the_boundary_comes_round_once_more() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "P", 150).await;
    let filter = ListFilter::default();
    let first = people.first(&filter).await.unwrap();
    // The cursor is a place in the list today (`fs-interfaces` page.rs): a person who sorts before
    // everything moves the boundary by one, so the last of page one comes again. Nothing errs.
    add(&people, Card::individual("A new first")).await;
    let second = people.next(&filter, &first.next.unwrap()).await.unwrap();
    assert_eq!(second.items.len(), 51);
    assert_eq!(second.items[0].envelope.id, first.items[99].envelope.id);
    assert!(second.next.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_removed_before_the_boundary_skips_the_next_one() {
    let served = Served::start().await;
    let people = served.client();
    let ids = fill(&people, "P", 150).await;
    let filter = ListFilter::default();
    let first = people.first(&filter).await.unwrap();
    people.delete(ids[0]).await.unwrap();
    let second = people.next(&filter, &first.next.unwrap()).await.unwrap();
    // 149 are left; the place 100 is now the 101st of the old list: P 100 was skipped.
    assert_eq!(second.items.len(), 49);
    assert_eq!(second.items[0].envelope.name, "P 101");
    assert!(second.next.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_walk_gives_page_after_page_then_none_and_keeps_saying_none() {
    let served = Served::start().await;
    let people = Arc::new(served.client());
    fill(people.as_ref(), "P", 130).await;
    let mut walk = PageWalk::new(people, ListFilter::default());
    assert_eq!(walk.next_page().await.unwrap().unwrap().items.len(), 100);
    assert_eq!(walk.next_page().await.unwrap().unwrap().items.len(), 30);
    assert!(walk.next_page().await.unwrap().is_none());
    assert!(walk.next_page().await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_walk_that_met_an_error_stays_where_it_was() {
    let mut served = Served::start().await;
    let people = Arc::new(served.client());
    fill(people.as_ref(), "P", 130).await;
    let mut walk = PageWalk::new(people.clone(), ListFilter::default());
    assert_eq!(walk.next_page().await.unwrap().unwrap().items.len(), 100);
    served.stop();
    assert!(walk.next_page().await.is_err());
    assert!(walk.next_page().await.is_err(), "still at the same place");
}

/// What the server checks of a cursor is its form (a version byte and a position), nothing else
/// (`fs-interfaces` page.rs). These tests pin that, so a stronger cursor there shows up here.
#[tokio::test(flavor = "multi_thread")]
async fn a_well_formed_cursor_is_taken_as_a_place_whoever_made_it_and_for_whatever_filter() {
    let served = Served::start().await;
    let people = served.client();
    fill(&people, "Alpha", 150).await;
    fill(&people, "Beta", 5).await;
    // Made up, but of the right form: version 1 and the place 3.
    let mut made_up = vec![1u8];
    made_up.extend_from_slice(&3u64.to_le_bytes());
    let got = people
        .next(&ListFilter::default(), &Cursor::from_bytes(made_up))
        .await
        .unwrap();
    assert_eq!(got.items[0].envelope.name, "Alpha 003");

    // The cursor of the whole list (place 100) under the filter "beta": no error, the wrong place.
    let whole = people.first(&ListFilter::default()).await.unwrap();
    let wrong = people
        .next(&term("beta"), &whole.next.unwrap())
        .await
        .unwrap();
    assert!(wrong.items.is_empty() && wrong.next.is_none());
}
