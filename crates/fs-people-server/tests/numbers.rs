//! A number that cannot be kept exactly is refused at the door.

mod common;

use common::{BAD_REQUEST, expect_problem, persons_of, with_server};

#[tokio::test(flavor = "current_thread")]
async fn a_number_that_would_change_is_a_malformed_request() {
    with_server(|root| async move {
        let persons = persons_of(&root).await;
        for number in ["123456789012345678901234567890", "-0", "1.2345678901234567890", "1e400"] {
            let text = format!(
                r#"{{"@type":"Card","version":"1.0","uid":"n","name":{{"full":"N"}},"example.org:n":{number}}}"#
            );
            let mut inexact = persons.create_request();
            inexact.get().set_jscontact(text.as_str());
            expect_problem(inexact.send().promise.await, BAD_REQUEST);
        }
    })
    .await;
}
