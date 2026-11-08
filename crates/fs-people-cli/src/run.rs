//! Carrying out one command against a `people::Client`.
//!
//! Pattern: Command (executor) — one function per command, each a few contract calls. Errors come
//! back as the text of the problem the server sent; the contract's problem types are not reinterpreted.

use fs_interfaces::common_capnp;
use fs_interfaces::people_capnp::{people, person, persons};
use fs_interfaces::rpc::{read_envelope, write_uuid};
use fs_objects::ObjectId;
use serde_json::json;

use crate::args::Command;
use crate::show::{line, pretty};

type Out = Result<String, String>;

fn failed(e: &capnp::Error) -> String {
    e.to_string()
}

fn id_of(text: &str) -> Result<ObjectId, String> {
    text.trim()
        .parse()
        .map_err(|_| format!("{text:?} is not a person id"))
}

/// Run `command`; the text to print.
///
/// # Errors
/// The server refused (its problem text), or an id or a file could not be read.
pub async fn run(root: &people::Client, command: Command) -> Out {
    let persons = root
        .persons_request()
        .send()
        .promise
        .await
        .map_err(|e| failed(&e))?
        .get()
        .and_then(people::persons_results::Reader::get_persons)
        .map_err(|e| failed(&e))?;
    match command {
        Command::List(term) => list(&persons, term.as_deref()).await,
        Command::Show(id) => show(&persons, id_of(&id)?).await,
        Command::Add { name, email } => add(&persons, &name, email.as_deref()).await,
        Command::Delete(id) => delete(&persons, id_of(&id)?).await,
        Command::Find(email) => find(&persons, &email).await,
        Command::Export(id) => export(&persons, id_of(&id)?).await,
        Command::Import(path) => import(&persons, &path).await,
    }
}

fn one(p: person::Reader<'_>) -> Result<(ObjectId, String), String> {
    let id = read_envelope(p.get_envelope().map_err(|e| failed(&e))?)
        .map_err(|e| failed(&e))?
        .id;
    let text = p
        .get_jscontact()
        .map_err(|e| failed(&e))?
        .to_string()
        .map_err(|e| e.to_string())?;
    Ok((id, text))
}

async fn list(persons: &persons::Client, term: Option<&str>) -> Out {
    let mut lines = vec![];
    let mut cursor: Vec<u8> = vec![];
    loop {
        let mut req = persons.list_request();
        if let Some(term) = term {
            req.get().init_filter().set_term(term);
        }
        req.get().init_cursor().set_opaque(&cursor);
        let answer = req.send().promise.await.map_err(|e| failed(&e))?;
        let page = answer
            .get()
            .and_then(persons::list_results::Reader::get_page)
            .map_err(|e| failed(&e))?;
        for p in page.get_items().map_err(|e| failed(&e))? {
            let (id, text) = one(p)?;
            lines.push(line(&id.to_string(), &text));
        }
        let next = page
            .get_next()
            .and_then(common_capnp::cursor::Reader::get_opaque)
            .map_err(|e| failed(&e))?;
        if next.is_empty() {
            return Ok(lines.join("\n"));
        }
        if next == cursor.as_slice() {
            return Err("the server answered the same cursor twice; stopped".into());
        }
        cursor = next.to_vec();
    }
}

async fn show(persons: &persons::Client, id: ObjectId) -> Out {
    let mut req = persons.get_request();
    write_uuid(req.get().init_id(), Some(id.as_uuid()));
    let answer = req.send().promise.await.map_err(|e| failed(&e))?;
    let r = answer.get().map_err(|e| failed(&e))?;
    if !r.get_found() {
        return Err(format!("no person {id}"));
    }
    Ok(pretty(&one(r.get_person().map_err(|e| failed(&e))?)?.1))
}

async fn add(persons: &persons::Client, name: &str, email: Option<&str>) -> Out {
    let mut card = json!({
        "@type": "Card", "version": "1.0",
        "uid": ObjectId::new().to_string(), "name": {"full": name},
    });
    if let Some(email) = email {
        card["emails"] = json!({"e1": {"address": email}});
    }
    let mut req = persons.create_request();
    req.get().set_jscontact(card.to_string().as_str());
    let answer = req.send().promise.await.map_err(|e| failed(&e))?;
    let (id, text) = one(answer
        .get()
        .and_then(persons::create_results::Reader::get_person)
        .map_err(|e| failed(&e))?)?;
    Ok(line(&id.to_string(), &text))
}

async fn delete(persons: &persons::Client, id: ObjectId) -> Out {
    let mut req = persons.delete_request();
    write_uuid(req.get().init_id(), Some(id.as_uuid()));
    req.send().promise.await.map_err(|e| failed(&e))?;
    Ok(format!("deleted {id}"))
}

async fn find(persons: &persons::Client, email: &str) -> Out {
    let mut req = persons.find_by_email_request();
    req.get().set_email(email);
    let answer = req.send().promise.await.map_err(|e| failed(&e))?;
    let items = answer
        .get()
        .and_then(persons::find_by_email_results::Reader::get_items)
        .map_err(|e| failed(&e))?;
    let mut lines = vec![];
    for p in items {
        let (id, text) = one(p)?;
        lines.push(line(&id.to_string(), &text));
    }
    Ok(lines.join("\n"))
}

async fn export(persons: &persons::Client, id: ObjectId) -> Out {
    let mut req = persons.export_text_request();
    write_uuid(req.get().init_id(), Some(id.as_uuid()));
    req.get().set_media_type("text/vcard");
    let answer = req.send().promise.await.map_err(|e| failed(&e))?;
    answer
        .get()
        .and_then(persons::export_text_results::Reader::get_text)
        .map_err(|e| failed(&e))?
        .to_string()
        .map_err(|e| e.to_string())
}

async fn import(persons: &persons::Client, path: &std::path::Path) -> Out {
    let text = read_limited(path)?;
    let mut req = persons.import_text_request();
    req.get().set_media_type("text/vcard");
    req.get().set_text(text.as_str());
    let answer = req.send().promise.await.map_err(|e| failed(&e))?;
    let report = answer
        .get()
        .and_then(persons::import_text_results::Reader::get_report)
        .map_err(|e| failed(&e))?;
    let created = report.get_created().map_err(|e| failed(&e))?.len();
    let skipped = report.get_skipped().map_err(|e| failed(&e))?.len();
    let failures: Vec<String> = report
        .get_failed()
        .map_err(|e| failed(&e))?
        .iter()
        .map(|t| {
            t.map_err(|e| failed(&e))?
                .to_string()
                .map_err(|e| e.to_string())
        })
        .collect::<Result<_, _>>()?;
    let summary = format!(
        "created {created}, skipped {skipped}, failed {}",
        failures.len()
    );
    if failures.is_empty() {
        Ok(summary)
    } else {
        // Something was refused: the reasons go to stderr with a non-zero exit, so a script sees it.
        Err(format!("{summary}\n{}", failures.join("\n")))
    }
}

/// The most an import file may hold. Larger files are refused before they are read.
const MAX_IMPORT_BYTES: u64 = 16 * 1024 * 1024;

fn read_limited(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read as _;
    let named = |e: std::io::Error| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(named)?;
    let mut bytes = vec![];
    file.take(MAX_IMPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(named)?;
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        return Err(format!("{}: larger than 16 MiB", path.display()));
    }
    String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8 text", path.display()))
}
