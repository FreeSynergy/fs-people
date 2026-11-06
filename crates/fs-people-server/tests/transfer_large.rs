//! A small file that fails in millions of places: the report still arrives, and the connection stays.

mod common;

use common::{card, id_of, persons_of, uuids, with_server};

async fn id_by_uid(
    persons: &fs_interfaces::people_capnp::persons::Client,
    uid: &str,
) -> uuid::Uuid {
    let mut get = persons.get_by_uid_request();
    get.get().set_uid(uid);
    let answer = get.send().promise.await.unwrap();
    assert!(answer.get().unwrap().get_found(), "{uid}");
    id_of(answer.get().unwrap().get_person().unwrap())
}

#[tokio::test(flavor = "current_thread")]
async fn a_small_file_that_fails_millions_of_times_keeps_the_connection_and_its_report() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        let ones = "1,".repeat(2_000_000);
        let text = format!(
            "[{},{ones}{}]",
            card("first", "First", "f@x.org"),
            card("last", "Last", "l@x.org")
        );
        let mut import = persons.import_text_request();
        import.get().set_media_type("application/jscontact+json");
        import.get().set_text(text.as_str());
        let answer = import.send().promise.await.unwrap();
        let report = answer.get().unwrap().get_report().unwrap();
        let mut created = uuids(report.get_created().unwrap());
        created.sort_unstable();
        let mut known = vec![
            id_by_uid(&persons, "first").await,
            id_by_uid(&persons, "last").await,
        ];
        known.sort_unstable();
        assert_eq!(created, known, "everything stored is in the report");
        let failed = report.get_failed().unwrap();
        let last = failed.get(failed.len() - 1).unwrap().to_str().unwrap();
        let more: u64 = last.strip_suffix(" more").unwrap().parse().unwrap();
        assert_eq!(more + u64::from(failed.len()) - 1, 2_000_000);
    })
    .await;
}
