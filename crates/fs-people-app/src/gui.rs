//! The window of `fs-people` (ADR 0063): wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, then export or show. The
//! runtime lives as long as the window: the workbench's calls run on it.

use std::path::Path;
use std::sync::Arc;

use fs_components_app::{Status, WindowOptions, launch};
use fs_interfaces::rpc::{default_data_dir, default_socket, prepare_data_dir};
use fs_objects::{Store, TursoStore};
use fs_people_core::PeopleRepo;

use crate::args::{Args, HELP, parse};
use crate::{PeopleAdapter, RemotePeople, app_widget, export_all, import_file};

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
    // A picture alone needs no server; with `--socket` it draws what that server holds.
    let picture_only = args.snapshot.is_some() && args.socket.is_none();
    let local = args.local
        || picture_only
        || args.store.is_some()
        || args.import.is_some()
        || args.export.is_some();
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
        let remote = connect_or_start(&socket, args.socket.is_none())?;
        let remote = Arc::new(remote);
        PeopleAdapter::new(remote.clone(), remote, handle)
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

/// The people kept in this process: in the file of `--store`; else, for a picture only, in memory;
/// else (also for an export) in `app.db` in the data folder of the user (`$XDG_DATA_HOME/freesynergy/people`), so the
/// people are there at the next start.
fn open_local(runtime: &tokio::runtime::Runtime, args: &Args) -> Result<Arc<PeopleRepo>, String> {
    let file = match (
        &args.store,
        args.snapshot.is_some() || args.export.is_some(),
    ) {
        (Some(file), _) => Some(file.clone()),
        (None, true) => None,
        (None, false) => {
            let dir = default_data_dir("people")
                .ok_or("no data folder: give --store or set $XDG_DATA_HOME or $HOME")?;
            let dir = prepare_data_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            Some(dir.join("app.db"))
        }
    };
    let store: Arc<dyn Store> = runtime
        .block_on(async {
            match &file {
                Some(file) => TursoStore::open(file).await,
                None => TursoStore::open_in_memory().await,
            }
        })
        .map(|s| Arc::new(s) as Arc<dyn Store>)
        .map_err(|e| format!("cannot open the store: {e}"))?;
    Ok(Arc::new(PeopleRepo::new(store, ACTOR)))
}

/// Connect to the server; if none answers on the user's own socket, start `fs-people serve` (detached,
/// so other windows can use it too and it keeps the people after this window closes) and wait for it.
/// A socket given with `--socket` is never answered by starting anything: the person chose it.
fn connect_or_start(socket: &Path, may_start: bool) -> Result<RemotePeople, String> {
    let first = RemotePeople::connect(socket);
    if first.is_ok() || !may_start {
        return first;
    }
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut server = std::process::Command::new(me);
    server
        .arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut server, 0);
    server
        .spawn()
        .map_err(|e| format!("could not start the people server: {e}"))?;
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Ok(remote) = RemotePeople::connect(socket) {
            return Ok(remote);
        }
    }
    first
}
