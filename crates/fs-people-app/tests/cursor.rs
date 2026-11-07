//! "Show more" and search read on from a cursor: the pages already read are not read again.
//!
//! Pattern: Integration tests against the real server; the client only counts its page reads.

mod common;

use common::World;
use fs_components_workbenches::people::PeopleSource;
use fs_people_core::Card;

fn fill(w: &World, prefix: &str, count: usize) {
    for i in 0..count {
        w.add(Card::individual(format!("{prefix} {i:03}")));
    }
}

#[test]
fn show_more_continues_from_the_cursor() {
    let w = World::in_memory();
    fill(&w, "P", 250);
    let a = w.adapter.rows("", 0, 120).unwrap();
    assert_eq!((a.len(), w.pages.reads()), (120, (1, 1)));
    let b = w.adapter.rows("", 120, 120).unwrap();
    assert_eq!((b.len(), w.pages.reads()), (120, (1, 2)));
    assert_eq!(b[0].cells[0], "P 120");
    let c = w.adapter.rows("", 240, 120).unwrap();
    assert_eq!(
        (c.len(), w.pages.reads()),
        (10, (1, 2)),
        "the end was known"
    );
    assert!(w.adapter.rows("", 250, 120).unwrap().is_empty());
    assert_eq!(w.pages.reads(), (1, 2));
}

#[test]
fn a_search_continues_from_the_cursor_and_a_new_term_starts_over() {
    let w = World::in_memory();
    fill(&w, "Alpha", 150);
    fill(&w, "Beta", 3);
    let a = w.adapter.rows("alpha", 0, 100).unwrap();
    assert_eq!((a.len(), w.pages.reads()), (100, (1, 0)));
    let b = w.adapter.rows("alpha", 100, 100).unwrap();
    assert_eq!((b.len(), w.pages.reads()), (50, (1, 1)));
    let beta = w.adapter.rows("beta", 0, 100).unwrap();
    assert_eq!((beta.len(), w.pages.reads()), (3, (2, 1)));
}

#[test]
fn a_reload_from_the_start_or_a_jump_reads_afresh_and_stays_right() {
    let w = World::in_memory();
    fill(&w, "P", 250);
    w.adapter.rows("", 0, 100).unwrap();
    let again = w.adapter.rows("", 0, 100).unwrap();
    assert_eq!((again.len(), w.pages.reads().0), (100, 2));
    let jump = w.adapter.rows("", 200, 100).unwrap();
    assert_eq!(jump.len(), 50);
    assert_eq!(jump[0].cells[0], "P 200");
}

#[test]
fn a_person_added_after_the_first_page_does_not_break_show_more() {
    let w = World::in_memory();
    fill(&w, "P", 150);
    w.adapter.rows("", 0, 100).unwrap();
    w.add(Card::individual("Q last"));
    let more = w.adapter.rows("", 100, 100).unwrap();
    assert_eq!(more.len(), 51);
}

/// The window asks for one row more than it shows and asks on from the rows it shows, as
/// `PeopleState` of the workbench does: it must still be one walk, no restart.
#[test]
fn the_windows_show_more_is_one_walk_without_restarts() {
    let w = World::in_memory();
    fill(&w, "P", 650);
    let mut s = w.state();
    assert_eq!(s.rows().len(), 200);
    assert_eq!(w.pages.reads(), (1, 2));
    for shown in [400, 600, 650] {
        s.load_more();
        assert_eq!(s.rows().len(), shown);
    }
    assert!(!s.has_more());
    assert_eq!(w.pages.reads(), (1, 6), "7 pages for 650 people");
    assert_eq!(s.rows()[649].cells[0], "P 649");
    assert_eq!(s.rows()[200].cells[0], "P 200", "no row twice, none lost");
}
