//! Import the demo contacts, list, search, export one as vCard, show same-person proposals.
//!
//! cargo run -p fs-contacts-core --example demo -- [contacts.json]

use std::sync::Arc;

use fs_contacts_core::{ContactBook, ContactRepo, ListFilter};
use fs_store::{Store, TursoStore};

const DEMO: &str = "/srv/FreeSynergy/fs-documentation/temp/demo-data/contacts.json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| DEMO.into());
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await?);
    // Program against the trait; `ContactRepo` is the store-backed implementation.
    let repo: Arc<dyn ContactBook> =
        Arc::new(ContactRepo::new(store, "https://local.invalid/users/owner"));

    let rep = repo
        .import_jscontact(&std::fs::read_to_string(path)?)
        .await?;
    println!(
        "imported {} cards ({} skipped, {} failed)",
        rep.created.len(),
        rep.skipped.len(),
        rep.failed.len()
    );

    println!("\nall contacts:");
    for o in repo.list(&ListFilter::default()).await? {
        println!(
            "  {}  {:<45} {:?}",
            o.envelope.id,
            o.envelope.name,
            o.index.get("email").cloned().unwrap_or_default()
        );
    }

    for term in ["matrix", "info@", "+31"] {
        let hits = repo.search(term).await?;
        println!(
            "\nsearch {term:?}: {:?}",
            hits.iter()
                .map(|o| o.envelope.name.as_str())
                .collect::<Vec<_>>()
        );
    }

    if let Some(o) = repo.search("nlnet").await?.first() {
        println!(
            "\nvCard of {}:\n{}",
            o.envelope.name,
            repo.export_vcard(o.envelope.id).await?
        );
    }

    let proposals = repo.find_same_person_proposals().await?;
    println!("same-person proposals: {}", proposals.len());

    // vCard import keeps the original (ADR 0030)
    let vcf =
        "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Ada Lovelace\r\nEMAIL:ada@example.org\r\nEND:VCARD\r\n";
    let rep = repo.import_vcard(vcf).await?;
    let ada = repo.get(rep.created[0]).await?.unwrap();
    println!(
        "imported vCard -> {} (original {} bytes, {})",
        ada.envelope.name,
        ada.original.as_ref().unwrap().bytes.len(),
        ada.original.as_ref().unwrap().media_type
    );
    Ok(())
}
