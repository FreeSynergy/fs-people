//! The binary: wiring only.
//!
//! Pattern: Facade consumer — parse arguments, connect to the server, import, then export or show.
//! The runtime lives as long as the window: the workbench's calls wait on it.

use std::sync::Arc;

use fs_components_app::{Status, WindowOptions, launch};
use fs_people_app::args::{HELP, parse};
use fs_people_app::{PeopleAdapter, app_widget, export_all, import_file};
use fs_people_client::RpcPeople;

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
    let socket = args.socket_path()?;
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let people = Arc::new(RpcPeople::connect(socket).map_err(|e| {
        format!(
            "cannot reach the people server at {}: {e}",
            socket.display()
        )
    })?);
    let handle = runtime.handle().clone();

    let mut status = None;
    if let Some(file) = &args.import {
        let summary = import_file(people.as_ref(), &handle, file)?;
        eprintln!("{}", summary.message());
        status = Some(if summary.is_clean() {
            Status::Info(summary.message())
        } else {
            Status::Problem(summary.message())
        });
    }
    if let Some(file) = &args.export {
        let report = export_all(people.as_ref(), &handle, file, &args.export_sensitive)?;
        eprintln!("{}", report.message(file));
        if !report.failed.is_empty() {
            return Err(format!("{} could not be exported", report.failed.len()));
        }
        if args.snapshot.is_none() {
            return Ok(());
        }
    }

    let adapter = PeopleAdapter::new(people.clone(), people, handle);
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
