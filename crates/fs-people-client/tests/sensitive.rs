//! The server tells the two refusals of a sensitive card by two types, and the client by the type.
//!
//! Pattern: Integration tests over the real socket. The link refusal is asked for by a raw call of
//! `confirmSame`, which the client does not offer, to see the type on the wire.

mod common;

use common::{Served, add, person};
use fs_interfaces::people_capnp::people;
use fs_interfaces::rpc::{Problem, connect, write_object_id};
use fs_people_core::{Error, People, VCardFormat};

#[tokio::test(flavor = "multi_thread")]
async fn a_release_missing_and_a_link_refused_have_types_of_their_own() {
    let served = Served::start().await;
    let people = served.client();
    let mut secret = person("Secret", "s@x.org");
    secret.mark_sensitive(["pd:SexualPreference"]);
    let secret_id = people.create(secret).await.unwrap().envelope.id;
    let other = add(&people, person("Ada", "a@x.org")).await;

    let refused = people
        .export_released(&VCardFormat, secret_id, &[])
        .await
        .unwrap_err();
    assert!(matches!(refused, Error::NotReleased(_)), "{refused:?}");

    let socket = served.socket.clone();
    let seen = tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio::task::LocalSet::new().block_on(&runtime, async move {
            let root = connect::<people::Client>(&socket).await.unwrap();
            let persons = root.client.persons_request().send().promise.await.unwrap();
            let persons = persons.get().unwrap().get_persons().unwrap();
            let mut req = persons.confirm_same_request();
            write_object_id(req.get().init_a(), secret_id);
            write_object_id(req.get().init_b(), other);
            Problem::from_error(&req.send().promise.await.err().expect("refused")).unwrap()
        })
    })
    .await
    .unwrap();
    assert_eq!(
        seen.problem_type,
        "https://freesynergy.net/problems/people/sensitive-link"
    );
    assert_eq!(seen.status, 403);
}
