//! Replace the shown values of a map of sub-objects, keeping what a form does not show.
//!
//! Pattern: Command helper of `edit.rs` — one function for every multiple field, so the rule
//! (an entry stays with its details when its value stays) is written once.

use fs_types::BasicRules;

use super::fields::Field;
use crate::card::Map;

/// The entry id after `prefix` that is not taken: `e1`, `e2`, ...
fn fresh<T>(map: &Map<T>, prefix: &str) -> String {
    // A map of n entries leaves one of the n + 1 ids free.
    (1..=map.len() + 1)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !map.contains_key(id))
        .unwrap_or_else(|| format!("{prefix}{}", map.len() + 1))
}

/// Replace the shown values of a map of sub-objects. An entry whose shown value is among `values`
/// stays as it is (with its contexts, pref, label); an entry that shows nothing stays; the others go;
/// a value without an entry gets a new one.
pub(super) fn sync<T>(
    map: &mut Map<T>,
    prefix: &str,
    field: &Field,
    values: &[String],
    rules: &BasicRules,
    read: impl Fn(&T) -> Option<String>,
    make: impl Fn(&str) -> T,
) {
    let same = |raw: &str, value: &str| {
        raw == value
            || rules
                .check(field.basic, raw, &[])
                .is_ok_and(|canonical| canonical == value)
    };
    let mut old = std::mem::take(map);
    let hidden: Vec<String> = old
        .iter()
        .filter(|(_, o)| read(o).is_none())
        .map(|(id, _)| id.clone())
        .collect();
    for id in hidden {
        if let Some(o) = old.remove(&id) {
            map.insert(id, o);
        }
    }
    for value in values {
        let kept = old
            .iter()
            .find(|(_, o)| read(o).is_some_and(|raw| same(&raw, value)))
            .map(|(id, _)| id.clone());
        if let Some((id, o)) = kept.and_then(|id| old.remove_entry(&id)) {
            map.insert(id, o);
        } else {
            let id = fresh(map, prefix);
            map.insert(id, make(value));
        }
    }
}
