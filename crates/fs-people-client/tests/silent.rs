//! A server that says nothing: the wait ends, with an error that names the deadline.
//!
//! Pattern: Integration tests against the trait; the silent listener and the provider that never
//! answers are the only things here that are not the real server, because the real one answers.

mod common;

use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use common::{ME, Served};
use fs_interfaces::Capabilities;
use fs_people_client::{RpcPeople, Timeouts};
use fs_people_core::{Error, ListFilter, People, PeopleRepo, Result};
use fs_store::{StoredObject, TursoStore};

const SHORT: Duration = Duration::from_millis(300);

/// Reports itself, and never answers a list.
struct Hanging;

#[async_trait]
impl People for Hanging {
    fn capabilities(&self) -> Capabilities {
        Capabilities::new("hanging")
    }
    async fn get(&self, _: fs_store::ObjectId) -> Result<Option<StoredObject>> {
        Ok(None)
    }
    async fn list(&self, _: &ListFilter) -> Result<Vec<StoredObject>> {
        std::future::pending().await
    }
}

#[test]
fn a_listener_that_never_answers_ends_connect_with_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mute.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let keep = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
    let started = std::time::Instant::now();
    let timeouts = Timeouts {
        connect: SHORT,
        ..Timeouts::default()
    };
    let got = RpcPeople::connect_with(&path, timeouts);
    let said = got.err().expect("an error").to_string();
    assert!(said.contains("did not answer within 300ms"), "{said}");
    assert!(started.elapsed() < Duration::from_secs(5));
    drop(keep);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_that_is_never_answered_ends_with_the_deadline() {
    let store = TursoStore::open_in_memory().await.unwrap();
    let repo = Arc::new(PeopleRepo::new(Arc::new(store), ME));
    let served = Served::serving(Arc::new(Hanging), repo).await;
    let timeouts = Timeouts {
        call: SHORT,
        ..Timeouts::default()
    };
    let people = RpcPeople::connect_with(&served.socket, timeouts).unwrap();
    let outer = Duration::from_secs(10);
    let got = tokio::time::timeout(outer, people.list(&ListFilter::default()))
        .await
        .expect("the client's own deadline comes first");
    let said = got.unwrap_err();
    assert!(
        matches!(said, Error::Store(fs_store::Error::Backend(_))),
        "{said:?}"
    );
    assert!(
        said.to_string().contains("did not answer within 300ms"),
        "{said}"
    );
    // The connection is still good for what is answered.
    assert!(
        people
            .get(fs_store::ObjectId::new())
            .await
            .unwrap()
            .is_none()
    );
}
