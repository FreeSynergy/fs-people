//! A server that is not there, or goes away: an error, not a hang.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use std::time::Duration;

use common::{Served, person};
use fs_people_client::RpcPeople;
use fs_people_core::{Error, ListFilter, People};

#[test]
fn nothing_listening_is_an_error_at_connect() {
    let dir = tempfile::tempdir().unwrap();
    let got = RpcPeople::connect(&dir.path().join("none.sock"));
    assert!(got.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_stopped_answers_with_an_error_every_time() {
    let mut served = Served::start().await;
    let people = served.client();
    people.create(person("Ada", "a@x.org")).await.unwrap();
    served.stop();
    for _ in 0..2 {
        let got = tokio::time::timeout(Duration::from_secs(10), people.list(&ListFilter::default()))
            .await
            .expect("no hang");
        assert!(matches!(got, Err(Error::Store(fs_store::Error::Backend(_)))), "{got:?}");
    }
}
