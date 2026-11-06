//! The report of an import on the wire.
//!
//! Pattern: Adapter, with a budget — what a file produced can be far larger than the file, and an answer
//! that does not fit one message ends the connection and loses the report with it.

use fs_interfaces::people_capnp::import_report;
use fs_interfaces::rpc::MAX_PAGE_BYTES;
use fs_people_core::ImportReport;

use super::person::{UUID_BYTES, count, write_ids};

/// What one text costs in a list: the pointer, the text with its end, padded.
fn text_weight(text: &str) -> usize {
    8 + (text.len() + 8) / 8 * 8
}

/// Write as many `items` as fit `budget`, and a last entry saying how many more there were: a small
/// file can fail in millions of places, and an answer that does not fit ends the connection.
fn write_capped<'a>(
    out: impl FnOnce(u32) -> capnp::text_list::Builder<'a>,
    items: &[&str],
    budget: usize,
) -> capnp::Result<()> {
    let room = budget.saturating_sub(text_weight("999999999 more"));
    let mut used = 0;
    let fit = items
        .iter()
        .take_while(|t| {
            used += text_weight(t);
            used <= room
        })
        .count();
    let more = items.len() - fit;
    let mut list = out(count(fit + usize::from(more > 0))?);
    for (i, text) in (0u32..).zip(&items[..fit]) {
        list.set(i, *text);
    }
    if more > 0 {
        list.set(count(fit)?, format!("{more} more").as_str());
    }
    Ok(())
}

/// Write the report of an import: `created` whole, the rest as far as it fits. The contract has no
/// place for `unclear` yet, and the cards there were stored, but whoever imports must hear of them:
/// provisionally they come first in `failed`.
pub(crate) fn write_report(
    mut out: import_report::Builder<'_>,
    report: &ImportReport,
) -> capnp::Result<()> {
    let created = count(report.created.len())?;
    write_ids(out.reborrow().init_created(created), &report.created);
    let rest = MAX_PAGE_BYTES.saturating_sub(report.created.len().saturating_mul(UUID_BYTES)) / 2;
    let skipped: Vec<&str> = report.skipped.iter().map(String::as_str).collect();
    write_capped(|n| out.reborrow().init_skipped(n), &skipped, rest)?;
    let unclear: Vec<String> = report
        .unclear
        .iter()
        .map(|who| format!("stored, its sensitive marking names no category: {who}"))
        .collect();
    let failed: Vec<&str> = unclear
        .iter()
        .chain(&report.failed)
        .map(String::as_str)
        .collect();
    write_capped(|n| out.init_failed(n), &failed, rest)
}

#[cfg(test)]
mod tests {
    use capnp::message::Builder;
    use fs_interfaces::rpc::MAX_MESSAGE_BYTES;
    use fs_store::ObjectId;

    use super::*;

    #[test]
    fn a_report_of_millions_fits_and_says_how_many_it_left_out() {
        let created = vec![ObjectId::new(); 1000];
        let report = ImportReport {
            created: created.clone(),
            skipped: vec!["known-uid".to_string(); 3_000_000],
            failed: vec!["a reason of some length, as they come".to_string(); 3_000_000],
            unclear: vec!["uid: Name".to_string()],
        };
        let mut message = Builder::new_default();
        write_report(message.init_root(), &report).unwrap();
        let words: usize = message
            .get_segments_for_output()
            .iter()
            .map(|s| s.len())
            .sum();
        assert!(words < MAX_MESSAGE_BYTES * 3 / 4, "{words} bytes");
        let reader = message
            .get_root_as_reader::<import_report::Reader>()
            .unwrap();
        assert_eq!(
            reader.get_created().unwrap().len(),
            1000,
            "created is whole"
        );
        let failed = reader.get_failed().unwrap();
        assert_eq!(
            failed.get(0).unwrap().to_str().unwrap(),
            "stored, its sensitive marking names no category: uid: Name"
        );
        let last = failed
            .get(failed.len() - 1)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let shown = u64::from(failed.len()) - 2; // without the unclear entry and this one
        assert_eq!(last, format!("{} more", 3_000_000 - shown));
        let skipped = reader.get_skipped().unwrap();
        let last = skipped
            .get(skipped.len() - 1)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(
            last,
            format!("{} more", 3_000_000 - (u64::from(skipped.len()) - 1))
        );
    }

    #[test]
    fn a_small_report_is_written_whole() {
        let report = ImportReport {
            created: vec![],
            skipped: vec!["s".into()],
            failed: vec!["f".into()],
            unclear: vec!["u: U".into()],
        };
        let mut message = Builder::new_default();
        write_report(message.init_root(), &report).unwrap();
        let reader = message
            .get_root_as_reader::<import_report::Reader>()
            .unwrap();
        let texts = |l: capnp::text_list::Reader<'_>| -> Vec<String> {
            l.iter().map(|t| t.unwrap().to_string().unwrap()).collect()
        };
        assert_eq!(texts(reader.get_skipped().unwrap()), ["s"]);
        assert_eq!(
            texts(reader.get_failed().unwrap()),
            ["stored, its sensitive marking names no category: u: U", "f"]
        );
    }

    #[test]
    fn the_last_entry_counts_what_did_not_fit() {
        let items = ["0123456789"; 100];
        let mut message = Builder::new_default();
        let mut root: import_report::Builder<'_> = message.init_root();
        let budget = text_weight("999999999 more") + 10 * text_weight(items[0]);
        write_capped(|n| root.reborrow().init_failed(n), &items, budget).unwrap();
        let failed = root.into_reader().get_failed().unwrap();
        assert_eq!(failed.len(), 11);
        assert_eq!(failed.get(10).unwrap().to_str().unwrap(), "90 more");
    }
}
