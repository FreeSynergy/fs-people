//! The window of `fs-people` (ADR 0063): wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, then export or show. The
//! runtime lives as long as the window: the workbench's calls run on it.

use std::path::Path;
use std::sync::Arc;

use fs_components_app::{Status, WindowOptions, launch};
use fs_interfaces::rpc::{
    CommandStarter, companion_path, default_data_dir, default_socket, prepare_data_dir,
};
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
        let remote = connect_or_start(&socket, args.socket.is_some())?;
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

/// Reach the server: the version handshake (a server of another version is replaced, none running is started). This
/// never fails: without a server that fits, the window still opens and every call says what is wrong ("never a black
/// window"). A socket given with `--socket` is never answered by starting a server of another place: the server is
/// started on that very socket.
fn connect_or_start(socket: &Path, own_socket: bool) -> Result<RemotePeople, String> {
    // This window is a user of the server for as long as it lives: the shared lock on the lease is let go by the
    // system when the process ends, however it ends. A server this window starts ends itself when no window holds the
    // lease any more (and a server somebody started by hand has none).
    let lease = companion_path(socket, "lease");
    hold_lease(&lease);
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut words: Vec<std::ffi::OsString> = vec!["serve".into()];
    if own_socket {
        words.extend(["--socket".into(), socket.as_os_str().to_owned()]);
    }
    words.extend(["--lease".into(), lease.into_os_string()]);
    let starter = CommandStarter::new(me, words);
    Ok(RemotePeople::matching(socket, Arc::new(starter)))
}

/// Hold the shared lock on `path` until this process ends.
fn hold_lease(path: &Path) {
    static HELD: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    // Read and write, not append: Windows takes a lock only on a handle that can read or write data.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path);
    if let Ok(file) = file
        && file.lock_shared().is_ok()
    {
        let _ = HELD.set(file);
    }
}
