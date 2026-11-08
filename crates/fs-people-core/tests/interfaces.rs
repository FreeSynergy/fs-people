//! The program is usable through its traits alone: `People`, `Importer`, `Exporter`,
//! `MatchStrategy` (ADR 0043).

mod common;

use std::sync::Arc;

use common::{ME, person, repo, setup};
use fs_people_core::{
    Candidate, Card, Exporter, ImportedCard, Importer, JsContactFormat, MatchKind, MatchReason,
    MatchStrategy, People, PeopleRepo, Result, SameAsProposal, VCardFormat,
};

#[tokio::test]
async fn contact_book_works_as_a_trait_object() {
    let book: Arc<dyn People> = Arc::new(repo().await);
    let o = book.create(person("Ada", "ada@example.org")).await.unwrap();
    assert_eq!(book.search("ada").await.unwrap().len(), 1);
    assert!(
        book.export_vcard(o.envelope.id)
            .await
            .unwrap()
            .contains("FN:Ada")
    );
    book.delete(o.envelope.id).await.unwrap();
    assert!(book.get(o.envelope.id).await.unwrap().is_none());
}

struct OneLine;

impl Importer for OneLine {
    fn media_type(&self) -> &'static str {
        "text/plain"
    }
    fn import(&self, text: &str) -> Result<Vec<Result<ImportedCard>>> {
        Ok(text
            .lines()
            .map(|l| {
                Ok(ImportedCard {
                    card: Card::individual(l),
                    original: None,
                })
            })
            .collect())
    }
}

#[tokio::test]
async fn a_new_format_plugs_in_as_an_importer() {
    let r = repo().await;
    let rep = r.import(&OneLine, "Ada\nBob").await.unwrap();
    assert_eq!(rep.created.len(), 2);
    assert_eq!(VCardFormat.output_media_type(), "text/vcard");
    let json = JsContactFormat.export(&Card::individual("Cy")).unwrap();
    assert_eq!(
        r.import(&JsContactFormat, &json)
            .await
            .unwrap()
            .created
            .len(),
        1
    );
}

struct EveryoneIsEveryone;

impl MatchStrategy for EveryoneIsEveryone {
    fn propose(&self, c: &[Candidate]) -> Vec<SameAsProposal> {
        let reason = MatchReason {
            kind: MatchKind::Email,
            value: "fake".into(),
        };
        c.windows(2)
            .map(|w| {
                let (a, b) = if w[0].id < w[1].id {
                    (w[0].id, w[1].id)
                } else {
                    (w[1].id, w[0].id)
                };
                SameAsProposal {
                    a,
                    b,
                    reasons: vec![reason.clone()],
                }
            })
            .collect()
    }
}

#[tokio::test]
async fn the_match_strategy_is_swappable() {
    let (r, store) = setup().await;
    let r = r.with_match_strategy(Arc::new(EveryoneIsEveryone));
    r.create(person("A", "a@x.org")).await.unwrap();
    r.create(person("B", "b@x.org")).await.unwrap();
    assert_eq!(r.find_same_person_proposals().await.unwrap().len(), 1);
    // the default strategy needs a shared contact point
    assert_eq!(
        PeopleRepo::new(store, ME)
            .find_same_person_proposals()
            .await
            .unwrap(),
        [] as [fs_people_core::SameAsProposal; 0]
    );
}
