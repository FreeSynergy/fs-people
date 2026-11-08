//! Linking profiles to people (ADR 0058): proposals only ask, a "no" is kept, the user links and
//! unlinks by hand, and what is taken over is a separate, unbuilt step.

mod common;

use common::{person, repo};
use fs_objects::ObjectId;
use fs_people_core::{Adoption, Card, Error, ProposalState};

async fn two_with_one_address(r: &fs_people_core::PeopleRepo) -> (ObjectId, ObjectId) {
    let a = r.create(person("Ada", "ada@example.org")).await.unwrap();
    let b = r
        .create(person("A. Lovelace", "ADA@example.org"))
        .await
        .unwrap();
    let (a, b) = (a.envelope.id, b.envelope.id);
    if a < b { (a, b) } else { (b, a) }
}

async fn proposed(r: &fs_people_core::PeopleRepo) -> Vec<(ObjectId, ObjectId)> {
    r.find_same_person_proposals()
        .await
        .unwrap()
        .iter()
        .map(|p| (p.a, p.b))
        .collect()
}

#[tokio::test]
async fn a_rejected_pair_is_not_proposed_again_until_reopened() {
    let r = repo().await;
    let (a, b) = two_with_one_address(&r).await;
    assert_eq!(proposed(&r).await, [(a, b)]);
    assert_eq!(r.same_person_state(a, b).await.unwrap(), None);

    let rejection = r.reject_same_person(b, a).await.unwrap();
    assert!(proposed(&r).await.is_empty(), "asked once, not again");
    assert_eq!(
        r.same_person_state(a, b).await.unwrap(),
        Some(ProposalState::Rejected)
    );
    assert!(matches!(
        r.reject_same_person(a, b).await,
        Err(Error::Conflict(_))
    ));

    r.reopen_same_person(rejection.envelope.id).await.unwrap();
    assert_eq!(proposed(&r).await, [(a, b)]);
    assert!(r.reopen_same_person(rejection.envelope.id).await.is_err());
}

#[tokio::test]
async fn the_user_links_by_hand_also_a_rejected_pair_and_unlinking_does_not_ask_again() {
    let r = repo().await;
    let (a, b) = two_with_one_address(&r).await;
    r.reject_same_person(a, b).await.unwrap();

    let link = r.confirm_same_person(a, b).await.unwrap();
    assert_eq!(
        r.same_person_state(a, b).await.unwrap(),
        Some(ProposalState::Confirmed)
    );
    assert_eq!(r.linked_to(a).await.unwrap(), [b]);
    assert!(matches!(
        r.reject_same_person(a, b).await,
        Err(Error::Conflict(_))
    ));

    r.unlink(link.envelope.id).await.unwrap();
    assert!(r.linked_to(a).await.unwrap().is_empty());
    assert_eq!(
        r.same_person_state(a, b).await.unwrap(),
        Some(ProposalState::Rejected),
        "taking a link back is an answer"
    );
    assert!(proposed(&r).await.is_empty());
}

#[tokio::test]
async fn a_hand_made_link_needs_no_proposal() {
    let r = repo().await;
    let a = r
        .create(person("Ada", "a@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let b = r
        .create(person("Grace", "g@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    assert!(proposed(&r).await.is_empty());
    r.confirm_same_person(a, b).await.unwrap();
    assert_eq!(r.linked_to(b).await.unwrap(), [a]);
}

#[tokio::test]
async fn name_at_the_same_organisation_is_a_proposal_a_name_alone_is_not() {
    let r = repo().await;
    let mut x = Card::individual("Ada Lovelace");
    x.add_organization("Analytical Engines");
    let mut y = Card::individual("ada lovelace");
    y.add_organization("ANALYTICAL ENGINES");
    let z = Card::individual("Ada Lovelace");
    let mut w = Card::individual("Ada Lovelace");
    w.add_organization("Elsewhere Ltd");
    let ids: Vec<_> = {
        let mut v = vec![];
        for c in [x, y, z, w] {
            v.push(r.create(c).await.unwrap().envelope.id);
        }
        v
    };
    let found = r.find_same_person_proposals().await.unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    let (a, b) = if ids[0] < ids[1] {
        (ids[0], ids[1])
    } else {
        (ids[1], ids[0])
    };
    assert_eq!((found[0].a, found[0].b), (a, b));
    assert_eq!(
        found[0].reasons[0].kind,
        fs_people_core::MatchKind::NameAndOrganisation
    );
}

#[tokio::test]
async fn deleting_a_card_deletes_the_rejections_that_mention_it() {
    let r = repo().await;
    let (a, b) = two_with_one_address(&r).await;
    r.reject_same_person(a, b).await.unwrap();
    r.delete(a).await.unwrap();
    assert_eq!(r.same_person_state(a, b).await.unwrap(), None);
}

#[tokio::test]
async fn a_missing_card_and_itself_cannot_be_rejected() {
    let r = repo().await;
    let (a, _) = two_with_one_address(&r).await;
    assert!(matches!(
        r.reject_same_person(a, ObjectId::new()).await,
        Err(Error::NotFound(_))
    ));
    assert!(r.reject_same_person(a, a).await.is_err());
}

struct Nobody;
impl Adoption for Nobody {}

#[tokio::test]
async fn taking_over_is_an_interface_only() {
    let n = Nobody;
    let link = ObjectId::new();
    assert!(matches!(n.offered(link).await, Err(Error::Unsupported(_))));
    assert!(matches!(
        n.adopt(link, &[]).await,
        Err(Error::Unsupported(_))
    ));
}

#[tokio::test]
async fn a_sensitive_card_is_never_named_in_a_rejection() {
    let r = repo().await;
    let mut secret = person("Secret", "same@example.org");
    secret.mark_sensitive(["pd:HealthRecord"]);
    let a = r
        .create(person("Ada", "same@example.org"))
        .await
        .unwrap()
        .envelope
        .id;
    let s = r.create(secret).await.unwrap().envelope.id;
    assert!(matches!(
        r.reject_same_person(a, s).await,
        Err(Error::Sensitive(_))
    ));
}
