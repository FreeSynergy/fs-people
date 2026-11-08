//! Groups / folders (ADR 0037).

mod common;

use common::{person, repo};
use fs_people_core::{Error, ListFilter};

#[tokio::test]
async fn groups_hold_members() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Bob", "b@x.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let g = r.create_group("Team").await.unwrap();
    let gid = g.envelope.id;
    assert_eq!(g.index["kind"], ["group"]);
    r.add_member(gid, a).await.unwrap();
    r.add_member(gid, b).await.unwrap();
    r.add_member(gid, b).await.unwrap(); // idempotent
    let names: Vec<_> = r
        .members(gid)
        .await
        .unwrap()
        .into_iter()
        .map(|o| o.envelope.name)
        .collect();
    assert_eq!(names.len(), 2);
    assert_eq!(r.groups_of(a).await.unwrap().len(), 1);
    assert!(
        matches!(r.add_member(a, b).await, Err(Error::Conflict(_))),
        "only groups have members"
    );
    assert!(matches!(
        r.add_member(gid, gid).await,
        Err(Error::Conflict(_))
    ));
    r.remove_member(gid, a).await.unwrap();
    assert_eq!(r.members(gid).await.unwrap().len(), 1);
    assert_eq!(
        r.groups_of(a).await.unwrap(),
        [] as [fs_objects::StoredObject; 0]
    );
    let groups = r
        .list(&ListFilter {
            kind: Some("group".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(groups.len(), 1);
}
