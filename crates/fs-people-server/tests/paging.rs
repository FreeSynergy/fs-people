//! `list` across pages: the order of the contract, every id, nothing twice.

mod common;

use common::{id_of, name_of, persons_of, uuids, with_server};
use serde_json::json;

#[tokio::test(flavor = "current_thread")]
async fn lists_page_in_name_order_through_the_cursor() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        // Created out of name order, so the order below is the provider's work.
        let cards: Vec<_> = (0..130)
            .rev()
            .map(|i| {
                json!({"@type": "Card", "version": "1.0", "uid": format!("uid-{i}"),
                    "name": {"full": format!("Person {i:03}")}})
            })
            .collect();
        let mut import = persons.import_text_request();
        import.get().set_media_type("application/jscontact+json");
        import.get().set_text(json!(cards).to_string().as_str());
        let report = import.send().promise.await.unwrap();
        let mut created = uuids(
            report
                .get()
                .unwrap()
                .get_report()
                .unwrap()
                .get_created()
                .unwrap(),
        );
        created.reverse(); // now in the order of the names

        let (mut names, mut ids) = (vec![], vec![]);
        let mut cursor: Vec<u8> = vec![];
        let mut sizes = vec![];
        loop {
            let mut req = persons.list_request();
            req.get().init_cursor().set_opaque(&cursor);
            let answer = req.send().promise.await.unwrap();
            let page = answer.get().unwrap().get_page().unwrap();
            let items = page.get_items().unwrap();
            sizes.push(items.len());
            names.extend(items.iter().map(name_of));
            ids.extend(items.iter().map(id_of));
            cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
            if cursor.is_empty() {
                break;
            }
        }
        assert_eq!(sizes, [100, 30]);
        let expected: Vec<_> = (0..130).map(|i| format!("Person {i:03}")).collect();
        assert_eq!(names, expected);
        assert_eq!(ids, created);
    })
    .await;
}
