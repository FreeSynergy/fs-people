//! Answers near the limit of a message: a page is cut, an answer without a cursor refuses in words.

mod common;

use common::{name_of, persons_of, problem_of, with_server};
use serde_json::json;

const BIG: usize = 20 * 1024 * 1024;

fn big_card(uid: &str, name: &str) -> String {
    json!({"@type": "Card", "version": "1.0", "uid": uid, "name": {"full": name},
        "emails": {"e1": {"address": "same@x.org"}},
        "notes": {"n1": {"note": "x".repeat(BIG)}}})
    .to_string()
}

#[tokio::test(flavor = "current_thread")]
async fn nothing_too_large_ends_the_connection() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        for (uid, name) in [("big-1", "A"), ("big-2", "B")] {
            let mut create = persons.create_request();
            create.get().set_jscontact(big_card(uid, name).as_str());
            create.send().promise.await.unwrap();
        }

        let mut seen = vec![];
        let mut cursor: Vec<u8> = vec![];
        loop {
            let mut req = persons.list_request();
            req.get().init_cursor().set_opaque(&cursor);
            let answer = req.send().promise.await.unwrap();
            let page = answer.get().unwrap().get_page().unwrap();
            let items = page.get_items().unwrap();
            assert_eq!(items.len(), 1, "two would pass the weight of a page");
            seen.push(name_of(items.get(0)));
            cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
            if cursor.is_empty() {
                break;
            }
        }
        assert_eq!(seen, ["A", "B"]);

        let mut find = persons.find_by_email_request();
        find.get().set_email("same@x.org");
        let refused = problem_of(find.send().promise.await);
        assert_eq!(refused.status, 413);
        assert!(refused.problem_type.ends_with("/people/too-large"));

        let answer = root.support_report_request().send().promise.await;
        assert!(answer.is_ok(), "the connection went with it");
    })
    .await;
}
