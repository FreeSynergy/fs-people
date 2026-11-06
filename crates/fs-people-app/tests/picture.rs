//! The whole program as a picture, through the shell's real path. No window is opened.
//!
//! Pattern: Pixel tests — skipped with a message where there is no graphics adapter. Pictures are
//! kept where `FS_COMP_SHOTS` says. A hang here is a deadlock.

mod common;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use common::{World, person};
use fs_components_app::{AppError, Status};
use fs_components_render::{Canvas, RenderError, draw_to_png};
use fs_components_workbenches::people::PeopleWidget;
use fs_people_app::app_widget;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner)
}

fn out(name: &str) -> PathBuf {
    let dir = std::env::var_os("FS_COMP_SHOTS").map_or_else(
        || std::env::temp_dir().join(format!("fs-people-app-{}", std::process::id())),
        PathBuf::from,
    );
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// The picture's bytes; it stays only where `FS_COMP_SHOTS` asks for it.
fn read_and_clear(path: &PathBuf) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    if std::env::var_os("FS_COMP_SHOTS").is_none() {
        std::fs::remove_file(path).unwrap();
        let _ = std::fs::remove_dir(path.parent().unwrap());
    }
    bytes
}

fn world() -> World {
    let w = World::in_memory();
    let mut ada = person("Ada Lovelace", "ada@x.org");
    ada.add_phone("+44 20 1234");
    ada.add_organization("Analytical Engines Ltd");
    ada.add_email("home@x.org");
    w.add(ada);
    w.add(person("Grace Hopper", "grace@x.org"));
    let bot = w.add(person("Helper", "bot@x.org"));
    w.runtime
        .block_on(w.repo.mark_ai(bot.parse().unwrap()))
        .unwrap();
    w
}

#[test]
fn the_program_draws_through_picture_and_through_the_live_build_alike() {
    let _one = serial();
    let w = world();
    let app = app_widget(w.adapter.clone()).unwrap();
    app.set_status(Some(Status::Info("Imported 3, 0 already known".into())));
    let once = out("people-app.png");
    match app.picture(1000, 640, &once) {
        Err(AppError::Picture(RenderError::NoGraphics(why))) => {
            eprintln!("skipped: {why}");
            return;
        }
        other => other.unwrap(),
    }
    let live = out("people-app-live.png");
    let canvas = Canvas::new(1000, 640, app.styles());
    draw_to_png(
        &canvas,
        |width, height| app.build(width, height),
        || {},
        &live,
    )
    .unwrap();
    let (a, b) = (read_and_clear(&once), read_and_clear(&live));
    assert!(!a.is_empty() && !b.is_empty());
    assert_eq!(a, b, "the window's path draws what the picture draws");
}

#[test]
fn a_problem_reaches_the_status_line_of_the_picture() {
    let _one = serial();
    let w = world();
    let app = app_widget(w.adapter.clone()).unwrap();
    let plain = out("people-plain.png");
    if matches!(
        app.picture(800, 500, &plain),
        Err(AppError::Picture(RenderError::NoGraphics(_)))
    ) {
        return;
    }
    app.set_status(Some(Status::Problem(
        "The import stopped: this card is marked sensitive and this store is not encrypted".into(),
    )));
    let problem = out("people-problem.png");
    app.picture(800, 500, &problem).unwrap();
    assert_ne!(read_and_clear(&plain), read_and_clear(&problem));
}

#[test]
fn the_marks_of_an_ai_and_of_sensitive_data_are_drawn() {
    let _one = serial();
    let w = world();
    let mut secret = person("Secret Person", "s@x.org");
    secret.mark_sensitive(["pd:HealthRecord"]);
    let secret = w.add(secret);
    let widget = PeopleWidget::new(w.adapter.clone(), fs_people_app::labels());
    let canvas = Canvas::new(1000, 640, widget.styles());
    let draw = |path: &PathBuf| draw_to_png(&canvas, |x, y| widget.build(x, y), || {}, path);
    let plain = out("people-plain-person.png");
    if matches!(draw(&plain), Err(RenderError::NoGraphics(_))) {
        return;
    }
    widget.with_state(|s| s.select(&secret));
    let marked = out("people-sensitive.png");
    draw(&marked).unwrap();
    assert_ne!(read_and_clear(&plain), read_and_clear(&marked));
}
