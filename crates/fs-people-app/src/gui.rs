//! The window of `fs-people` (ADR 0063): wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, then export or show. The
//! runtime lives as long as the window: the workbench's calls run on it.

use std::sync::Arc;

use fs_components_app::{Status, WindowOptions, launch};
use fs_interfaces::rpc::default_socket;
use fs_objects::{Store, TursoStore};
use fs_people_core::PeopleRepo;

use crate::args::{Args, HELP, parse};
use crate::{NoRoles, PeopleAdapter, RemotePeople, app_widget, export_all, import_file};

/// Who acts on this store. Provisional: there is no sign-in yet.
const ACTOR: &str = "https://localhost/actors/owner";

/// Open the window (or do the one thing the words ask for). `words` are the words without the
/// program name.
///
/// # Errors
/// The message for the person: bad words, a store that does not open, a file that cannot be read.
pub fn run(words: &[String]) -> Result<(), String> {
    let args = parse(words)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let handle = runtime.handle().clone();
    let local =
        args.local || args.store.is_some() || args.import.is_some() || args.export.is_some();
    let mut status = None;
    let adapter = if local {
        let repo = open_local(&runtime, &args)?;
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
        PeopleAdapter::new(repo.clone(), repo, handle)
    } else {
        let socket = args
            .socket
            .clone()
            .or_else(|| default_socket("people"))
            .ok_or("no socket: give --socket or set XDG_RUNTIME_DIR")?;
        let remote = RemotePeople::connect(&socket)?;
        PeopleAdapter::new(Arc::new(remote), Arc::new(NoRoles), handle)
    };
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

/// The people kept in this process: in a file with `--store`, else in memory.
fn open_local(runtime: &tokio::runtime::Runtime, args: &Args) -> Result<Arc<PeopleRepo>, String> {
    let store: Arc<dyn Store> = runtime
        .block_on(async {
            match &args.store {
                Some(file) => TursoStore::open(file).await,
                None => TursoStore::open_in_memory().await,
            }
        })
        .map(|s| Arc::new(s) as Arc<dyn Store>)
        .map_err(|e| format!("cannot open the store: {e}"))?;
    Ok(Arc::new(PeopleRepo::new(store, ACTOR)))
}
