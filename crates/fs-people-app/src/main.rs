//! The binary: wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, then export or show. The
//! runtime lives as long as the window: the workbench's calls run on it.

use std::sync::Arc;

use fs_components_app::{Status, WindowOptions, launch};
use fs_people_app::args::{HELP, parse};
use fs_people_app::{PeopleAdapter, app_widget, export_all, import_file};
use fs_people_core::PeopleRepo;
use fs_store::{Store, TursoStore};

/// Who acts on this store. Provisional: there is no sign-in yet.
const ACTOR: &str = "https://localhost/actors/owner";

fn main() {
    if let Err(message) = run() {
        eprintln!("fs-people-app: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse(&argv)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let store: Arc<dyn Store> = runtime
        .block_on(async {
            match &args.store {
                Some(file) => TursoStore::open(file).await,
                None => TursoStore::open_in_memory().await,
            }
        })
        .map(|s| Arc::new(s) as Arc<dyn Store>)
        .map_err(|e| format!("cannot open the store: {e}"))?;
    let repo = Arc::new(PeopleRepo::new(store, ACTOR));
    let handle = runtime.handle().clone();

    let mut status = None;
    if let Some(file) = &args.import {
        let summary = import_file(repo.as_ref(), &handle, file)?;
        eprintln!("{}", summary.message());
        status = Some(if summary.is_clean() {
            Status::Info(summary.message())
        } else {
            Status::Problem(summary.message())
        });
    }
    if let Some(file) = &args.export {
        let report = export_all(repo.as_ref(), &handle, file, &args.export_sensitive)?;
        eprintln!("{}", report.message(file));
        if !report.failed.is_empty() {
            return Err(format!("{} could not be exported", report.failed.len()));
        }
        if args.snapshot.is_none() {
            return Ok(());
        }
    }

    let adapter = PeopleAdapter::new(repo.clone(), repo, handle);
    let app = app_widget(Arc::new(adapter)).map_err(|e| e.to_string())?;
    app.set_status(status);
    let (width, height) = args.size;
    if let Some(file) = &args.snapshot {
        app.picture(width, height, file)
            .map_err(|e| e.to_string())?;
        eprintln!("wrote {}", file.display());
        return Ok(());
    }
    let mut options = WindowOptions::default();
    options.width = width;
    options.height = height;
    launch(options, app).map_err(|e| e.to_string())
}
