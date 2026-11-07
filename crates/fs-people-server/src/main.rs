//! `fs-people-server`. Usage: `fs-people-server [--socket PATH] [--data DIR] [--actor IRI]`.
//!
//! Pattern: none (thin binary) — reads its locations, opens the store, serves until a signal.

use std::sync::Arc;

use fs_interfaces::rpc::{ServerOptions, prepare_data_dir, run_until_signal};
use fs_people_core::PeopleRepo;
use fs_people_server::PeopleServer;
use fs_objects::{Store, TursoStore};

fn main() {
    let options = ServerOptions::from_env_or_exit("people");
    let ServerOptions {
        socket,
        data,
        actor,
        instance,
    } = options;
    let served = run_until_signal(&socket, || async move {
        prepare_data_dir(&data)?;
        let store = TursoStore::open(data.join("objects.db"))
            .await
            .map_err(std::io::Error::other)?;
        let store: Arc<dyn Store> = Arc::new(store);
        let repo = PeopleRepo::new(store, actor);
        let repo = Arc::new(match instance {
            Some(domain) => repo.with_instance(domain),
            None => repo,
        });
        let restored = repo
            .restore_role_config()
            .await
            .map_err(std::io::Error::other)?;
        for line in restored.skipped() {
            eprintln!("fs-people-server: skipped stored role file: {line}");
        }
        let server = PeopleServer::new(repo.clone(), repo.clone(), repo);
        Ok(server.into_client().client)
    });
    if let Err(e) = served {
        eprintln!("fs-people-server: {e}");
        std::process::exit(1);
    }
}
